use super::*;
use pdx_application::{BuildSettings, BuildStore, BuildWorkspace, Cancellation};
use tempfile::TempDir;

pub fn default_state_root() -> Result<PathBuf> {
    let value =
        if let Some(path) = std::env::var_os("PDX_STATE_ROOT") {
            PathBuf::from(path)
        } else if cfg!(windows) {
            PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
                Diagnostic::new("state_unavailable", 3, "LOCALAPPDATA is required")
            })?)
            .join("Pandocker-X/state")
        } else if let Some(path) = std::env::var_os("XDG_STATE_HOME") {
            PathBuf::from(path).join("pandocker-x")
        } else {
            PathBuf::from(std::env::var_os("HOME").ok_or_else(|| {
                Diagnostic::new("state_unavailable", 3, "HOME or XDG_STATE_HOME is required")
            })?)
            .join(".local/state/pandocker-x")
        };
    if !value.is_absolute() {
        return Err(Diagnostic::input(
            "state_unavailable",
            "state root must be absolute",
        ));
    }
    Ok(value)
}

pub struct LocalBuild {
    root: PathBuf,
    output: String,
    state: PathBuf,
    id: String,
    input: PathBuf,
    temp: Option<TempDir>,
    _lease: File,
    originals: BTreeMap<String, InputDigest>,
    committed: bool,
}
fn ensure_dir(path: &Path) -> Result<()> {
    no_links(path)?;
    fs::create_dir_all(path).map_err(|e| save_error(path, e))?;
    no_links(path)?;
    if !fs::metadata(path).map_err(|e| io_error(path, e))?.is_dir() {
        return Err(Diagnostic::safety("directory required"));
    }
    Ok(())
}
fn normalized_root(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Diagnostic::safety(
            "absolute root without parent components required",
        ));
    }
    let path = absolute(path)?;
    let mut ancestor = path.as_path();
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        suffix.push(
            ancestor
                .file_name()
                .ok_or_else(|| Diagnostic::safety("invalid state root"))?
                .to_owned(),
        );
        ancestor = ancestor
            .parent()
            .ok_or_else(|| Diagnostic::safety("invalid state root"))?;
    }
    let mut normalized = fs::canonicalize(ancestor).map_err(|e| io_error(ancestor, e))?;
    for component in suffix.into_iter().rev() {
        normalized.push(component);
    }
    Ok(normalized)
}
fn root_overlap(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        let a = a.to_string_lossy().replace('\\', "/").to_uppercase();
        let b = b.to_string_lossy().replace('\\', "/").to_uppercase();
        a == b || a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
    }
    #[cfg(not(windows))]
    {
        a.starts_with(b) || b.starts_with(a)
    }
}
impl BuildStore for LocalStore {
    type Workspace = LocalBuild;
    fn begin(&self, root: &Path, plan: &BuildPlan, settings: &BuildSettings) -> Result<LocalBuild> {
        let state = normalized_root(&settings.state_root)?;
        if root_overlap(&state, &normalized_root(root)?) {
            return Err(Diagnostic::safety(
                "State and Project roots must not overlap",
            ));
        }
        ensure_dir(&state)?;
        // The lease belongs to the output, so changing State cannot bypass exclusion.
        LocalStore.output(root, &plan.output_dir)?;
        let output = checked_path(root, &plan.output_dir)?;
        ensure_dir(&output)?;
        let lock_path = checked_path(&output, ".pdx-build.lock")?;
        let lease = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| save_error(&lock_path, e))?;
        lease.try_lock().map_err(|e| {
            Diagnostic::new(
                "build_busy",
                3,
                format!("cannot acquire Project/Target build lease: {e}"),
            )
        })?;
        let workspaces = checked_path(&state, "workspaces")?;
        ensure_dir(&workspaces)?;
        let temp = tempfile::Builder::new()
            .prefix("pdx-build-")
            .tempdir_in(&workspaces)
            .map_err(|e| save_error(&workspaces, e))?;
        let id = temp
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        validate_id(&id)?;
        let input = temp.path().join("input");
        ensure_dir(&input)?;
        Ok(LocalBuild {
            root: root.into(),
            output: plan.output_dir.clone(),
            state,
            id,
            input,
            temp: Some(temp),
            _lease: lease,
            originals: BTreeMap::new(),
            committed: false,
        })
    }
}
impl LocalBuild {
    fn write_snapshot(
        &self,
        name: &str,
        bytes: &[u8],
        records: &mut BTreeMap<String, InputDigest>,
        total: &mut usize,
    ) -> Result<()> {
        relative_path(name)?;
        *total += bytes.len();
        if bytes.len() > BUILD_FILE_BYTES || *total > BUILD_INPUT_BYTES {
            return Err(Diagnostic::input(
                "snapshot_budget",
                "snapshot exceeds 32MiB/file or 256MiB total",
            ));
        }
        let path = checked_path(&self.input, name)?;
        ensure_dir(path.parent().unwrap())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| save_error(&path, e))?;
        file.write_all(bytes).map_err(|e| save_error(&path, e))?;
        records.insert(name.into(), digest(bytes));
        Ok(())
    }
    fn copy_source(
        &mut self,
        name: &str,
        destination: &str,
        records: &mut BTreeMap<String, InputDigest>,
        total: &mut usize,
    ) -> Result<()> {
        LocalStore.input(&self.root, name, false)?;
        let bytes = LocalStore.read(&self.root, name, BUILD_FILE_BYTES)?;
        LocalStore.input(&self.root, name, false)?;
        self.originals.insert(name.into(), digest(&bytes));
        self.write_snapshot(destination, &bytes, records, total)
    }
    fn verify_originals(&self, cancel: &Cancellation) -> Result<()> {
        for (name, expected) in &self.originals {
            cancel.check()?;
            let bytes = LocalStore.read(&self.root, name, BUILD_FILE_BYTES)?;
            let actual = digest(&bytes);
            if actual.sha256 != expected.sha256 || actual.bytes != expected.bytes {
                return Err(Diagnostic::input(
                    "input_changed",
                    format!("input changed during build: {name}"),
                ));
            }
        }
        Ok(())
    }
}
impl BuildWorkspace for LocalBuild {
    fn id(&self) -> &str {
        &self.id
    }
    fn input_dir(&self) -> &Path {
        &self.input
    }
    fn snapshot(
        &mut self,
        plan: &BuildPlan,
        project: &[u8],
        lock: &[u8],
        cancel: &Cancellation,
    ) -> Result<BTreeMap<String, InputDigest>> {
        let mut names = plan
            .ordered_inputs
            .iter()
            .chain(&plan.bibliography)
            .chain(&plan.metadata_inputs)
            .cloned()
            .collect::<Vec<_>>();
        let mut budget = 10000;
        for resource in &plan.resources {
            cancel.check()?;
            LocalStore.input(&self.root, resource, true)?;
            if checked_path(&self.root, resource)?.is_dir() {
                ensure_dir(&checked_path(&self.input, resource)?)?;
            }
            walk(&self.root, resource, &mut budget, &mut names)?;
        }
        if let Some(base) = &plan.template_source.path {
            names.extend(plan.template_files.iter().map(|p| format!("{base}/{p}")));
        }
        names.sort();
        names.dedup();
        if names.len() > 10000 {
            return Err(Diagnostic::input(
                "snapshot_budget",
                "snapshot exceeds 10000 files",
            ));
        }
        let mut records = BTreeMap::new();
        let mut total = 0;
        for name in names {
            cancel.check()?;
            self.copy_source(&name, &name, &mut records, &mut total)?;
        }
        self.copy_source(
            "pdx.toml",
            ".pdx/original-pdx.toml",
            &mut records,
            &mut total,
        )?;
        self.copy_source(
            "pdx.lock",
            ".pdx/original-pdx.lock",
            &mut records,
            &mut total,
        )?;
        if plan.template_source.id.is_some() {
            for (name, bytes) in embedded_files() {
                self.write_snapshot(
                    &format!(".pdx/template/{name}"),
                    &bytes,
                    &mut records,
                    &mut total,
                )?;
            }
        }
        self.write_snapshot("pdx.toml", project, &mut records, &mut total)?;
        self.write_snapshot("pdx.lock", lock, &mut records, &mut total)?;
        self.write_snapshot(
            ".pdx/merged-metadata.json",
            &json_bytes(&plan.merged_metadata)?,
            &mut records,
            &mut total,
        )?;
        self.verify_originals(cancel)?;
        Ok(records)
    }
    fn publish(
        &mut self,
        record: &BuildRecord,
        pdf: &[u8],
        cancel: &Cancellation,
    ) -> Result<BuildResult> {
        cancel.check()?;
        validate_pdf_bytes(pdf)?;
        self.verify_originals(cancel)?;
        LocalStore.output(&self.root, &self.output)?;
        let output = checked_path(&self.root, &self.output)?;
        ensure_dir(&output)?;
        let generation = checked_path(&output, &self.id)?;
        fs::create_dir(&generation).map_err(|e| save_error(&generation, e))?;
        let mut owned = Vec::new();
        let commit = (|| {
            no_links(&generation)?;
            let pdf_path = generation.join("document.pdf");
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&pdf_path)
                .map_err(|e| save_error(&pdf_path, e))?;
            owned.push(pdf_path.clone());
            file.write_all(pdf)
                .and_then(|_| file.sync_all())
                .map_err(|e| save_error(&pdf_path, e))?;
            let pending = generation.join("build.json.pending");
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&pending)
                .map_err(|e| save_error(&pending, e))?;
            owned.push(pending.clone());
            file.write_all(&json_bytes(record)?)
                .and_then(|_| file.sync_all())
                .map_err(|e| save_error(&pending, e))?;
            cancel.check()?;
            let marker = generation.join("build.json");
            no_links(&generation)?;
            if marker.exists() {
                return Err(Diagnostic::new(
                    "save_failed",
                    5,
                    "generation commit marker already exists",
                ));
            }
            fs::rename(&pending, &marker).map_err(|e| save_error(&marker, e))?;
            self.committed = true;
            Ok(BuildResult {
                schema_version: 1,
                build_id: self.id.clone(),
                target: record.target.clone(),
                artifact_dir: generation.to_string_lossy().into_owned(),
                pdf: pdf_path.to_string_lossy().into_owned(),
                record: marker.to_string_lossy().into_owned(),
                runtime_verified: record.evidence.runtime_verified,
                pdf_verified: record.pdf_verified,
                static_only: false,
                unverified: if record.evidence.runtime_verified && record.pdf_verified {
                    vec!["Official typesetting conformance and cross-host reproducibility".into()]
                } else {
                    vec!["Test double: real Docker/PDF not verified".into()]
                },
            })
        })();
        if commit.is_err() && !self.committed {
            for path in owned.into_iter().rev() {
                if path.exists() {
                    fs::remove_file(&path).map_err(|e| {
                        Diagnostic::new(
                            "cleanup_failed",
                            5,
                            format!("{}: {e}", generation.display()),
                        )
                    })?;
                }
            }
            fs::remove_dir(&generation).map_err(|e| {
                Diagnostic::new(
                    "cleanup_failed",
                    5,
                    format!("{}: {e}", generation.display()),
                )
            })?;
        }
        commit
    }
    fn finish(&mut self, error: Option<&Diagnostic>) -> Result<()> {
        let mut failed = None;
        if let Some(error) = error {
            let records = checked_path(&self.state, "failures")?;
            if let Err(e) = ensure_dir(&records).and_then(|_| {
                let path = checked_path(&records, &format!("{}.json", self.id))?;
                let bytes = json_bytes(&serde_json::json!({"schema_version":1,"build_id":self.id,"status":"failed","code":error.code,"exit_code":error.exit_code}))?;
                OpenOptions::new().create_new(true).write(true).open(&path).and_then(|mut f| f.write_all(&bytes)).map_err(|e| save_error(&path, e))
            }) { failed = Some(e); }
        }
        if let Some(temp) = self.temp.take() {
            let path = temp.path().to_path_buf();
            if let Err(e) = temp.close() {
                failed = Some(Diagnostic::new(
                    "cleanup_failed",
                    5,
                    format!("workspace retained {}: {e}", path.display()),
                ));
            }
        }
        failed.map_or(Ok(()), Err)
    }
}
