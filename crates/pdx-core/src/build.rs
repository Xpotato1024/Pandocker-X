use crate::{BuildPlan, Diagnostic, Result, TemplateIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const BUILD_FILE_BYTES: usize = 32 * 1024 * 1024;
pub const BUILD_INPUT_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePin {
    pub image_id: String,
    pub platform: String,
}
impl RuntimePin {
    pub fn local(image_id: &str) -> Result<Self> {
        let pin = Self {
            image_id: image_id.into(),
            platform: "linux/amd64".into(),
        };
        pin.validate()?;
        Ok(pin)
    }
    pub fn validate(&self) -> Result<()> {
        let hash = self.image_id.strip_prefix("sha256:").unwrap_or("");
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || hash.bytes().all(|b| b == b'0')
            || self.platform != "linux/amd64"
        {
            return Err(Diagnostic::input(
                "invalid_runtime",
                "runtime requires a nonzero Docker image inspect .Id sha256:<64 lowercase hex> and linux/amd64; tags and registry references are refused",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputDigest {
    pub sha256: String,
    pub bytes: u64,
}
pub fn digest(bytes: &[u8]) -> InputDigest {
    InputDigest {
        sha256: format!("sha256:{:x}", Sha256::digest(bytes)),
        bytes: bytes.len() as u64,
    }
}
pub fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|e| Diagnostic::new("serialization", 5, e.to_string()))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEvidence {
    pub driver: String,
    pub runtime_verified: bool,
    pub tools: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    pub schema_version: u32,
    pub build_id: String,
    pub target: String,
    pub document: String,
    pub template: TemplateIdentity,
    pub runtime: RuntimePin,
    pub evidence: RuntimeEvidence,
    pub inputs: BTreeMap<String, InputDigest>,
    pub ordered_inputs: Vec<String>,
    pub pdf: InputDigest,
    pub pdf_verified: bool,
    pub status: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct BuildResult {
    pub schema_version: u32,
    pub build_id: String,
    pub target: String,
    pub artifact_dir: String,
    pub pdf: String,
    pub record: String,
    pub runtime_verified: bool,
    pub pdf_verified: bool,
    pub static_only: bool,
    pub unverified: Vec<String>,
}
pub fn validate_pdf_bytes(pdf: &[u8]) -> Result<()> {
    let end = pdf
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(0, |n| n + 1);
    if pdf.len() > BUILD_FILE_BYTES || !pdf.starts_with(b"%PDF-") || !pdf[..end].ends_with(b"%%EOF")
    {
        return Err(Diagnostic::new(
            "invalid_pdf",
            1,
            "new PDF is empty, oversized or lacks PDF header/end marker",
        ));
    }
    Ok(())
}
/// Fixed recipe argv only; no shell text and no host paths.
pub fn pandoc_arguments(plan: &BuildPlan) -> Vec<String> {
    let mut args = vec![
        "--from=markdown".into(),
        "--to=latex".into(),
        "--standalone".into(),
        "--output=.pdx/document.tex".into(),
        "--metadata-file=.pdx/merged-metadata.json".into(),
        "--variable=mainfont:Noto Serif CJK JP".into(),
    ];
    if plan.merged_metadata["settings"]["toc"].as_bool() == Some(true) {
        args.push("--toc".into());
    }
    if plan.merged_metadata["settings"]["numbersections"].as_bool() == Some(true) {
        args.push("--number-sections".into());
    }
    if !plan.bibliography.is_empty() {
        args.push("--citeproc".into());
    }
    args.extend(
        plan.bibliography
            .iter()
            .map(|p| format!("--bibliography=./{p}")),
    );
    args.push("--".into());
    args.extend(plan.ordered_inputs.iter().map(|p| format!("./{p}")));
    args
}
