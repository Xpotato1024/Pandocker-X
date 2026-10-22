mod support;
use pdx_application::*;
use pdx_core::*;
use std::{collections::BTreeMap, fs, path::Path, time::Duration};
use support::*;

const IMAGE: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PDF: &[u8] = b"%PDF-1.4\nsynthetic test double, not a readable PDF\n%%EOF\n";
struct Fake<'a> {
    mode: &'a str,
    original: &'a Path,
}
impl BuildRuntime for Fake<'_> {
    fn probe(&self, _: &RuntimePin, _: &Cancellation, _: Duration) -> Result<RuntimeEvidence> {
        if self.mode == "missing" {
            return Err(Diagnostic::new("missing_runtime", 3, "fake missing"));
        }
        Ok(RuntimeEvidence {
            driver: "fake".into(),
            runtime_verified: false,
            tools: BTreeMap::new(),
        })
    }
    fn render(&self, request: RenderRequest<'_>, cancel: &Cancellation) -> Result<RenderedPdf> {
        assert!(request.input_dir.join("content/report.md").exists());
        assert!(!request.input_dir.join("private.txt").exists());
        assert!(!request.input_dir.join("output").exists());
        match self.mode {
            "fail" => return Err(Diagnostic::new("tex_failed", 1, "fake typesetting failure")),
            "timeout" => return Err(Diagnostic::new("build_timeout", 1, "fake timeout")),
            "cancel" => cancel.cancel(),
            "change" => fs::write(self.original.join("content/report.md"), "changed").unwrap(),
            "conflict" => {
                let generation = self.original.join("output/report").join(request.build_id);
                fs::create_dir_all(&generation).unwrap();
                fs::write(generation.join("unowned.txt"), "preserve").unwrap();
            }
            _ => (),
        }
        Ok(RenderedPdf {
            bytes: if self.mode == "invalid" {
                Vec::new()
            } else {
                PDF.to_vec()
            },
            pdf_verified: false,
        })
    }
}
fn settings(parent: &Path) -> BuildSettings {
    BuildSettings {
        state_root: parent.join("state"),
        timeout: Duration::from_secs(10),
    }
}
fn pin(root: &Path) {
    app()
        .pin_runtime(
            Some(root),
            IMAGE,
            &Fake {
                mode: "ok",
                original: root,
            },
            &Cancellation::default(),
        )
        .unwrap();
}

#[test]
fn runtime_pin_is_explicit_versioned_and_preserved() {
    let temp = sandbox();
    let root = project(temp.path());
    let before = snapshot(&root);
    assert_eq!(
        app()
            .build(
                Some(&root),
                None,
                &settings(temp.path()),
                &Fake {
                    mode: "ok",
                    original: &root
                },
                &Cancellation::default()
            )
            .unwrap_err()
            .code,
        "missing_runtime"
    );
    assert_eq!(before, snapshot(&root));
    assert!(RuntimePin::local("latest").is_err());
    pin(&root);
    app().lock(Some(&root)).unwrap();
    let lock = parse_lock(&fs::read_to_string(root.join("pdx.lock")).unwrap()).unwrap();
    assert_eq!(lock.schema_version, 2);
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/lock-v2.schema.json")).unwrap();
    jsonschema::draft7::new(&schema)
        .unwrap()
        .validate(&serde_json::to_value(&lock).unwrap())
        .unwrap();
    assert!(matches!(lock.runtime, RuntimeResolution::Docker(_)));
    let plan = app().plan(Some(&root), None).unwrap();
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/plan-v2.schema.json")).unwrap();
    jsonschema::draft7::new(&schema)
        .unwrap()
        .validate(&serde_json::to_value(&plan).unwrap())
        .unwrap();
    assert_eq!(plan.schema_version, 2);
    assert!(!plan.status.runtime_verified);
}

