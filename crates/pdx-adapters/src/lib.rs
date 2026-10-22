//! ローカルFSと組込template、および明示build用Docker adapter。
mod build;
mod docker;
pub use build::default_state_root;
pub use docker::DockerRuntime;
use pdx_application::ProjectStore;
use pdx_core::*;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub struct LocalStore;
static NEXT: AtomicU64 = AtomicU64::new(0);

fn io_error(path: &Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::input("filesystem", format!("{}: {error}", path.display()))
}
fn save_error(path: &Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::new("save_failed", 5, format!("{}: {error}", path.display()))
}
fn is_link(meta: &fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

/// Check every ancestor, including ProjectRoot's own ancestors, before canonicalization.
fn no_links(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        // A Windows Prefix (especially \\?\C:) is not a filesystem root until
        // RootDir is appended. Querying it alone returns ERROR_INVALID_FUNCTION.
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(meta) if is_link(&meta) => {
                return Err(Diagnostic::safety(format!(
                    "symlink/reparse point: {}",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(&current, e)),
        }
    }
    Ok(())
}
fn absolute(path: &Path) -> Result<PathBuf> {
    let path = std::path::absolute(path).map_err(|e| io_error(path, e))?;
    no_links(&path)?;
    Ok(path)
}

fn checked_path(root: &Path, relative: &str) -> Result<PathBuf> {
    relative_path(relative)?;
    no_links(root)?;
    let mut current = root.to_path_buf();
    let mut components = relative.split('/').peekable();
    while let Some(part) = components.next() {
        if current.is_dir() {
            for entry in fs::read_dir(&current).map_err(|e| io_error(&current, e))? {
                let entry = entry.map_err(|e| io_error(&current, e))?;
                if identity(&entry.file_name().to_string_lossy()) == identity(part)
                    && entry.file_name() != part
                {
                    return Err(Diagnostic::safety(format!(
                        "filesystem case alias: {} / {part}",
                        entry.path().display()
                    )));
                }
            }
        }
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if is_link(&meta) => {
                return Err(Diagnostic::safety(format!(
                    "symlink/reparse point: {}",
                    current.display()
                )));
            }
            Ok(meta) if components.peek().is_some() && !meta.is_dir() => {
                return Err(Diagnostic::safety(format!(
                    "non-directory path ancestor: {}",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(&current, e)),
        }
    }
    Ok(current)
}

fn walk(root: &Path, relative: &str, budget: &mut usize, files: &mut Vec<String>) -> Result<()> {
    if relative.split('/').count() > 64 {
        return Err(Diagnostic::input(
            "filesystem_budget",
            "directory depth exceeds 64",
        ));
    }
    if *budget == 0 {
        return Err(Diagnostic::input(
            "filesystem_budget",
            "directory exceeds 10000 entries",
        ));
    }
    *budget -= 1;
    let path = checked_path(root, relative)?;
    let meta = fs::symlink_metadata(&path).map_err(|e| io_error(&path, e))?;
    if meta.is_dir() {
        for entry in fs::read_dir(&path).map_err(|e| io_error(&path, e))? {
            let entry = entry.map_err(|e| io_error(&path, e))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| Diagnostic::safety("non-UTF8 internal path"))?;
            walk(root, &format!("{relative}/{name}"), budget, files)?;
        }
    } else if meta.is_file() {
        files.push(relative.into());
    } else {
        return Err(Diagnostic::safety(format!(
            "unsupported filesystem object: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn embedded_files() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        (
            "template.toml".into(),
            include_bytes!("../../../templates/default-report/template.toml").to_vec(),
        ),
        (
            "metadata.schema.json".into(),
            include_bytes!("../../../templates/default-report/metadata.schema.json").to_vec(),
        ),
        (
            "defaults.yaml".into(),
            include_bytes!("../../../templates/default-report/defaults.yaml").to_vec(),
        ),
        (
            "sample.md".into(),
            include_bytes!("../../../templates/default-report/sample.md").to_vec(),
        ),
        (
            "LICENSE.txt".into(),
            include_bytes!("../../../templates/default-report/LICENSE.txt").to_vec(),
        ),
    ])
}

impl ProjectStore for LocalStore {
    fn locate(&self, explicit: Option<&Path>) -> Result<PathBuf> {
        if let Some(path) = explicit {
            let root = absolute(path)?;
            if !root.is_dir() {
                return Err(Diagnostic::input(
                    "project_not_found",
                    "explicit path must be the project directory",
                ));
            }
            checked_path(&root, "pdx.toml")?;
            return fs::canonicalize(&root).map_err(|e| io_error(&root, e));
        }
        let mut root =
            absolute(&std::env::current_dir().map_err(|e| io_error(Path::new("."), e))?)?;
        loop {
            let config = checked_path(&root, "pdx.toml")?;
            if config.exists() {
                return Ok(root);
            }
            if !root.pop() {
                return Err(Diagnostic::input(
                    "project_not_found",
                    "no pdx.toml found in ancestors",
                ));
            }
        }
    }
    fn read(&self, root: &Path, relative: &str, limit: usize) -> Result<Vec<u8>> {
        let path = checked_path(root, relative)?;
        let meta = fs::symlink_metadata(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Diagnostic::new(
                    if relative == "pdx.lock" {
                        "missing_dependency"
                    } else {
                        "missing_input"
                    },
                    if relative == "pdx.lock" { 3 } else { 2 },
                    relative,
                )
            } else {
                io_error(&path, e)
            }
        })?;
        if !meta.is_file() {
            return Err(Diagnostic::input(
                "invalid_input",
                format!("not a regular file: {relative}"),
            ));
        }
        if meta.len() > limit as u64 {
            return Err(Diagnostic::input(
                "parse_budget",
                format!("{relative}: file exceeds {limit} bytes"),
            ));
        }
        let file = File::open(&path).map_err(|e| io_error(&path, e))?;
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| io_error(&path, e))?;
        if bytes.len() > limit {
            return Err(Diagnostic::input("parse_budget", relative));
        }
        Ok(bytes)
    }
    fn input(&self, root: &Path, relative: &str, directory_allowed: bool) -> Result<()> {
        let path = checked_path(root, relative)?;
        let meta = fs::symlink_metadata(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Diagnostic::input("missing_input", relative)
            } else {
                io_error(&path, e)
            }
        })?;
        if !meta.is_file() && !(directory_allowed && meta.is_dir()) {
            return Err(Diagnostic::input("invalid_input", relative));
        }
        if meta.is_dir() {
            walk(root, relative, &mut 10000, &mut Vec::new())?;
        }
        Ok(())
    }
    fn output(&self, root: &Path, relative: &str) -> Result<()> {
        let path = checked_path(root, relative)?;
        // Existing ancestors must be directories, even when the leaf does not exist.
        let mut current = root.to_path_buf();
        for part in relative.split('/') {
            current.push(part);
            if current.exists() && !current.is_dir() {
                return Err(Diagnostic::safety(format!(
                    "output path is not a directory: {}",
                    current.display()
                )));
            }
        }
        if path.exists() {
            walk(root, relative, &mut 10000, &mut Vec::new())?;
        }
        Ok(())
    }
    fn template(&self, root: &Path, reference: &TemplateRef) -> Result<TemplatePackage> {
        match (&reference.id, &reference.path) {
            (Some(id), None) if id == "default-report" => resolve_package(&embedded_files()),
            (Some(id), None) => Err(Diagnostic::new(
                "missing_dependency",
                3,
                format!("unregistered template: {id}"),
            )),
            (None, Some(relative)) => {
                self.input(root, relative, true)?;
                let package_root = checked_path(root, relative)?;
                if !package_root.is_dir() {
                    return Err(Diagnostic::input(
                        "invalid_template",
                        "template path must be a directory",
                    ));
                }
                let manifest = parse_manifest(
                    &String::from_utf8(self.read(&package_root, "template.toml", 1_048_576)?)
                        .map_err(|_| Diagnostic::input("invalid_utf8", "template.toml"))?,
                )?;
                let names = package_files(&manifest);
                let mut found = Vec::new();
                // Scan all entries to reject undeclared assets and unsafe links before hashing.
                for entry in fs::read_dir(&package_root).map_err(|e| io_error(&package_root, e))? {
                    let entry = entry.map_err(|e| io_error(&package_root, e))?;
                    let name = entry
                        .file_name()
                        .into_string()
                        .map_err(|_| Diagnostic::safety("non-UTF8 template path"))?;
                    walk(&package_root, &name, &mut 10000, &mut found)?;
                }
                found.sort();
                if let Some(missing) = names.iter().find(|name| !found.contains(name)) {
                    return Err(Diagnostic::new(
                        "missing_dependency",
                        3,
                        format!("missing template asset: {missing}"),
                    ));
                }
                if found != names {
                    return Err(Diagnostic::input(
                        "invalid_template",
                        "package files differ from manifest declarations",
                    ));
                }
                if names.len() > 256 {
                    return Err(Diagnostic::input(
                        "parse_budget",
                        "template exceeds 256 files",
                    ));
                }
                let mut files = BTreeMap::new();
                let mut total = 0;
                for name in names {
                    let bytes = self.read(&package_root, &name, 16 * 1_048_576)?;
                    total += bytes.len();
                    if total > 32 * 1_048_576 {
                        return Err(Diagnostic::input("parse_budget", "template exceeds 32 MiB"));
                    }
                    files.insert(name, bytes);
                }
                resolve_package(&files)
            }
            _ => Err(Diagnostic::input(
                "invalid_template_ref",
                "exactly one of id/path is required",
            )),
        }
    }
    fn initialize(
        &self,
        path: &Path,
        files: &BTreeMap<String, Vec<u8>>,
        directories: &[&str],
    ) -> Result<PathBuf> {
        let root = absolute(path)?;
        let created_root = match fs::create_dir(&root) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && root.is_dir() => false,
            Err(e) => return Err(save_error(&root, e)),
        };
        let mut created_files = Vec::new();
        let mut created_dirs = Vec::new();
        let result = (|| {
            no_links(&root)?;
            if fs::read_dir(&root)
                .map_err(|e| io_error(&root, e))?
                .next()
                .is_some()
            {
                return Err(Diagnostic::new(
                    "init_collision",
                    4,
                    "init requires a new or empty directory",
                ));
            }
            let guard = root.join(".pdx-init.guard");
            let _guard = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&guard)
                .map_err(|e| save_error(&guard, e))?;
            created_files.push(guard.clone());
            // A competing init must either own the guard or observe a collision.
            for entry in fs::read_dir(&root).map_err(|e| io_error(&root, e))? {
                if entry.map_err(|e| io_error(&root, e))?.path() != guard {
                    return Err(Diagnostic::new(
                        "init_collision",
                        4,
                        "directory changed during init",
                    ));
                }
            }
            for directory in directories {
                let path = checked_path(&root, directory)?;
                fs::create_dir(&path).map_err(|e| save_error(&path, e))?;
                created_dirs.push(path);
            }
            for (relative, bytes) in files {
                let path = checked_path(&root, relative)?;
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| save_error(&path, e))?;
                created_files.push(path.clone());
                file.write_all(bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(|e| save_error(&path, e))?;
            }
            drop(_guard);
            fs::remove_file(&guard).map_err(|e| save_error(&guard, e))?;
            created_files.retain(|p| p != &guard);
            Ok(root.clone())
        })();
        if let Err(mut error) = result {
            let mut remaining = Vec::new();
            for path in created_files.iter().rev() {
                if fs::remove_file(path).is_err() {
                    remaining.push(path.display().to_string());
                }
            }
            for path in created_dirs.iter().rev() {
                if fs::remove_dir(path).is_err() {
                    remaining.push(path.display().to_string());
                }
            }
            if created_root && fs::remove_dir(&root).is_err() {
                remaining.push(root.display().to_string());
            }
            if !remaining.is_empty() {
                error = Diagnostic::new(
                    "cleanup_failed",
                    5,
                    format!("{error}; retained: {}", remaining.join(", ")),
                );
            }
            return Err(error);
        }
        result
    }
    fn save_lock(&self, root: &Path, bytes: &[u8]) -> Result<()> {
        let destination = checked_path(root, "pdx.lock")?;
        let name = format!(
            ".pdx-lock-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let temporary = checked_path(root, &name)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| save_error(&temporary, e))?;
        let result = (|| {
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(|e| save_error(&temporary, e))?;
            drop(file);
            checked_path(root, "pdx.lock")?;
            fs::rename(&temporary, &destination).map_err(|e| save_error(&destination, e))
        })();
        if result.is_err()
            && let Err(e) = fs::remove_file(&temporary)
        {
            return Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!("{}: {e}", temporary.display()),
            ));
        }
        result
    }
}
