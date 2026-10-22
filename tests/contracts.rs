mod support;
use pdx_adapters::LocalStore;
use pdx_application::ProjectStore;
use pdx_core::*;
use serde_json::json;
use std::{collections::BTreeMap, fs};
use support::*;

#[test]
fn versioned_machine_contracts_match_typed_results() {
    fn validate(schema: &str, value: serde_json::Value) {
        let schema: serde_json::Value = serde_json::from_str(schema).unwrap();
        jsonschema::draft7::meta::validate(&schema).unwrap();
        jsonschema::draft7::new(&schema)
            .unwrap()
            .validate(&value)
            .unwrap();
    }
    let temp = sandbox();
    let root = project(temp.path());
    let project =
        pdx_core::parse_project(&fs::read_to_string(root.join("pdx.toml")).unwrap()).unwrap();
    validate(
        include_str!("../schemas/project-v1.schema.json"),
        serde_json::to_value(project).unwrap(),
    );
    let lock = pdx_core::parse_lock(&fs::read_to_string(root.join("pdx.lock")).unwrap()).unwrap();
    validate(
        include_str!("../schemas/lock-v1.schema.json"),
        serde_json::to_value(lock).unwrap(),
    );
    let package = resolve_package(&pdx_adapters::embedded_files()).unwrap();
    validate(
        include_str!("../schemas/template-v1.schema.json"),
        serde_json::to_value(package.manifest).unwrap(),
    );
    validate(
        include_str!("../schemas/plan-v1.schema.json"),
        serde_json::to_value(app().plan(Some(&root), None).unwrap()).unwrap(),
    );
}

#[test]
fn embedded_package_is_self_contained() {
    pdx_core::resolve_package(&pdx_adapters::embedded_files()).unwrap();
}

#[test]
fn init_check_plan_japanese_path_and_preservation() {
    let temp = sandbox();
    let root = project(temp.path());
    let before = snapshot(&root);
    // Verbatim canonical roots on Windows must not query a drive prefix alone.
    app().check(Some(&root.canonicalize().unwrap())).unwrap();
    let a = serde_json::to_value(app().plan(Some(&root), None).unwrap()).unwrap();
    let b = serde_json::to_value(app().plan(Some(&root), None).unwrap()).unwrap();
    assert_eq!(a, b);
    assert_eq!(before, snapshot(&root));
    assert_eq!(a["static_only"], true);
    assert_eq!(a["runtime_verified"], false);
    assert_eq!(a["build_ready"], false);
    assert_eq!(a["runtime"], "unresolved");
    assert!(a.get("command_line").is_none());
    assert!(!root.join("output").exists());
    assert!(!root.join(".pdx").exists());
}

#[test]
fn sources_order_crlf_two_targets_and_metadata_merge() {
    let temp = sandbox();
    let root = project(temp.path());
    fs::write(root.join("content/second.md"), "# second\r\n").unwrap();
    fs::write(
        root.join("metadata/common.yaml"),
        "title: document\r\nauthor: [one, two]\r\nsettings:\r\n  numbersections: true\r\n",
    )
    .unwrap();
    fs::write(
        root.join("metadata/target.yaml"),
        "title: target\nauthor: [three]\nsettings:\n  toc: false\n",
    )
    .unwrap();
    edit_config(
        &root,
        "sources = [\"content/report.md\"]",
        "sources = [\"content/second.md\", \"content/report.md\"]",
    );
    let mut config = fs::read_to_string(root.join("pdx.toml")).unwrap();
    config.push_str("\n[targets.alternate]\ndocument = \"report\"\ntemplate = { id = \"default-report\" }\nmetadata = \"metadata/target.yaml\"\noutput_dir = \"output/alternate\"\n");
    fs::write(root.join("pdx.toml"), config.replace('\n', "\r\n")).unwrap();
    app().lock(Some(&root)).unwrap();
    let before = snapshot(&root);
    let first = app().plan(Some(&root), None).unwrap();
    let second = app().plan(Some(&root), Some("alternate")).unwrap();
    assert_eq!(first.target, "report");
    assert_eq!(
        first.ordered_inputs,
        ["content/second.md", "content/report.md"]
    );
    assert_eq!(first.merged_metadata["title"], "document");
    assert_eq!(
        second.merged_metadata,
        json!({"title":"target","author":["three"],"lang":"ja-JP","settings":{"toc":false,"numbersections":true}})
    );
    assert_eq!(before, snapshot(&root));
    assert_eq!(
        app().plan(Some(&root), Some("unknown")).unwrap_err().code,
        "unknown_target"
    );
}

