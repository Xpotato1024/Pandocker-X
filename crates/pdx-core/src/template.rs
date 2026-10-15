use crate::{
    Diagnostic, Result, TemplateIdentity, parse_metadata, parse_toml, reject_aliases, validate_id,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Recipe {
    MarkdownStandalone,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateManifest {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub recipe: Recipe,
    pub engine: String,
    pub required_tools: Vec<String>,
    pub metadata_schema: String,
    pub defaults: String,
    pub resources: Vec<String>,
    pub license: String,
    pub provenance: String,
    pub sample: String,
}

#[derive(Debug, Clone)]
pub struct TemplatePackage {
    pub manifest: TemplateManifest,
    pub identity: TemplateIdentity,
    pub defaults: Value,
    pub schema: Value,
    pub files: Vec<String>,
}

pub fn parse_manifest(text: &str) -> Result<TemplateManifest> {
    // Inspect recipe first so unsupported recipes have a stable dedicated code.
    let raw: toml::Value = parse_toml(text)?;
    if raw
        .get("recipe")
        .and_then(toml::Value::as_str)
        .is_some_and(|s| s != "markdown-standalone")
    {
        return Err(Diagnostic::input(
            "unsupported_recipe",
            "R1 supports markdown-standalone only",
        ));
    }
    let m: TemplateManifest = parse_toml(text)?;
    if m.schema_version != 1 {
        return Err(Diagnostic::input(
            "unsupported_schema",
            "template schema_version must be 1",
        ));
    }
    validate_id(&m.id)?;
    if m.version.is_empty() || m.license.trim().is_empty() || m.provenance.trim().is_empty() {
        return Err(Diagnostic::input(
            "invalid_template",
            "version/license/provenance must not be empty",
        ));
    }
    if m.engine != "lualatex"
        || m.required_tools.len() != 2
        || !m.required_tools.iter().any(|s| s == "pandoc")
        || !m.required_tools.iter().any(|s| s == "lualatex")
    {
        return Err(Diagnostic::input(
            "tool_mismatch",
            "markdown-standalone requires engine=lualatex and tools pandoc,lualatex",
        ));
    }
    let mut paths = vec![
        "template.toml",
        m.metadata_schema.as_str(),
        m.defaults.as_str(),
        m.sample.as_str(),
    ];
    paths.extend(m.resources.iter().map(String::as_str));
    reject_aliases(paths.iter().copied())?;
    let mut unique = paths.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != paths.len() {
        return Err(Diagnostic::input(
            "invalid_template",
            "package roles must have distinct paths",
        ));
    }
    Ok(m)
}

pub fn package_files(m: &TemplateManifest) -> Vec<String> {
    let mut files = vec![
        "template.toml".into(),
        m.metadata_schema.clone(),
        m.defaults.clone(),
        m.sample.clone(),
    ];
    files.extend(m.resources.clone());
    files.sort();
    files
}

pub fn resolve_package(files: &BTreeMap<String, Vec<u8>>) -> Result<TemplatePackage> {
    let text = |name: &str| -> Result<&str> {
        let bytes = files.get(name).ok_or_else(|| {
            Diagnostic::new(
                "missing_dependency",
                3,
                format!("missing template file {name}"),
            )
        })?;
        std::str::from_utf8(bytes).map_err(|_| Diagnostic::input("invalid_utf8", name))
    };
    let manifest = parse_manifest(text("template.toml")?)?;
    let names = package_files(&manifest);
    if names.len() != files.len() {
        return Err(Diagnostic::input(
            "invalid_template",
            "package contains undeclared files",
        ));
    }
    let schema_text = text(&manifest.metadata_schema)?;
    if schema_text.len() > 65536 {
        return Err(Diagnostic::input("schema_budget", "schema exceeds 64 KiB"));
    }
    let schema: Value = crate::parse_json(schema_text)
        .map_err(|e| Diagnostic::input("invalid_schema", e.message))?;
    schema_validator(&schema)?;
    let defaults = parse_metadata(text(&manifest.defaults)?)?;
    let mut hash = Sha256::new();
    hash.update(b"pdx-template-v1\0");
    for name in &names {
        let bytes = files
            .get(name)
            .ok_or_else(|| Diagnostic::new("missing_dependency", 3, name))?;
        // Length framing avoids ambiguous concatenation; ordered names use UTF-8 byte order.
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    let identity = TemplateIdentity {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        digest: format!("sha256:{:x}", hash.finalize()),
    };
    Ok(TemplatePackage {
        manifest,
        identity,
        defaults,
        schema,
        files: names,
    })
}

fn schema_validator(schema: &Value) -> Result<jsonschema::Validator> {
    if schema.get("$schema").and_then(Value::as_str)
        != Some("http://json-schema.org/draft-07/schema#")
    {
        return Err(Diagnostic::input(
            "invalid_schema",
            "explicit JSON Schema draft-07 is required",
        ));
    }
    let mut budget = 4096;
    schema_budget(schema, schema, 0, &mut budget, &mut Vec::new())?;
    jsonschema::draft7::meta::validate(schema)
        .map_err(|e| Diagnostic::input("invalid_schema", e.to_string()))?;
    jsonschema::draft7::new(schema).map_err(|e| Diagnostic::input("invalid_schema", e.to_string()))
}

fn schema_budget(
    value: &Value,
    root: &Value,
    depth: usize,
    budget: &mut usize,
    refs: &mut Vec<String>,
) -> Result<()> {
    if depth > 24 || *budget == 0 {
        return Err(Diagnostic::input(
            "schema_budget",
            "schema depth/node/ref expansion budget exceeded",
        ));
    }
    *budget -= 1;
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if matches!(
                    key.as_str(),
                    "$id" | "$dynamicRef" | "$recursiveRef" | "pattern" | "patternProperties"
                ) {
                    return Err(Diagnostic::input(
                        "unsupported_schema_keyword",
                        format!("R1 schema excludes {key}"),
                    ));
                }
                if key == "$ref" {
                    let reference = value.as_str().ok_or_else(|| {
                        Diagnostic::input("invalid_schema", "$ref must be string")
                    })?;
                    if !reference.starts_with("#/") {
                        return Err(Diagnostic::input(
                            "external_schema_ref",
                            "only same-document JSON pointer refs are allowed",
                        ));
                    }
                    if refs.iter().any(|r| r == reference) {
                        return Err(Diagnostic::input(
                            "schema_budget",
                            "recursive schema refs are unsupported",
                        ));
                    }
                    let target = root.pointer(&reference[1..]).ok_or_else(|| {
                        Diagnostic::input("invalid_schema", "unresolved local $ref")
                    })?;
                    refs.push(reference.into());
                    schema_budget(target, root, depth + 1, budget, refs)?;
                    refs.pop();
                } else {
                    schema_budget(value, root, depth + 1, budget, refs)?;
                }
            }
        }
        Value::Array(values) => {
            for v in values {
                schema_budget(v, root, depth + 1, budget, refs)?;
            }
        }
        _ => {}
    }
    Ok(())
}
impl TemplatePackage {
    pub fn validate_metadata(&self, metadata: &Value) -> Result<()> {
        let validator = schema_validator(&self.schema)?;
        if let Some(error) = validator.iter_errors(metadata).next() {
            let mut d = Diagnostic::input("metadata_schema", error.to_string());
            d.instance_path = Some(error.instance_path().to_string());
            return Err(d);
        }
        Ok(())
    }
}
