use std::{
    fs,
    path::{Path, PathBuf},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
fn files(path: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files(&path, extension));
        } else if path.extension().is_some_and(|e| e == extension) {
            found.push(path);
        }
    }
    found
}
#[test]
fn workspace_dependency_direction_and_pure_core() {
    let workspace: toml::Value = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    let members = workspace["workspace"]["members"].as_array().unwrap();
    assert_eq!(members.len(), 5);
    for member in [
        "apps/pdx",
        "crates/pdx-core",
        "crates/pdx-application",
        "crates/pdx-adapters",
        "tests",
    ] {
        assert!(members.iter().any(|m| m.as_str() == Some(member)));
    }
    for (package, allowed) in [
        (
            "crates/pdx-core",
            vec![
                "serde",
                "serde_json",
                "toml",
                "sha2",
                "jsonschema",
                "serde-saphyr",
                "percent-encoding",
            ],
        ),
        ("crates/pdx-application", vec!["pdx-core"]),
        (
            "crates/pdx-adapters",
            vec!["pdx-core", "pdx-application", "tempfile", "serde_json"],
        ),
    ] {
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(root().join(package).join("Cargo.toml")).unwrap())
                .unwrap();
        for name in manifest["dependencies"].as_table().unwrap().keys() {
            assert!(allowed.contains(&name.as_str()), "{package}: {name}");
        }
        for path in files(&root().join(package).join("src"), "rs") {
            let text = fs::read_to_string(&path).unwrap();
            if package != "crates/pdx-adapters" {
                for banned in [
                    "std::fs",
                    "std::env",
                    "std::process",
                    "std::io",
                    "println!",
                    "eprintln!",
                    "include_str!(\"../../../apps",
                ] {
                    assert!(!text.contains(banned), "{}: {banned}", path.display());
                }
            }
        }
    }
    let deps = &workspace["workspace"]["dependencies"];
    assert_eq!(
        deps["jsonschema"]["default-features"].as_bool(),
        Some(false)
    );
    assert!(deps["jsonschema"].get("features").is_none());
    assert_eq!(
        deps["serde-saphyr"]["features"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        deps["serde-saphyr"]["features"][0].as_str(),
        Some("deserialize")
    );
    assert!(include_str!("../rust-toolchain.toml").contains("1.99.0"));
}

#[test]
fn every_cross_crate_test_file_is_a_cargo_target() {
    let manifest: toml::Value = toml::from_str(include_str!("Cargo.toml")).unwrap();
    let targets = manifest["test"].as_array().unwrap();
    let declared: Vec<_> = targets
        .iter()
        .map(|t| t["path"].as_str().unwrap())
        .collect();
    for entry in fs::read_dir(root().join("tests")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            assert!(
                declared.contains(&path.file_name().unwrap().to_str().unwrap()),
                "unregistered test: {}",
                path.display()
            );
        }
    }
    assert_eq!(declared.len(), 4);
}

#[test]
fn canonical_docs_links_source_map_and_utf8_lf() {
    let root = root();
    let mut paths = files(&root.join("docs"), "md");
    paths.extend([
        root.join("README.md"),
        root.join("AGENTS.md"),
        root.join("CONTRIBUTING.md"),
    ]);
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains('\u{fffd}'), "{}", path.display());
        // Historical R0 evidence is immutable; current human-facing sources use LF.
        if !path.starts_with(root.join("docs/reports")) {
            assert!(!text.contains('\r'), "CRLF: {}", path.display());
        }
        for part in text.split("](").skip(1) {
            let destination = part.split(')').next().unwrap().split('#').next().unwrap();
            if destination.is_empty()
                || destination.contains("://")
                || destination.starts_with("mailto:")
            {
                continue;
            }
            assert!(
                path.parent().unwrap().join(destination).exists(),
                "{} -> {destination}",
                path.display()
            );
        }
    }
    let map = fs::read_to_string(root.join("docs/README.md")).unwrap();
    for canonical in [
        "contracts/project.md",
        "contracts/template.md",
        "contracts/cli.md",
        "contracts/build.md",
        "architecture/system.md",
        "requirements/functional.md",
        "requirements/non-functional.md",
        "operations/development.md",
        "operations/rounds.md",
    ] {
        assert!(map.contains(canonical), "{canonical}");
    }
    let architecture = fs::read_to_string(root.join("docs/architecture/system.md")).unwrap();
    for path in [
        "apps/pdx",
        "crates/pdx-core",
        "crates/pdx-application",
        "crates/pdx-adapters",
        "tests/Cargo.toml",
        "templates",
        "schemas",
    ] {
        assert!(architecture.contains(path));
        assert!(root.join(path).exists());
    }
}

#[test]
fn just_and_ci_only_invoke_implemented_validation() {
    let just = include_str!("../justfile");
    for line in just.lines().filter(|s| s.starts_with("    ")) {
        assert!(line.trim().starts_with("cargo "));
    }
    assert!(!just.contains("package-check"));
    assert!(!just.contains("pdf"));
    let ci = include_str!("../.github/workflows/static.yml");
    for required in [
        "windows-latest",
        "ubuntu-latest",
        "contents: read",
        "cargo fmt",
        "cargo check",
        "cargo clippy",
        "cargo test",
    ] {
        assert!(ci.contains(required));
    }
    for forbidden in [
        "pull_request_target",
        "tags:",
        "publish",
        "docker build",
        "contents: write",
    ] {
        assert!(!ci.contains(forbidden));
    }
}