#[test]
fn fake_build_creates_distinct_generations_without_claiming_real_pdf() {
    let temp = sandbox();
    let root = project(temp.path());
    pin(&root);
    fs::write(root.join("private.txt"), "not declared").unwrap();
    fs::create_dir_all(root.join("output/report")).unwrap();
    fs::write(root.join("output/report/previous.pdf"), b"old pdf").unwrap();
    let run = || {
        app()
            .build(
                Some(&root),
                None,
                &settings(temp.path()),
                &Fake {
                    mode: "ok",
                    original: &root,
                },
                &Cancellation::default(),
            )
            .unwrap()
    };
    let first = run();
    let before = snapshot(&root);
    let second = run();
    assert_ne!(first.build_id, second.build_id);
    for (name, hash) in before {
        assert_eq!(snapshot(&root)[&name], hash);
    }
    assert!(!first.runtime_verified && !first.pdf_verified);
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/build-result-v1.schema.json")).unwrap();
    jsonschema::draft7::new(&schema)
        .unwrap()
        .validate(&serde_json::to_value(&first).unwrap())
        .unwrap();
    let record: BuildRecord = serde_json::from_slice(&fs::read(first.record).unwrap()).unwrap();
    assert_eq!(record.evidence.driver, "fake");
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/build-record-v1.schema.json")).unwrap();
    jsonschema::draft7::new(&schema)
        .unwrap()
        .validate(&serde_json::to_value(&record).unwrap())
        .unwrap();
    assert_eq!(record.pdf.sha256, digest(PDF).sha256);
    assert!(record.inputs.contains_key(".pdx/merged-metadata.json"));
    assert_eq!(
        fs::read_dir(temp.path().join("state/workspaces"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn failures_cancellation_invalid_pdf_and_retry_preserve_previous_generation() {
    for (mode, code) in [
        ("fail", "tex_failed"),
        ("timeout", "build_timeout"),
        ("cancel", "cancelled"),
        ("invalid", "invalid_pdf"),
        ("missing", "missing_runtime"),
    ] {
        let temp = sandbox();
        let root = project(temp.path());
        pin(&root);
        let first = app()
            .build(
                Some(&root),
                None,
                &settings(temp.path()),
                &Fake {
                    mode: "ok",
                    original: &root,
                },
                &Cancellation::default(),
            )
            .unwrap();
        let before = snapshot(&root);
        let error = app()
            .build(
                Some(&root),
                None,
                &settings(temp.path()),
                &Fake {
                    mode,
                    original: &root,
                },
                &Cancellation::default(),
            )
            .unwrap_err();
        assert_eq!(error.code, code, "{mode}: {error}");
        assert_eq!(before, snapshot(&root));
        assert_eq!(fs::read(first.pdf).unwrap(), PDF);
        app()
            .build(
                Some(&root),
                None,
                &settings(temp.path()),
                &Fake {
                    mode: "ok",
                    original: &root,
                },
                &Cancellation::default(),
            )
            .unwrap();
    }
}

#[test]
fn input_changes_conflicts_and_state_overlap_are_rejected() {
    for mode in ["change", "conflict"] {
        let temp = sandbox();
        let root = project(temp.path());
        pin(&root);
        let error = app()
            .build(
                Some(&root),
                None,
                &settings(temp.path()),
                &Fake {
                    mode,
                    original: &root,
                },
                &Cancellation::default(),
            )
            .unwrap_err();
        assert_eq!(
            error.code,
            if mode == "change" {
                "input_changed"
            } else {
                "save_failed"
            }
        );
        if mode == "conflict" {
            let dirs = fs::read_dir(root.join("output/report"))
                .unwrap()
                .filter(|e| e.as_ref().unwrap().path().is_dir())
                .collect::<Vec<_>>();
            assert_eq!(dirs.len(), 1);
            assert!(
                dirs[0]
                    .as_ref()
                    .unwrap()
                    .path()
                    .join("unowned.txt")
                    .exists()
            );
        }
    }
    let temp = sandbox();
    let root = project(temp.path());
    pin(&root);
    let mut config = settings(temp.path());
    config.state_root = root.join("state");
    assert_eq!(
        app()
            .build(
                Some(&root),
                None,
                &config,
                &Fake {
                    mode: "ok",
                    original: &root
                },
                &Cancellation::default()
            )
            .unwrap_err()
            .code,
        "unsafe_path"
    );
    config.state_root = temp
        .path()
        .join("unused/..")
        .join(root.file_name().unwrap());
    assert_eq!(
        app()
            .build(
                Some(&root),
                None,
                &config,
                &Fake {
                    mode: "ok",
                    original: &root
                },
                &Cancellation::default()
            )
            .unwrap_err()
            .code,
        "unsafe_path"
    );
}

#[test]
fn concurrent_target_lease_is_released_and_local_template_is_snapshotted() {
    let temp = sandbox();
    let root = project(temp.path());
    local_template(&root);
    app().lock(Some(&root)).unwrap();
    pin(&root);
    let plan = app().plan(Some(&root), None).unwrap();
    let store = pdx_adapters::LocalStore;
    let mut first = store.begin(&root, &plan, &settings(temp.path())).unwrap();
    assert!(
        matches!(store.begin(&root,&plan,&settings(temp.path())),Err(e) if e.code=="build_busy")
    );
    let other = BuildSettings {
        state_root: temp.path().join("another-state"),
        timeout: Duration::from_secs(10),
    };
    assert!(matches!(store.begin(&root,&plan,&other),Err(e) if e.code=="build_busy"));
    first.finish(None).unwrap();
    drop(first);
    app()
        .build(
            Some(&root),
            None,
            &settings(temp.path()),
            &Fake {
                mode: "ok",
                original: &root,
            },
            &Cancellation::default(),
        )
        .unwrap();
}
