//! CLI/GUI共通の静的use caseと実在するFS port。
use pdx_core::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
mod build;
pub use build::*;

pub trait ProjectStore {
    fn locate(&self, explicit: Option<&Path>) -> Result<PathBuf>;
    fn read(&self, root: &Path, relative: &str, limit: usize) -> Result<Vec<u8>>;
    fn input(&self, root: &Path, relative: &str, directory_allowed: bool) -> Result<()>;
    fn output(&self, root: &Path, relative: &str) -> Result<()>;
    fn template(&self, root: &Path, reference: &TemplateRef) -> Result<TemplatePackage>;
    fn initialize(
        &self,
        path: &Path,
        files: &BTreeMap<String, Vec<u8>>,
        directories: &[&str],
    ) -> Result<PathBuf>;
    fn save_lock(&self, root: &Path, bytes: &[u8]) -> Result<()>;
}

pub struct Application<S> {
    store: S,
}
pub struct CheckedProject {
    pub root: PathBuf,
    pub default_target: String,
    pub plans: BTreeMap<String, BuildPlan>,
    pub project: Project,
    pub lock: DependencyLock,
}
impl<S: ProjectStore> Application<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
    fn load(
        &self,
        path: Option<&Path>,
        require_lock: bool,
    ) -> Result<(CheckedProject, DependencyLock)> {
        let root = self.store.locate(path)?;
        let project = parse_project(&self.text(&root, "pdx.toml", 1_048_576)?)?;
        let mut packages = BTreeMap::new();
        let mut plans = BTreeMap::new();
        // Check every document, including currently unreferenced documents.
        for doc in project.documents.values() {
            for p in doc.sources.iter().chain(&doc.bibliography) {
                self.store.input(&root, p, false)?;
            }
            for p in &doc.resources {
                self.store.input(&root, p, true)?;
            }
            if let Some(p) = &doc.metadata {
                parse_metadata(&self.text(&root, p, METADATA_BYTES)?)?;
            }
        }
        for (id, target) in &project.targets {
            self.store.output(&root, &target.output_dir)?;
            let package = self.store.template(&root, &target.template)?;
            let doc = &project.documents[&target.document];
            let mut metadata = package.defaults.clone();
            for p in doc.metadata.iter().chain(target.metadata.iter()) {
                merge_metadata(
                    &mut metadata,
                    parse_metadata(&self.text(&root, p, METADATA_BYTES)?)?,
                );
            }
            let plan = build_plan(&project, id, &package, metadata)?;
            packages.insert(id.clone(), package);
            plans.insert(id.clone(), plan);
        }
        let expected = DependencyLock::from_packages(&packages);
        let actual = if require_lock {
            let lock = parse_lock(&self.text(&root, "pdx.lock", 1_048_576)?)?;
            lock.validate(&packages)?;
            lock
        } else {
            expected.clone()
        };
        for plan in plans.values_mut() {
            plan.runtime = actual.runtime.clone();
            if matches!(actual.runtime, RuntimeResolution::Docker(_)) {
                plan.schema_version = 2;
            }
        }
        let checked = CheckedProject {
            root,
            plans,
            default_target: project.default_target.clone(),
            project,
            lock: actual,
        };
        Ok((checked, expected))
    }
    fn text(&self, root: &Path, path: &str, limit: usize) -> Result<String> {
        String::from_utf8(self.store.read(root, path, limit)?)
            .map_err(|_| Diagnostic::input("invalid_utf8", path))
    }
    pub fn check(&self, path: Option<&Path>) -> Result<CheckedProject> {
        Ok(self.load(path, true)?.0)
    }
    pub fn plan(&self, path: Option<&Path>, target: Option<&str>) -> Result<BuildPlan> {
        let checked = self.check(path)?;
        checked
            .plans
            .get(target.unwrap_or(&checked.default_target))
            .cloned()
            .ok_or_else(|| Diagnostic::input("unknown_target", target.unwrap_or("")))
    }
    pub fn lock(&self, path: Option<&Path>) -> Result<CheckedProject> {
        let (checked, mut lock) = self.load(path, false)?;
        if let Ok(text) = self.text(&checked.root, "pdx.lock", 1_048_576)
            && let Ok(previous) = parse_lock(&text)
            && let RuntimeResolution::Docker(pin) = previous.runtime
        {
            pin.validate()?;
            lock.schema_version = 2;
            lock.runtime = RuntimeResolution::Docker(pin);
        }
        self.store.save_lock(&checked.root, &encode_lock(&lock)?)?;
        Ok(checked)
    }
    pub fn init(&self, path: &Path, template: &str) -> Result<PathBuf> {
        // R1 init only supports the embedded starter. Local packages are selected in pdx.toml.
        let reference = TemplateRef {
            id: Some(template.into()),
            path: None,
        };
        let package = self.store.template(path, &reference)?;
        let project = parse_project(STARTER_CONFIG)?;
        let mut metadata = package.defaults.clone();
        merge_metadata(&mut metadata, parse_metadata(STARTER_METADATA)?);
        build_plan(&project, "report", &package, metadata)?;
        let lock = DependencyLock::from_packages(&BTreeMap::from([("report".into(), package)]));
        let files = BTreeMap::from([
            ("pdx.toml".into(), STARTER_CONFIG.as_bytes().to_vec()),
            ("pdx.lock".into(), encode_lock(&lock)?),
            (
                "content/report.md".into(),
                include_bytes!("../../../templates/default-report/sample.md").to_vec(),
            ),
            (
                "metadata/common.yaml".into(),
                STARTER_METADATA.as_bytes().to_vec(),
            ),
            (
                "references.bib".into(),
                b"% Add bibliography entries here.\n".to_vec(),
            ),
            (".gitignore".into(), b"/output/\n/.pdx/\n".to_vec()),
        ]);
        self.store
            .initialize(path, &files, &["content", "metadata", "figures"])
    }
}

pub const STARTER_METADATA: &str = "title: 新しいレポート\nauthor:\n  - 著者名\n";
pub const STARTER_CONFIG: &str = r#"schema_version = 1
default_target = "report"

[documents.report]
format = "markdown"
sources = ["content/report.md"]
metadata = "metadata/common.yaml"
bibliography = ["references.bib"]
resources = ["figures"]

[targets.report]
document = "report"
template = { id = "default-report" }
output_dir = "output/report"
"#;
