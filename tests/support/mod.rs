#![allow(dead_code)]
use pdx_adapters::{LocalStore, embedded_files};
use pdx_application::Application;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub fn sandbox() -> tempfile::TempDir {
    let parent = std::env::var_os("PDX_TEST_TEMP")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    assert!(parent.is_absolute());
    tempfile::Builder::new()
        .prefix("pdx-contract-")
        .tempdir_in(parent)
        .unwrap()
}
pub fn app() -> Application<LocalStore> {
    Application::new(LocalStore)
}
pub fn project(parent: &Path) -> PathBuf {
    let root = parent.join("日本語 project");
    app().init(&root, "default-report").unwrap();
    root
}
pub fn edit_config(root: &Path, from: &str, to: &str) {
    let path = root.join("pdx.toml");
    let original = fs::read_to_string(&path).unwrap();
    assert!(original.contains(from), "{from}");
    fs::write(path, original.replace(from, to)).unwrap();
}
pub fn local_template(root: &Path) -> PathBuf {
    let path = root.join("templates/custom");
    fs::create_dir_all(&path).unwrap();
    for (name, bytes) in embedded_files() {
        fs::write(path.join(name), bytes).unwrap();
    }
    edit_config(
        root,
        "id = \"default-report\"",
        "path = \"templates/custom\"",
    );
    path
}
pub fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn scan(base: &Path, path: &Path, output: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                output.insert(format!("{name}/"), String::new());
                scan(base, &path, output);
            } else {
                output.insert(
                    name,
                    format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
                );
            }
        }
    }
    let mut output = BTreeMap::new();
    scan(root, root, &mut output);
    output
}