#[test]
fn manuscript_edits_do_not_change_dependency_lock() {
    let temp = sandbox();
    let root = project(temp.path());
    let lock = fs::read(root.join("pdx.lock")).unwrap();
    fs::write(root.join("content/report.md"), "# changed").unwrap();
    fs::write(root.join("figures/new.svg"), "<svg/>").unwrap();
    fs::write(
        root.join("metadata/common.yaml"),
        "title: changed\nauthor: [other]\n",
    )
    .unwrap();
    let before = snapshot(&root);
    app().check(Some(&root)).unwrap();
    app().plan(Some(&root), None).unwrap();
    assert_eq!(lock, fs::read(root.join("pdx.lock")).unwrap());
    assert_eq!(before, snapshot(&root));
}

#[test]
fn configuration_error_matrix_preserves_files() {
    for (from, to, code) in [
        (
            "schema_version = 1",
            "schema_version = 99",
            "unsupported_schema",
        ),
        (
            "schema_version = 1",
            "schema_version = \"1\"",
            "invalid_toml",
        ),
        (
            "schema_version = 1",
            "schema_version = 1\nschema_version = 1",
            "invalid_toml",
        ),
        (
            "schema_version = 1",
            "unknown = true\nschema_version = 1",
            "invalid_toml",
        ),
        ("schema_version = 1", "schema_version = [", "invalid_toml"),
        (
            "default_target = \"report\"",
            "default_target = \"missing\"",
            "unknown_target",
        ),
        (
            "document = \"report\"",
            "document = \"missing\"",
            "unknown_document",
        ),
        (
            "sources = [\"content/report.md\"]",
            "sources = []",
            "empty_sources",
        ),
        (
            "sources = [\"content/report.md\"]",
            "sources = [\"missing.md\"]",
            "missing_input",
        ),
        (
            "format = \"markdown\"",
            "format = \"latex\"",
            "unsupported_format",
        ),
        (
            "id = \"default-report\"",
            "id = \"unregistered\"",
            "missing_dependency",
        ),
        (
            "id = \"default-report\"",
            "id = \"default-report\", path = \"templates/local\"",
            "invalid_template_ref",
        ),
    ] {
        let temp = sandbox();
        let root = project(temp.path());
        edit_config(&root, from, to);
        let before = snapshot(&root);
        let error = app().check(Some(&root)).err().expect(to);
        assert_eq!(error.code, code, "{to}: {error}");
        assert!(app().plan(Some(&root), None).is_err());
        assert_eq!(before, snapshot(&root));
    }
}

#[test]
fn metadata_error_matrix() {
    for (text, code) in [
        ("author: [one]", "metadata_schema"),
        ("title: 1\nauthor: [one]", "metadata_schema"),
        ("title: a\nauthor: [one]\nunknown: true", "metadata_schema"),
        ("title: a\ntitle: b\nauthor: [one]", "invalid_metadata"),
        ("title: a\n---\nauthor: [one]", "invalid_metadata"),
        ("title: a\nauthor: [one]\n---", "invalid_metadata"),
        ("1: a", "invalid_metadata"),
        ("title: !include other.yaml", "invalid_metadata"),
        ("title: !dangerous data", "invalid_metadata"),
        ("title: &a x\nauthor: [*a]", "invalid_metadata"),
    ] {
        let temp = sandbox();
        let root = project(temp.path());
        fs::write(root.join("metadata/common.yaml"), text).unwrap();
        let before = snapshot(&root);
        let error = app().check(Some(&root)).err().expect(text);
        assert_eq!(error.code, code, "{text}: {error}");
        assert_eq!(before, snapshot(&root));
        if code == "metadata_schema" {
            assert!(error.instance_path.is_some());
        }
    }
}

