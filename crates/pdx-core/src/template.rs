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
    schema_budget(
        schema,
        schema,
        SchemaPosition::Schema,
        0,
        &mut budget,
        &mut Vec::new(),
    )?;
    jsonschema::draft7::meta::validate(schema)
        .map_err(|e| Diagnostic::input("invalid_schema", e.to_string()))?;
    jsonschema::draft7::new(schema).map_err(|e| Diagnostic::input("invalid_schema", e.to_string()))
}

#[derive(Clone, Copy)]
enum SchemaPosition {
    Schema,
    SchemaMap,
    SchemaArray,
    Dependencies,
    Data,
}

fn schema_budget(
    value: &Value,
    root: &Value,
    position: SchemaPosition,
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
                let is_schema = matches!(position, SchemaPosition::Schema);
                if is_schema
                    && key == "$schema"
                    && value.as_str() != Some("http://json-schema.org/draft-07/schema#")
                {
                    return Err(Diagnostic::input(
                        "unsupported_schema",
                        "R1 requires draft-07 in every schema position",
                    ));
                }
                if is_schema
                    && matches!(
                        key.as_str(),
                        "$id" | "$dynamicRef" | "$recursiveRef" | "pattern" | "patternProperties"
                    )
                {
                    return Err(Diagnostic::input(
                        "unsupported_schema_keyword",
                        format!("R1 schema excludes {key}"),
                    ));
                }
                if is_schema && key == "$ref" {
                    let reference = value.as_str().ok_or_else(|| {
                        Diagnostic::input("invalid_schema", "$ref must be string")
                    })?;
                    if !reference.starts_with("#/") {
                        return Err(Diagnostic::input(
                            "external_schema_ref",
                            "only same-document JSON pointer refs are allowed",
                        ));
                    }
                    // Match the standard resolver's URI-fragment decoding before
                    // JSON Pointer traversal. Compare decoded refs for cycles too.
                    let pointer = percent_encoding::percent_decode_str(&reference[1..])
                        .decode_utf8()
                        .map_err(|_| Diagnostic::input("invalid_schema", "non-UTF8 local $ref"))?;
                    if refs.iter().any(|r| r == pointer.as_ref()) {
                        return Err(Diagnostic::input(
                            "schema_budget",
                            "recursive schema refs are unsupported",
                        ));
                    }
                    let target = root.pointer(&pointer).ok_or_else(|| {
                        Diagnostic::input("invalid_schema", "unresolved local $ref")
                    })?;
                    refs.push(pointer.into_owned());
                    schema_budget(
                        target,
                        root,
                        SchemaPosition::Schema,
                        depth + 1,
                        budget,
                        refs,
                    )?;
                    refs.pop();
                }
                // Only draft-07 schema-bearing keywords change position. Names in
                // properties/definitions/dependencies and const/enum/default data
                // are not keywords. Count all data, including the $ref string.
                let child = match position {
                    SchemaPosition::SchemaMap => SchemaPosition::Schema,
                    SchemaPosition::Dependencies if !value.is_array() => SchemaPosition::Schema,
                    SchemaPosition::Schema => match key.as_str() {
                        "properties" | "definitions" => SchemaPosition::SchemaMap,
                        "dependencies" => SchemaPosition::Dependencies,
                        "allOf" | "anyOf" | "oneOf" => SchemaPosition::SchemaArray,
                        "items" if value.is_array() => SchemaPosition::SchemaArray,
                        "items"
                        | "additionalItems"
                        | "additionalProperties"
                        | "contains"
                        | "propertyNames"
                        | "not"
                        | "if"
                        | "then"
                        | "else" => SchemaPosition::Schema,
                        _ => SchemaPosition::Data,
                    },
                    _ => SchemaPosition::Data,
                };
                schema_budget(value, root, child, depth + 1, budget, refs)?;
            }
        }
        Value::Array(values) => {
            for v in values {
                let child = if matches!(position, SchemaPosition::SchemaArray) {
                    SchemaPosition::Schema
                } else {
                    SchemaPosition::Data
                };
                schema_budget(v, root, child, depth + 1, budget, refs)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn restricted_keywords_are_rejected_in_every_schema_position() {
        for (leaf, code) in [
            (json!({"pattern": ".*"}), "unsupported_schema_keyword"),
            (
                json!({"patternProperties": {}}),
                "unsupported_schema_keyword",
            ),
            (json!({"$id": "local"}), "unsupported_schema_keyword"),
            (json!({"$dynamicRef": "#x"}), "unsupported_schema_keyword"),
            (json!({"$recursiveRef": "#"}), "unsupported_schema_keyword"),
            (
                json!({"$ref": "https://example.invalid/schema"}),
                "external_schema_ref",
            ),
        ] {
            for mut schema in [
                leaf.clone(),
                json!({"properties": {"pattern": leaf}}),
                json!({"definitions": {"$ref": leaf}}),
                json!({"dependencies": {"pattern": leaf}}),
                json!({"items": leaf}),
                json!({"items": [leaf]}),
                json!({"additionalItems": leaf}),
                json!({"additionalProperties": leaf}),
                json!({"propertyNames": leaf}),
                json!({"contains": leaf}),
                json!({"allOf": [leaf]}),
                json!({"anyOf": [leaf]}),
                json!({"oneOf": [leaf]}),
                json!({"not": leaf}),
                json!({"if": leaf}),
                json!({"then": leaf}),
                json!({"else": leaf}),
                // A ref may turn an otherwise ignored data position into a schema.
                json!({"x-data": leaf, "$ref": "#/x-data"}),
            ] {
                schema["$schema"] = json!("http://json-schema.org/draft-07/schema#");
                let error = schema_validator(&schema).expect_err("restricted schema");
                assert_eq!(error.code, code, "{schema}");
            }
        }
    }

    #[test]
    fn literal_data_still_counts_towards_schema_budget() {
        let mut data = json!(null);
        for _ in 0..25 {
            data = json!({"nested": data});
        }
        for data in [data, json!(vec![0; 4096])] {
            let schema = json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "default": data
            });
            assert_eq!(
                schema_validator(&schema).err().unwrap().code,
                "schema_budget"
            );
        }
    }
}
