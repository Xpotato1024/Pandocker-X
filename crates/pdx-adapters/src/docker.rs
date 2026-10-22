use pdx_application::{BuildRuntime, Cancellation, RenderRequest, RenderedPdf};
use pdx_core::*;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const LOG_LIMIT: usize = 32 * 1024 * 1024;
pub struct DockerRuntime {
    executable: PathBuf,
    captured: Arc<AtomicUsize>,
}
impl Default for DockerRuntime {
    fn default() -> Self {
        Self {
            executable: "docker".into(),
            captured: Arc::new(AtomicUsize::new(0)),
        }
    }
}
struct Output {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
fn read_bounded(
    mut stream: impl Read,
    total: Arc<AtomicUsize>,
    exceeded: Arc<AtomicBool>,
) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let size = stream.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        let before = total.fetch_add(size, Ordering::Relaxed);
        let keep = size.min(LOG_LIMIT.saturating_sub(before));
        output.extend_from_slice(&buffer[..keep]);
        if keep != size {
            exceeded.store(true, Ordering::SeqCst);
        }
    }
    Ok(output)
}
fn remaining(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|v| !v.is_zero())
        .ok_or_else(|| Diagnostic::new("build_timeout", 1, "Docker operation deadline exceeded"))
}
fn csv_source(path: &Path) -> Result<String> {
    let mut path = path
        .to_str()
        .ok_or_else(|| Diagnostic::safety("Docker mount path must be UTF-8"))?
        .to_owned();
    if let Some(rest) = path.strip_prefix("\\\\?\\") {
        path = rest.to_owned();
    }
    if path.chars().any(char::is_control) {
        return Err(Diagnostic::safety("control character in Docker mount path"));
    }
    Ok(format!(
        "type=bind,\"source={}\",target=/pdx/input,readonly",
        path.replace('"', "\"\"")
    ))
}
impl DockerRuntime {
    fn invoke(
        &self,
        host: Option<&str>,
        args: &[OsString],
        timeout: Duration,
        cancel: &Cancellation,
    ) -> Result<Output> {
        cancel.check()?;
        let mut command = Command::new(&self.executable);
        if let Some(host) = host {
            command
                .args(["--host", host])
                .env_remove("DOCKER_HOST")
                .env_remove("DOCKER_CONTEXT");
        }
        let mut child = command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Diagnostic::new(
                    "missing_runtime",
                    3,
                    format!("cannot start Docker CLI: {e}"),
                )
            })?;
        let total = self.captured.clone();
        let exceeded = Arc::new(AtomicBool::new(false));
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let t = total.clone();
        let e = exceeded.clone();
        let out = thread::spawn(move || read_bounded(stdout, t, e));
        let e = exceeded.clone();
        let err = thread::spawn(move || read_bounded(stderr, total, e));
        let started = Instant::now();
        let mut failure = None;
        let status = loop {
            if let Err(error) = cancel.check() {
                failure = Some(error);
            }
            if started.elapsed() >= timeout {
                failure = Some(Diagnostic::new(
                    "build_timeout",
                    1,
                    "Docker operation timed out",
                ));
            }
            if exceeded.load(Ordering::SeqCst) {
                failure = Some(Diagnostic::new(
                    "log_budget",
                    1,
                    "Docker output exceeded 32MiB",
                ));
            }
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => (),
                Err(e) => {
                    failure = Some(Diagnostic::new("runtime_failed", 3, e.to_string()));
                }
            }
            if failure.is_some() {
                let _ = child.kill();
                break child.wait();
            }
            thread::sleep(Duration::from_millis(20));
        };
        let stdout = out
            .join()
            .map_err(|_| Diagnostic::new("runtime_failed", 3, "stdout reader failed"))?
            .map_err(|e| Diagnostic::new("runtime_failed", 3, e.to_string()))?;
        let stderr = err
            .join()
            .map_err(|_| Diagnostic::new("runtime_failed", 3, "stderr reader failed"))?
            .map_err(|e| Diagnostic::new("runtime_failed", 3, e.to_string()))?;
        if let Some(error) = failure {
            return Err(error);
        }
        let status = status.map_err(|e| Diagnostic::new("runtime_failed", 3, e.to_string()))?;
        if exceeded.load(Ordering::SeqCst) {
            return Err(Diagnostic::new(
                "log_budget",
                1,
                "Docker stdout/stderr exceeded 32MiB",
            ));
        }
        Ok(Output {
            code: status.code().unwrap_or(1),
            stdout,
            stderr,
        })
    }
    fn call(
        &self,
        host: Option<&str>,
        args: &[String],
        deadline: Instant,
        cancel: &Cancellation,
    ) -> Result<Output> {
        self.invoke(
            host,
            &args.iter().map(OsString::from).collect::<Vec<_>>(),
            remaining(deadline)?,
            cancel,
        )
    }
    fn local_host(&self, deadline: Instant, cancel: &Cancellation) -> Result<String> {
        let host = if let Some(host) = std::env::var_os("DOCKER_HOST") {
            host.into_string().map_err(|_| {
                Diagnostic::new("unsupported_runtime", 3, "Docker endpoint must be UTF-8")
            })?
        } else {
            let out = self.call(
                None,
                &[
                    "context".into(),
                    "inspect".into(),
                    "--format".into(),
                    "{{json .Endpoints.docker.Host}}".into(),
                ],
                deadline,
                cancel,
            )?;
            if out.code != 0 {
                return Err(Diagnostic::new(
                    "missing_runtime",
                    3,
                    "Docker local context is unavailable",
                ));
            }
            serde_json::from_slice::<String>(&out.stdout).map_err(|_| {
                Diagnostic::new(
                    "unsupported_runtime",
                    3,
                    "cannot resolve Docker context endpoint",
                )
            })?
        };
        if !(host.starts_with("unix:///") || host.starts_with("npipe:////./pipe/"))
            || host.chars().any(char::is_control)
        {
            return Err(Diagnostic::new(
                "unsupported_runtime",
                3,
                "only a local Unix socket or Windows named pipe is supported; remote Docker is refused",
            ));
        }
        Ok(host)
    }
    fn inspect_image(
        &self,
        host: &str,
        pin: &RuntimePin,
        deadline: Instant,
        cancel: &Cancellation,
    ) -> Result<()> {
        pin.validate()?;
        let out = self.call(
            Some(host),
            &["image".into(), "inspect".into(), pin.image_id.clone()],
            deadline,
            cancel,
        )?;
        if out.code != 0 {
            return Err(Diagnostic::new(
                "missing_runtime",
                3,
                "pinned local image is unavailable; build never pulls images",
            ));
        }
        let images: Value = serde_json::from_slice(&out.stdout)
            .map_err(|_| Diagnostic::new("runtime_failed", 3, "invalid Docker image inspection"))?;
        let image = &images[0];
        if image["Config"]["Volumes"]
            .as_object()
            .is_some_and(|v| !v.is_empty())
        {
            return Err(Diagnostic::new(
                "unsupported_runtime",
                3,
                "runtime images declaring VOLUME are refused",
            ));
        }
        if image["Id"] != pin.image_id || image["Os"] != "linux" || image["Architecture"] != "amd64"
        {
            return Err(Diagnostic::new(
                "unsupported_runtime",
                3,
                "local image identity/platform mismatch",
            ));
        }
        Ok(())
    }
    fn cleanup(&self, host: &str, name: &str, owner: &str, uncertain_creation: bool) -> Result<()> {
        // Cleanup must still work after the rendering output budget is exhausted.
        let cleaner = Self {
            executable: self.executable.clone(),
            captured: Arc::new(AtomicUsize::new(0)),
        };
        let cancel = Cancellation::default();
        let deadline = Instant::now() + Duration::from_secs(10);
        let out = cleaner.call(
            Some(host),
            &["container".into(), "inspect".into(), name.into()],
            deadline,
            &cancel,
        )?;
        if out.code != 0 {
            if String::from_utf8_lossy(&out.stderr).contains("No such") {
                if uncertain_creation {
                    return Err(Diagnostic::new(
                        "cleanup_failed",
                        5,
                        format!(
                            "container creation outcome uncertain; inspect owned name {name} before recovery"
                        ),
                    ));
                }
                return Ok(());
            }
            return Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!("cannot inspect owned container {name}"),
            ));
        }
        let items: Value = serde_json::from_slice(&out.stdout).map_err(|_| {
            Diagnostic::new(
                "cleanup_failed",
                5,
                format!("cannot verify container {name}"),
            )
        })?;
        let object = &items[0];
        if object["Config"]["Labels"]["org.pandocker-x.build-id"] != owner {
            return Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!("container ownership differs: {name}; not removed"),
            ));
        }
        let id = object["Id"]
            .as_str()
            .filter(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| Diagnostic::new("cleanup_failed", 5, "invalid owned container ID"))?;
        let out = cleaner.call(
            Some(host),
            &[
                "container".into(),
                "rm".into(),
                "--force".into(),
                "--".into(),
                id.into(),
            ],
            deadline,
            &cancel,
        )?;
        if out.code != 0 && !String::from_utf8_lossy(&out.stderr).contains("No such") {
            return Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!("container retained: {name} ({id})"),
            ));
        }
        Ok(())
    }
    fn with_container<T>(
        &self,
        pin: &RuntimePin,
        input: Option<&Path>,
        id: &str,
        deadline: Instant,
        cancel: &Cancellation,
        operation: impl FnOnce(&str, &str) -> Result<T>,
    ) -> Result<T> {
        validate_id(id)?;
        let host = self.local_host(deadline, cancel)?;
        self.inspect_image(&host, pin, deadline, cancel)?;
        let name = format!("pdx-{id}");
        let user: String = if let Some(path) = input {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let meta =
                    std::fs::metadata(path).map_err(|e| Diagnostic::safety(e.to_string()))?;
                if meta.uid() == 0 {
                    return Err(Diagnostic::new(
                        "unsupported_runtime",
                        3,
                        "run pdx as a non-root user",
                    ));
                }
                format!("{}:{}", meta.uid(), meta.gid())
            }
            #[cfg(not(unix))]
            {
                let _ = path;
                "65532:65532".into()
            }
        } else {
            "65532:65532".into()
        };
        let mut args = vec![
            "run", "--detach", "--pull", "never", "--name", &name, "--label",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
        args.push(format!("org.pandocker-x.build-id={id}"));
        args.extend(
            [
                "--network",
                "none",
                "--read-only",
                "--cap-drop",
                "ALL",
                "--security-opt",
                "no-new-privileges",
                "--cpus",
                "2",
                "--memory",
                "2g",
                "--pids-limit",
                "256",
                "--tmpfs",
                "/tmp:rw,nosuid,nodev,size=512m",
                "--user",
                &user,
                "--env",
                "HOME=/tmp",
                "--env",
                "TEXMFVAR=/tmp/texmf-var",
                "--env",
                "TEXMFCONFIG=/tmp/texmf-config",
                "--env",
                "TEXMFHOME=/tmp/texmf-home",
                "--env",
                "openin_any=p",
                "--env",
                "openout_any=p",
                "--env",
                "shell_escape=f",
            ]
            .into_iter()
            .map(String::from),
        );
        if let Some(input) = input {
            args.extend(["--mount".into(), csv_source(input)?]);
        }
        args.extend([
            "--entrypoint".into(),
            "/bin/sleep".into(),
            pin.image_id.clone(),
            "600".into(),
        ]);
        let mut started = false;
        let result = self
            .call(Some(&host), &args, deadline, cancel)
            .and_then(|out| {
                if out.code != 0 {
                    Err(Diagnostic::new(
                        "runtime_failed",
                        3,
                        "cannot start isolated pinned runtime",
                    ))
                } else {
                    started = true;
                    operation(&host, &name)
                }
            });
        match (result, self.cleanup(&host, &name, id, !started)) {
            (value, Ok(())) => value,
            (Err(original), Err(cleanup)) => Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!("{original}; {cleanup}; owned container {name}"),
            )),
            (Ok(_), Err(cleanup)) => Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!("{cleanup}; owned container {name}"),
            )),
        }
    }
    fn exec(
        &self,
        (host, name): (&str, &str),
        program: &str,
        args: &[String],
        deadline: Instant,
        cancel: &Cancellation,
        (code, exit): (&'static str, u8),
    ) -> Result<Vec<u8>> {
        let mut command = vec![
            "exec".into(),
            "--workdir".into(),
            "/tmp/work".into(),
            name.into(),
            program.into(),
        ];
        command.extend_from_slice(args);
        let output = self.call(Some(host), &command, deadline, cancel)?;
        if output.code != 0 {
            return Err(Diagnostic::new(
                code,
                exit,
                format!(
                    "{program} failed (exit {}); no generation committed; tool output is not persisted",
                    output.code
                ),
            ));
        }
        Ok(output.stdout)
    }
    fn mkdir(
        &self,
        host: &str,
        name: &str,
        deadline: Instant,
        cancel: &Cancellation,
    ) -> Result<()> {
        let output = self.call(
            Some(host),
            &[
                "exec".into(),
                name.into(),
                "/bin/mkdir".into(),
                "-p".into(),
                "/tmp/work".into(),
            ],
            deadline,
            cancel,
        )?;
        if output.code != 0 {
            return Err(Diagnostic::new(
                "runtime_failed",
                3,
                "cannot prepare runtime tmpfs workspace",
            ));
        }
        Ok(())
    }
}
impl BuildRuntime for DockerRuntime {
    fn probe(
        &self,
        pin: &RuntimePin,
        cancel: &Cancellation,
        timeout: Duration,
    ) -> Result<RuntimeEvidence> {
        self.captured.store(0, Ordering::SeqCst);
        let id = format!(
            "probe-{}-{:x}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| Diagnostic::new("runtime_failed", 3, e.to_string()))?
                .as_nanos()
        );
        let deadline = Instant::now() + timeout;
        self.with_container(pin, None, &id, deadline, cancel, |host, name| {
            self.mkdir(host, name, deadline, cancel)?;
            let mut tools = BTreeMap::new();
            for tool in ["pandoc", "lualatex", "qpdf"] {
                let output = self.exec(
                    (host, name),
                    tool,
                    &["--version".into()],
                    deadline,
                    cancel,
                    ("runtime_tool_missing", 3),
                )?;
                let text = String::from_utf8(output).map_err(|_| {
                    Diagnostic::new("runtime_tool_missing", 3, "tool version is not UTF-8")
                })?;
                tools.insert(
                    tool.into(),
                    text.lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(512)
                        .collect(),
                );
            }
            let font = self.exec(
                (host, name),
                "fc-match",
                &["--format=%{family}".into(), "Noto Serif CJK JP".into()],
                deadline,
                cancel,
                ("runtime_tool_missing", 3),
            )?;
            if !String::from_utf8_lossy(&font).contains("Noto Serif CJK JP") {
                return Err(Diagnostic::new(
                    "runtime_tool_missing",
                    3,
                    "Noto Serif CJK JP font is required",
                ));
            }
            tools.insert("font".into(), "Noto Serif CJK JP".into());
            Ok(RuntimeEvidence {
                driver: "docker".into(),
                runtime_verified: true,
                tools,
            })
        })
    }
    fn render(&self, request: RenderRequest<'_>, cancel: &Cancellation) -> Result<RenderedPdf> {
        let deadline = Instant::now() + request.timeout;
        self.with_container(
            request.pin,
            Some(request.input_dir),
            request.build_id,
            deadline,
            cancel,
            |host, name| {
                self.mkdir(host, name, deadline, cancel)?;
                self.exec(
                    (host, name),
                    "/bin/cp",
                    &["-R".into(), "/pdx/input/.".into(), "/tmp/work".into()],
                    deadline,
                    cancel,
                    ("runtime_failed", 3),
                )?;
                self.exec(
                    (host, name),
                    "pandoc",
                    &pandoc_arguments(request.plan),
                    deadline,
                    cancel,
                    ("pandoc_failed", 1),
                )?;
                let tex_args = [
                    "-no-shell-escape",
                    "-interaction=nonstopmode",
                    "-halt-on-error",
                    "-file-line-error",
                    "-output-directory=.pdx",
                    ".pdx/document.tex",
                ]
                .map(String::from);
                for _ in 0..2 {
                    self.exec(
                        (host, name),
                        "lualatex",
                        &tex_args,
                        deadline,
                        cancel,
                        ("tex_failed", 1),
                    )?;
                }
                self.exec(
                    (host, name),
                    "qpdf",
                    &["--check".into(), ".pdx/document.pdf".into()],
                    deadline,
                    cancel,
                    ("invalid_pdf", 1),
                )?;
                let bytes = self.exec(
                    (host, name),
                    "/bin/cat",
                    &[".pdx/document.pdf".into()],
                    deadline,
                    cancel,
                    ("invalid_pdf", 1),
                )?;
                validate_pdf_bytes(&bytes)?;
                Ok(RenderedPdf {
                    bytes,
                    pdf_verified: true,
                })
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_budget_is_cumulative_and_detects_overflow() {
        let total = Arc::new(AtomicUsize::new(0));
        let exceeded = Arc::new(AtomicBool::new(false));
        let first = read_bounded(
            std::io::repeat(0).take((LOG_LIMIT - 1) as u64),
            total.clone(),
            exceeded.clone(),
        )
        .unwrap();
        assert_eq!(first.len(), LOG_LIMIT - 1);
        assert!(!exceeded.load(Ordering::SeqCst));
        drop(first);
        let second = read_bounded(&b"abc"[..], total, exceeded.clone()).unwrap();
        assert_eq!(second, b"a");
        assert!(exceeded.load(Ordering::SeqCst));
    }
    #[test]
    fn mount_csv_quotes_path_and_rejects_control_characters() {
        assert!(
            csv_source(Path::new("/a,b/原稿"))
                .unwrap()
                .contains("\"source=/a,b/原稿\"")
        );
        assert!(csv_source(Path::new("/a\nb")).is_err());
    }
}