#[test]
fn separate_documents_and_all_target_validation() {
    let temp = sandbox();
    let root = project(temp.path());
    fs::write(root.join("content/other.md"), "# Other document").unwrap();
    fs::write(
        root.join("metadata/other.yaml"),
        "title: Other\nauthor: [Other]\n",
    )
    .unwrap();
    let mut config = fs::read_to_string(root.join("pdx.toml")).unwrap();
    config.push_str("\n[documents.other]\nformat='markdown'\nsources=['content/other.md']\nmetadata='metadata/other.yaml'\nbibliography=[]\nresources=[]\n[targets.other]\ndocument='other'\ntemplate={id='default-report'}\noutput_dir='output/other'\n");
    fs::write(root.join("pdx.toml"), config).unwrap();
    app().lock(Some(&root)).unwrap();
    let plan = app().plan(Some(&root), Some("other")).unwrap();
    assert_eq!(plan.document, "other");
    assert_eq!(plan.ordered_inputs, ["content/other.md"]);
    assert_eq!(plan.merged_metadata["title"], "Other");
    fs::write(root.join("metadata/other.yaml"), "author: [Other]").unwrap();
    let before = snapshot(&root);
    assert_eq!(
        app().plan(Some(&root), Some("report")).unwrap_err().code,
        "metadata_schema"
    );
    assert_eq!(before, snapshot(&root));
}

#[test]
fn missing_template_assets_and_undeclared_files_are_distinct() {
    let temp = sandbox();
    let root = project(temp.path());
    let template = local_template(&root);
    fs::remove_file(template.join("sample.md")).unwrap();
    let error = app().lock(Some(&root)).err().unwrap();
    assert_eq!(error.code, "missing_dependency");
    assert_eq!(error.exit_code, 3);
    fs::write(template.join("sample.md"), "# sample").unwrap();
    fs::write(template.join("undeclared"), "unknown").unwrap();
    assert_eq!(
        app().lock(Some(&root)).err().unwrap().code,
        "invalid_template"
    );
}

#[test]
fn portable_path_and_output_overlap_matrix() {
    for path in [
        "../escape",
        "/absolute",
        "C:/absolute",
        "//host/share",
        "a\\b",
        "a:stream",
        "NUL.txt",
        "com1",
        "a.",
        "a ",
        "content/report.md",
        "content",
        "metadata",
        "metadata/common.yaml",
        "references.bib",
        "figures/out",
        "figures",
        "pdx.toml/out",
        "pdx.lock/out",
        ".git/out",
        ".pdx/out",
    ] {
        let temp = sandbox();
        let root = project(temp.path());
        // Literal TOML strings preserve backslashes.
        edit_config(&root, "\"output/report\"", &format!("'{path}'"));
        let before = snapshot(&root);
        let error = app().check(Some(&root)).err().expect(path);
        assert_eq!(error.exit_code, 4, "{path}: {error}");
        assert_eq!(before, snapshot(&root));
    }
    for output in [
        "output/report",
        "output/report/sub",
        "output",
        "OUTPUT/report",
        "output\\report",
    ] {
        let temp = sandbox();
        let root = project(temp.path());
        let mut config = fs::read_to_string(root.join("pdx.toml")).unwrap();
        config.push_str(&format!("\n[targets.other]\ndocument='report'\ntemplate={{id='default-report'}}\noutput_dir='{output}'\n"));
        fs::write(root.join("pdx.toml"), config).unwrap();
        assert_eq!(
            app().check(Some(&root)).err().unwrap().exit_code,
            4,
            "{output}"
        );
    }
}

#[test]
fn filesystem_alias_and_non_directory_outputs() {
    let temp = sandbox();
    let root = project(temp.path());
    edit_config(&root, "content/report.md", "Content/report.md");
    assert_eq!(app().check(Some(&root)).err().unwrap().exit_code, 4);
    edit_config(&root, "Content/report.md", "content/report.md");
    fs::write(root.join("output"), "user file").unwrap();
    let before = snapshot(&root);
    assert_eq!(app().check(Some(&root)).err().unwrap().exit_code, 4);
    assert_eq!(before, snapshot(&root));
}

