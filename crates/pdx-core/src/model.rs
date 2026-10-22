use crate::{Diagnostic, Result, TemplatePackage, identity, overlaps, reject_aliases};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub default_target: String,
    pub documents: BTreeMap<String, Document>,
    pub targets: BTreeMap<String, Target>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub format: String,
    pub sources: Vec<String>,
    pub metadata: Option<String>,
    pub bibliography: Vec<String>,
    pub resources: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub document: String,
    pub template: TemplateRef,
    pub metadata: Option<String>,
    pub output_dir: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateRef {
    pub id: Option<String>,
    pub path: Option<String>,
}

pub fn parse_project(text: &str) -> Result<Project> {
    let project: Project = parse_toml(text)?;
    project.validate()?;
    Ok(project)
}
pub fn parse_toml<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > 1_048_576 {
        return Err(Diagnostic::input("parse_budget", "TOML exceeds 1 MiB"));
    }
    toml::from_str(text).map_err(|e| Diagnostic::input("invalid_toml", e.to_string()))
}
pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(Diagnostic::input(
            "invalid_id",
            format!("invalid internal ID: {id}"),
        ));
    }
    Ok(())
}
impl Project {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(Diagnostic::input(
                "unsupported_schema",
                "project schema_version must be 1",
            ));
        }
        if !self.targets.contains_key(&self.default_target) {
            return Err(Diagnostic::input(
                "unknown_target",
                "default_target does not exist",
            ));
        }
        let mut paths = vec!["pdx.toml", "pdx.lock", ".git", ".pdx"];
        let mut protected = paths.clone();
        for (id, doc) in &self.documents {
            validate_id(id)?;
            if doc.format != "markdown" {
                return Err(Diagnostic::input(
                    "unsupported_format",
                    format!("document {id}: {}", doc.format),
                ));
            }
            if doc.sources.is_empty() {
                return Err(Diagnostic::input(
                    "empty_sources",
                    format!("document {id} has no sources"),
                ));
            }
            protected.extend(
                doc.sources
                    .iter()
                    .chain(&doc.bibliography)
                    .chain(&doc.resources)
                    .map(String::as_str),
            );
            protected.extend(doc.metadata.as_deref());
        }
        for (id, target) in &self.targets {
            validate_id(id)?;
            if !self.documents.contains_key(&target.document) {
                return Err(Diagnostic::input(
                    "unknown_document",
                    format!("target {id}: {}", target.document),
                ));
            }
            match (&target.template.id, &target.template.path) {
                (Some(id), None) => validate_id(id)?,
                (None, Some(path)) => protected.push(path),
                _ => {
                    return Err(Diagnostic::input(
                        "invalid_template_ref",
                        "template requires exactly one of id/path",
                    ));
                }
            }
            protected.extend(target.metadata.as_deref());
            paths.push(&target.output_dir);
        }
        paths.extend(&protected);
        reject_aliases(paths)?;
        for (id, a) in &self.targets {
            for p in &protected {
                if overlaps(&a.output_dir, p) {
                    return Err(Diagnostic::safety(format!(
                        "output {id} overlaps protected input {p}"
                    )));
                }
            }
            for (other, b) in &self.targets {
                if id < other && overlaps(&a.output_dir, &b.output_dir) {
                    return Err(Diagnostic::safety(format!(
                        "outputs {id} and {other} overlap"
                    )));
                }
            }
        }
        // App-owned namespaces may not be repurposed as manuscript inputs.
        for p in protected.iter().skip(4) {
            if ["pdx.toml", "pdx.lock", ".git", ".pdx"]
                .iter()
                .any(|r| overlaps(p, r))
            {
                return Err(Diagnostic::safety(format!(
                    "input overlaps reserved path {p}"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplateIdentity {
    pub id: String,
    pub version: String,
    pub digest: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResolution {
    Unresolved,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyLock {
    pub schema_version: u32,
    pub templates: BTreeMap<String, TemplateIdentity>,
    pub runtime: RuntimeResolution,
}
pub fn parse_lock(text: &str) -> Result<DependencyLock> {
    let value =
        crate::parse_json(text).map_err(|e| Diagnostic::input("invalid_lock", e.message))?;
    serde_json::from_value(value).map_err(|e| Diagnostic::input("invalid_lock", e.to_string()))
}
pub fn encode_lock(lock: &DependencyLock) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(lock)
        .map_err(|e| Diagnostic::input("invalid_lock", e.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}
impl DependencyLock {
    pub fn from_packages(packages: &BTreeMap<String, TemplatePackage>) -> Self {
        Self {
            schema_version: 1,
            templates: packages
                .iter()
                .map(|(k, p)| (k.clone(), p.identity.clone()))
                .collect(),
            runtime: RuntimeResolution::Unresolved,
        }
    }
    pub fn validate(&self, packages: &BTreeMap<String, TemplatePackage>) -> Result<()> {
        if self.schema_version != 1 {
            return Err(Diagnostic::input(
                "unsupported_schema",
                "lock schema_version must be 1",
            ));
        }
        if self.templates != Self::from_packages(packages).templates {
            return Err(Diagnostic::new(
                "stale_lock",
                3,
                "template identity/digest differs; run pdx lock explicitly",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StaticStatus {
    pub static_only: bool,
    pub runtime_verified: bool,
    pub build_ready: bool,
    pub unverified: Vec<String>,
}
impl Default for StaticStatus {
    fn default() -> Self {
        Self {
            static_only: true,
            runtime_verified: false,
            build_ready: false,
            unverified: vec![
                "Docker image resolution and tool probes".into(),
                "PDF generation and typesetting conformance".into(),
            ],
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct BuildPlan {
    pub schema_version: u32,
    pub target: String,
    pub document: String,
    pub recipe: crate::Recipe,
    pub ordered_inputs: Vec<String>,
    pub bibliography: Vec<String>,
    pub resources: Vec<String>,
    pub metadata_inputs: Vec<String>,
    pub merged_metadata: Value,
    pub template: TemplateIdentity,
    pub template_source: TemplateRef,
    pub template_files: Vec<String>,
    pub output_dir: String,
    pub required_tools: Vec<String>,
    pub runtime: RuntimeResolution,
    #[serde(flatten)]
    pub status: StaticStatus,
}

pub fn build_plan(
    project: &Project,
    target_id: &str,
    template: &TemplatePackage,
    metadata: Value,
) -> Result<BuildPlan> {
    let target = project
        .targets
        .get(target_id)
        .ok_or_else(|| Diagnostic::input("unknown_target", target_id))?;
    let doc = &project.documents[&target.document];
    template.validate_metadata(&metadata)?;
    let mut tools = template.manifest.required_tools.clone();
    tools.sort_by_key(|s| identity(s));
    Ok(BuildPlan {
        schema_version: 1,
        target: target_id.into(),
        document: target.document.clone(),
        recipe: template.manifest.recipe.clone(),
        ordered_inputs: doc.sources.clone(),
        bibliography: doc.bibliography.clone(),
        resources: doc.resources.clone(),
        metadata_inputs: doc
            .metadata
            .iter()
            .chain(target.metadata.iter())
            .cloned()
            .collect(),
        merged_metadata: metadata,
        template: template.identity.clone(),
        template_source: target.template.clone(),
        template_files: template.files.clone(),
        output_dir: target.output_dir.clone(),
        required_tools: tools,
        runtime: RuntimeResolution::Unresolved,
        status: StaticStatus::default(),
    })
}
