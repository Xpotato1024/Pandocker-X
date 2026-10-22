mod support;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use support::*;

fn binary() -> PathBuf {
    let test = std::env::current_exe().unwrap();
    let binary = test
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(format!("pdx{}", std::env::consts::EXE_SUFFIX));
    assert!(
        binary.is_file(),
        "real CLI must be built by the pdx package integration target: {}",
        binary.display()
    );
    binary
}
fn run_at(binary: &Path, cwd: &Path, args: &[&std::ffi::OsStr]) -> (i32, Value) {
    let output = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    assert_eq!(value["schema_version"], 1);
    assert!(value.is_object());
    let code = output.status.code().unwrap();
    assert_eq!(value["status"], if code == 0 { "success" } else { "error" });
    assert!(value["unverified"].is_array());
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/result-v1.schema.json")).unwrap();
    jsonschema::draft7::new(&schema)
        .unwrap()
        .validate(&value)
        .unwrap();
    (code, value)
}
#[test]
fn real_binary_outside_checkout_runs_init_check_plan_and_nearest_root() {
    let temp = sandbox();
    let install = temp.path().join("standalone app");
    fs::create_dir(&install).unwrap();
    let executable = install.join(format!("pdx{}", std::env::consts::EXE_SUFFIX));
    fs::copy(binary(), &executable).unwrap();
    let root = temp.path().join("独立 原稿");
    let (code, init) = run_at(
        &executable,
        &install,
        &[
            "init".as_ref(),
            root.as_os_str(),
            "--template".as_ref(),
            "default-report".as_ref(),
            "--json".as_ref(),
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(init["data"]["build_ready"], false);
    let before = snapshot(&root);
    for operation in ["check", "plan"] {
        let (code, result) = run_at(
            &executable,
            &install,
            &[operation.as_ref(), root.as_os_str(), "--json".as_ref()],
        );
        assert_eq!(code, 0);
        assert_eq!(result["data"]["static_only"], true);
        assert_eq!(result["data"]["runtime_verified"], false);
    }
    let child = root.join("content/deep");
    fs::create_dir(&child).unwrap();
    let (code, result) = run_at(&executable, &child, &["--json".as_ref(), "plan".as_ref()]);
    assert_eq!(code, 0);
    assert_eq!(result["data"]["target"], "report");
    let (code, _) = run_at(
        &executable,
        &child,
        &["check".as_ref(), child.as_os_str(), "--json".as_ref()],
    );
    assert_eq!(code, 2);
    let nested = child.join("nested");
    assert_eq!(
        run_at(
            &executable,
            &install,
            &["init".as_ref(), nested.as_os_str(), "--json".as_ref()]
        )
        .0,
        0
    );
    let (_, result) = run_at(&executable, &nested, &["check".as_ref(), "--json".as_ref()]);
    assert_eq!(
        Path::new(result["data"]["project_root"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        nested.canonicalize().unwrap()
    );
    fs::remove_dir_all(&child).unwrap();
    assert_eq!(before, snapshot(&root));
}

#[test]
fn json_arguments_and_operation_errors_are_one_object() {
    let temp = sandbox();
    let binary = binary();
    for args in [
        vec!["--json"],
        vec!["--json", "nonsense"],
        vec!["check", "--unknown", "--json"],
        vec!["plan", "--target", "--json"],
        vec!["init", "--json"],
        vec!["--json=true", "check"],
        vec!["build", "--json"],
    ] {
        let args: Vec<_> = args.iter().map(std::ffi::OsStr::new).collect();
        let (code, result) = run_at(&binary, temp.path(), &args);
        assert_eq!(code, 2);
        assert_eq!(result["error"]["code"], "invalid_arguments");
    }
    let root = project(temp.path());
    let (code, result) = run_at(
        &binary,
        temp.path(),
        &["init".as_ref(), root.as_os_str(), "--json".as_ref()],
    );
    assert_eq!(code, 4);
    assert_eq!(result["error"]["code"], "init_collision");
    fs::write(root.join("pdx.toml"), "broken [").unwrap();
    let before = snapshot(&root);
    let (code, result) = run_at(
        &binary,
        temp.path(),
        &["check".as_ref(), root.as_os_str(), "--json".as_ref()],
    );
    assert_eq!(code, 2);
    assert_eq!(result["error"]["code"], "invalid_toml");
    assert_eq!(before, snapshot(&root));
}

#[test]
fn real_cli_dependency_safety_and_save_exit_codes() {
    let temp = sandbox();
    let root = project(temp.path());
    let executable = binary();
    fs::remove_file(root.join("pdx.lock")).unwrap();
    let (code, result) = run_at(
        &executable,
        temp.path(),
        &["check".as_ref(), root.as_os_str(), "--json".as_ref()],
    );
    assert_eq!(code, 3);
    assert_eq!(result["error"]["code"], "missing_dependency");
    let (code, _) = run_at(
        &executable,
        temp.path(),
        &["lock".as_ref(), root.as_os_str(), "--json".as_ref()],
    );
    assert_eq!(code, 0);
    edit_config(&root, "output/report", "../escape");
    let (code, result) = run_at(
        &executable,
        temp.path(),
        &["plan".as_ref(), root.as_os_str(), "--json".as_ref()],
    );
    assert_eq!(code, 4);
    assert_eq!(result["error"]["code"], "unsafe_path");
    let missing_parent = temp.path().join("nonexistent/child");
    let (code, result) = run_at(
        &executable,
        temp.path(),
        &[
            "init".as_ref(),
            missing_parent.as_os_str(),
            "--json".as_ref(),
        ],
    );
    assert_eq!(code, 5);
    assert_eq!(result["error"]["code"], "save_failed");
    assert!(!missing_parent.exists());
}

#[test]
fn help_and_version_are_documented_plain_text_exceptions() {
    let temp = sandbox();
    for flag in ["--help", "--version"] {
        let output = Command::new(binary())
            .args(["--json", flag])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert!(serde_json::from_slice::<Value>(&output.stdout).is_err());
    }
}

#[test]
fn human_output_guides_template_switch_and_error_recovery() {
    let temp = sandbox();
    let executable = binary();
    let root = temp.path().join("新しい 原稿");
    let text = |args: &[&std::ffi::OsStr], code| {
        let output = Command::new(&executable)
            .args(args)
            .current_dir(temp.path())
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(code));
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        if code == 0 {
            assert!(stderr.is_empty(), "{stderr}");
            assert!(stdout.contains("build_ready=false"), "{stdout}");
        } else {
            assert!(stdout.is_empty(), "{stdout}");
            assert!(stderr.contains("次:"), "{stderr}");
        }
        (stdout, stderr)
    };
    let (init, _) = text(&["init".as_ref(), root.as_os_str()], 0);
    assert!(init.contains("pdx check"));
    let before = snapshot(&root);
    let (checked, _) = text(&["check".as_ref(), root.as_os_str()], 0);
    assert!(checked.contains("Target: report"));
    let (planned, _) = text(&["plan".as_ref(), root.as_os_str()], 0);
    assert!(planned.contains("内蔵template: default-report"));
    assert!(planned.contains("content/report.md"));
    assert!(planned.contains("output/report"));
    assert_eq!(before, snapshot(&root));

    let template = local_template(&root);
    let manifest = template.join("template.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("default-report", "local-report"),
    )
    .unwrap();
    let before = snapshot(&root);
    let (_, error) = text(&["check".as_ref(), root.as_os_str()], 3);
    assert!(error.contains("[stale_lock]"));
    assert!(error.contains("pdx lock"));
    assert_eq!(before, snapshot(&root));
    text(&["lock".as_ref(), root.as_os_str()], 0);
    let before = snapshot(&root);
    let (planned, _) = text(&["plan".as_ref(), root.as_os_str()], 0);
    assert!(planned.contains("Template: local-report @"));
    assert!(planned.contains("Template path: templates/custom"));
    let (_, error) = text(
        &[
            "plan".as_ref(),
            root.as_os_str(),
            "--target".as_ref(),
            "missing".as_ref(),
        ],
        2,
    );
    assert!(error.contains("[unknown_target]"));
    assert!(error.contains("pdx check"));
    assert_eq!(before, snapshot(&root));

    fs::write(
        root.join("metadata/common.yaml"),
        "title: 42\nauthor: [著者名]\n",
    )
    .unwrap();
    let before = snapshot(&root);
    let (_, error) = text(&["check".as_ref(), root.as_os_str()], 2);
    assert!(error.contains("[metadata_schema]"));
    assert!(error.contains("/title"));
    assert!(error.contains("metadata.schema.json"));
    assert_eq!(before, snapshot(&root));
    let (_, error) = text(&["check".as_ref(), "--unknown".as_ref()], 2);
    assert!(error.contains("[invalid_arguments]"));
    assert!(error.contains("pdx --help"));
}