#[test]
fn init_collision_rollback_and_concurrent_init() {
    let temp = sandbox();
    let root = project(temp.path());
    let before = snapshot(&root);
    assert_eq!(
        app().init(&root, "default-report").unwrap_err().code,
        "init_collision"
    );
    assert_eq!(before, snapshot(&root));
    let empty = temp.path().join("empty");
    fs::create_dir(&empty).unwrap();
    let files = BTreeMap::from([
        ("a".into(), b"owned".to_vec()),
        ("z/missing".into(), vec![0]),
    ]);
    assert_eq!(
        LocalStore
            .initialize(&empty, &files, &[])
            .unwrap_err()
            .exit_code,
        5
    );
    assert!(empty.is_dir());
    assert_eq!(fs::read_dir(&empty).unwrap().count(), 0);
    let failed = temp.path().join("new failure");
    assert!(LocalStore.initialize(&failed, &files, &[]).is_err());
    assert!(!failed.exists());
    let concurrent = temp.path().join("concurrent");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let root = concurrent.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                app().init(&root, "default-report").is_ok()
            })
        })
        .collect();
    let successes = threads
        .into_iter()
        .map(|t| usize::from(t.join().unwrap()))
        .sum::<usize>();
    assert_eq!(successes, 1);
    app().check(Some(&concurrent)).unwrap();
    assert!(!concurrent.join(".pdx-init.guard").exists());
}

#[test]
fn local_package_identity_digest_and_explicit_lock() {
    let temp = sandbox();
    let root = project(temp.path());
    let template = local_template(&root);
    // Same identity and bytes remain locked when switching from embedded to local.
    app().check(Some(&root)).unwrap();
    app().lock(Some(&root)).unwrap();
    let initial = app().plan(Some(&root), None).unwrap().template;
    // Add a new template identity with no institution-name branch in Rust.
    let manifest = fs::read_to_string(template.join("template.toml")).unwrap();
    fs::write(
        template.join("template.toml"),
        manifest.replace("default-report", "conference-example"),
    )
    .unwrap();
    let before = snapshot(&root);
    assert_eq!(app().check(Some(&root)).err().unwrap().code, "stale_lock");
    assert_eq!(before, snapshot(&root));
    app().lock(Some(&root)).unwrap();
    let changed = app().plan(Some(&root), None).unwrap().template;
    assert_eq!(changed.id, "conference-example");
    assert_ne!(changed.digest, initial.digest);
    fs::write(template.join("sample.md"), "# different bytes").unwrap();
    assert_eq!(
        app().plan(Some(&root), None).unwrap_err().code,
        "stale_lock"
    );
    app().lock(Some(&root)).unwrap();
    let other = sandbox();
    let copy = project(other.path());
    local_template(&copy);
    for (name, _) in pdx_adapters::embedded_files() {
        fs::copy(
            template.join(&name),
            copy.join("templates/custom").join(name),
        )
        .unwrap();
    }
    app().lock(Some(&copy)).unwrap();
    assert_eq!(
        app().plan(Some(&root), None).unwrap().template.digest,
        app().plan(Some(&copy), None).unwrap().template.digest
    );
}

#[test]
fn template_and_schema_rejections_and_local_refs() {
    for (file, from, to, code) in [
        (
            "template.toml",
            "markdown-standalone",
            "arbitrary-command",
            "unsupported_recipe",
        ),
        (
            "template.toml",
            "engine = \"lualatex\"",
            "engine = \"platex\"",
            "tool_mismatch",
        ),
        (
            "template.toml",
            "[\"pandoc\", \"lualatex\"]",
            "[\"pandoc\"]",
            "tool_mismatch",
        ),
        (
            "template.toml",
            "schema_version = 1",
            "unknown = true\nschema_version = 1",
            "invalid_toml",
        ),
        (
            "metadata.schema.json",
            "\"type\": \"object\"",
            "\"type\": 42",
            "invalid_schema",
        ),
    ] {
        let temp = sandbox();
        let root = project(temp.path());
        let template = local_template(&root);
        let path = template.join(file);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(from));
        fs::write(path, text.replace(from, to)).unwrap();
        assert_eq!(app().lock(Some(&root)).err().unwrap().code, code);
    }
    for reference in [
        "https://example.invalid/schema",
        "file:///secret",
        "other.json#/x",
    ] {
        let temp = sandbox();
        let root = project(temp.path());
        let template = local_template(&root);
        fs::write(
            template.join("metadata.schema.json"),
            json!({"$schema":"http://json-schema.org/draft-07/schema#","$ref":reference})
                .to_string(),
        )
        .unwrap();
        assert_eq!(
            app().lock(Some(&root)).err().unwrap().code,
            "external_schema_ref"
        );
    }
    let temp = sandbox();
    let root = project(temp.path());
    let template = local_template(&root);
    fs::write(template.join("metadata.schema.json"),json!({"$schema":"http://json-schema.org/draft-07/schema#","type":"object","properties":{"title":{"$ref":"#/definitions/title"}},"definitions":{"title":{"type":"string"}}}).to_string()).unwrap();
    app().lock(Some(&root)).unwrap();
    app().check(Some(&root)).unwrap();
    fs::write(template.join("metadata.schema.json"),json!({"$schema":"http://json-schema.org/draft-07/schema#","$ref":"#/definitions/cycle","definitions":{"cycle":{"$ref":"#/definitions/cycle"}}}).to_string()).unwrap();
    assert_eq!(app().lock(Some(&root)).err().unwrap().code, "schema_budget");
}

#[test]
fn missing_corrupt_lock_and_metadata_budgets() {
    let temp = sandbox();
    let root = project(temp.path());
    fs::remove_file(root.join("pdx.lock")).unwrap();
    assert_eq!(app().check(Some(&root)).err().unwrap().exit_code, 3);
    app().lock(Some(&root)).unwrap();
    let text = fs::read_to_string(root.join("pdx.lock")).unwrap();
    fs::write(
        root.join("pdx.lock"),
        text.replacen(
            "\"schema_version\": 1",
            "\"schema_version\": 1, \"schema_version\": 1",
            1,
        ),
    )
    .unwrap();
    assert_eq!(app().check(Some(&root)).err().unwrap().code, "invalid_lock");
    assert!(parse_metadata(&format!("a: {}", "x".repeat(METADATA_BYTES))).is_err());
    assert!(parse_metadata(&format!("a: {}0{}", "[".repeat(40), "]".repeat(40))).is_err());
    assert!(parse_metadata(&format!("a: [{}]", vec!["0"; 11000].join(","))).is_err());
}

#[test]
fn schema_names_and_literal_data_are_not_schema_keywords() {
    let temp = sandbox();
    let root = project(temp.path());
    let template = local_template(&root);
    fs::write(
        template.join("metadata.schema.json"),
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "$id": {"type": "string"},
                "$ref": {"type": "string"},
                "patternProperties": {"type": "object"},
                "$dynamicRef": {"type": "string"},
                "$recursiveRef": {"type": "string"}
            },
            "definitions": {
                "pattern": {"type": "string"},
                "$ref": {"type": "string"}
            },
            "dependencies": {"pattern": ["$ref"]},
            "default": {"pattern": ".*", "$ref": "https://example.invalid/schema"},
            "examples": [{"$id": "data"}],
            "not": {"const": {"$ref": "literal", "pattern": "literal"}},
            "enum": [{"title": "新しいレポート", "author": ["著者名"], "lang": "ja-JP", "settings": {"toc": true, "numbersections": false}}]
        })
        .to_string(),
    )
    .unwrap();
    app().lock(Some(&root)).unwrap();
    let before = snapshot(&root);
    app().check(Some(&root)).unwrap();
    app().plan(Some(&root), None).unwrap();
    assert_eq!(before, snapshot(&root));
    // Schema constraints still apply after accepting names and literal data.
    fs::write(root.join("metadata/common.yaml"), "title: different\n").unwrap();
    let error = app().check(Some(&root)).err().unwrap();
    assert_eq!(error.code, "metadata_schema");
    assert!(error.instance_path.is_some());
}

#[test]
fn schema_guard_rejects_percent_encoded_reference_bypass() {
    let temp = sandbox();
    let root = project(temp.path());
    let template = local_template(&root);
    fs::write(
        template.join("metadata.schema.json"),
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "$ref": "#/default/%61",
            "default": {"%61": true, "a": {"properties": {"title": {"pattern": "^x$"}}}}
        })
        .to_string(),
    )
    .unwrap();
    let error = app().lock(Some(&root)).err().unwrap();
    assert_eq!(error.code, "unsupported_schema_keyword");
    fs::write(
        template.join("metadata.schema.json"),
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {"title": {"type": "string"}},
            "properties": {"title": {"$ref": "#/definitions/%74itle"}}
        })
        .to_string(),
    )
    .unwrap();
    app().lock(Some(&root)).unwrap();
    app().check(Some(&root)).unwrap();
}

#[test]
fn schema_guard_rejects_nested_dialect_switch() {
    let temp = sandbox();
    let root = project(temp.path());
    let template = local_template(&root);
    fs::write(
        template.join("metadata.schema.json"),
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "properties": {"probe": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "prefixItems": [{"pattern": "^x$"}]
            }}
        })
        .to_string(),
    )
    .unwrap();
    let error = app().lock(Some(&root)).err().unwrap();
    assert_eq!(error.code, "unsupported_schema");
}

#[test]
fn explicit_project_path_does_not_search_parents() {
    let temp = sandbox();
    let root = project(temp.path());
    let child = root.join("content");
    assert_eq!(
        app().check(Some(&child)).err().unwrap().code,
        "missing_input"
    );
}

#[cfg(unix)]
#[test]
fn symlinks_are_rejected_in_all_filesystem_roles() {
    use std::os::unix::fs::symlink;
    for role in ["input", "resource", "output", "template", "config", "lock"] {
        let temp = sandbox();
        let root = project(temp.path());
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("file"), "outside").unwrap();
        match role {
            "input" => {
                fs::remove_file(root.join("content/report.md")).unwrap();
                symlink(outside.join("file"), root.join("content/report.md")).unwrap();
            }
            "resource" => {
                symlink(&outside, root.join("figures/link")).unwrap();
            }
            "output" => {
                symlink(&outside, root.join("output")).unwrap();
            }
            "template" => {
                edit_config(&root, "id = \"default-report\"", "path = \"template\"");
                symlink(&outside, root.join("template")).unwrap();
            }
            "config" => {
                fs::remove_file(root.join("pdx.toml")).unwrap();
                symlink(outside.join("file"), root.join("pdx.toml")).unwrap();
            }
            "lock" => {
                fs::remove_file(root.join("pdx.lock")).unwrap();
                symlink(outside.join("file"), root.join("pdx.lock")).unwrap();
            }
            _ => unreachable!(),
        }
        assert_eq!(
            app().check(Some(&root)).err().unwrap().exit_code,
            4,
            "{role}"
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_junctions_are_rejected_without_symlink_privilege() {
    for role in ["resource", "output", "template", "root"] {
        let temp = sandbox();
        let root = project(temp.path());
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        let link = match role {
            "resource" => root.join("figures/link"),
            "output" => root.join("output"),
            "template" => {
                edit_config(&root, "id = \"default-report\"", "path = \"template\"");
                root.join("template")
            }
            "root" => temp.path().join("root-link"),
            _ => unreachable!(),
        };
        // cmd builtin has no privilege requirement for a directory junction. Paths are task-created.
        let target = if role == "root" { &root } else { &outside };
        let status = std::process::Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let checked = if role == "root" { &link } else { &root };
        assert_eq!(
            app().check(Some(checked)).err().unwrap().exit_code,
            4,
            "{role}"
        );
        // Remove the junction itself before TempDir cleanup; never traverse its target.
        fs::remove_dir(&link).unwrap();
        assert!(target.is_dir());
    }
}
