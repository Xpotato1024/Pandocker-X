use crate::{Application, ProjectStore};
use pdx_core::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn check(&self) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            Err(Diagnostic::new(
                "cancelled",
                130,
                "build cancelled; previous generations preserved",
            ))
        } else {
            Ok(())
        }
    }
}
pub struct BuildSettings {
    pub state_root: PathBuf,
    pub timeout: Duration,
}
pub struct RenderRequest<'a> {
    pub build_id: &'a str,
    pub input_dir: &'a Path,
    pub plan: &'a BuildPlan,
    pub pin: &'a RuntimePin,
    pub timeout: Duration,
}
pub struct RenderedPdf {
    pub bytes: Vec<u8>,
    pub pdf_verified: bool,
}
pub trait BuildRuntime {
    fn probe(
        &self,
        pin: &RuntimePin,
        cancel: &Cancellation,
        timeout: Duration,
    ) -> Result<RuntimeEvidence>;
    fn render(&self, request: RenderRequest<'_>, cancel: &Cancellation) -> Result<RenderedPdf>;
}
pub trait BuildWorkspace {
    fn id(&self) -> &str;
    fn input_dir(&self) -> &Path;
    fn snapshot(
        &mut self,
        plan: &BuildPlan,
        project: &[u8],
        lock: &[u8],
        cancel: &Cancellation,
    ) -> Result<BTreeMap<String, InputDigest>>;
    fn publish(
        &mut self,
        record: &BuildRecord,
        pdf: &[u8],
        cancel: &Cancellation,
    ) -> Result<BuildResult>;
    fn finish(&mut self, error: Option<&Diagnostic>) -> Result<()>;
}
pub trait BuildStore: ProjectStore {
    type Workspace: BuildWorkspace;
    fn begin(
        &self,
        root: &Path,
        plan: &BuildPlan,
        settings: &BuildSettings,
    ) -> Result<Self::Workspace>;
}
impl<S: ProjectStore> Application<S> {
    pub fn pin_runtime<R: BuildRuntime>(
        &self,
        path: Option<&Path>,
        image_id: &str,
        runtime: &R,
        cancel: &Cancellation,
    ) -> Result<RuntimeEvidence> {
        cancel.check()?;
        let pin = RuntimePin::local(image_id)?;
        let (checked, mut lock) = self.load(path, false)?;
        let evidence = runtime.probe(&pin, cancel, Duration::from_secs(30))?;
        cancel.check()?;
        lock.schema_version = 2;
        lock.runtime = RuntimeResolution::Docker(pin);
        self.store.save_lock(&checked.root, &encode_lock(&lock)?)?;
        Ok(evidence)
    }
}
impl<S: BuildStore> Application<S> {
    pub fn build<R: BuildRuntime>(
        &self,
        path: Option<&Path>,
        target: Option<&str>,
        settings: &BuildSettings,
        runtime: &R,
        cancel: &Cancellation,
    ) -> Result<BuildResult> {
        cancel.check()?;
        if settings.timeout.is_zero() || settings.timeout > Duration::from_secs(600) {
            return Err(Diagnostic::input(
                "invalid_arguments",
                "timeout must be 1..600 seconds",
            ));
        }
        let checked = self.check(path)?;
        let id = target.unwrap_or(&checked.default_target);
        let plan = checked
            .plans
            .get(id)
            .ok_or_else(|| Diagnostic::input("unknown_target", id))?;
        let pin = match &checked.lock.runtime {
            RuntimeResolution::Docker(pin) => pin,
            RuntimeResolution::Unresolved => {
                return Err(Diagnostic::new(
                    "missing_runtime",
                    3,
                    "runtime unresolved; explicitly pin a prepared local image with pdx lock --runtime-image",
                ));
            }
        };
        let started = Instant::now();
        let evidence = runtime.probe(pin, cancel, settings.timeout.min(Duration::from_secs(30)))?;
        let mut workspace = self.store.begin(&checked.root, plan, settings)?;
        let operation = (|| {
            cancel.check()?;
            let selected = checked.project.targets[id].clone();
            let project = Project {
                schema_version: 1,
                default_target: id.into(),
                documents: BTreeMap::from([(
                    selected.document.clone(),
                    checked.project.documents[&selected.document].clone(),
                )]),
                targets: BTreeMap::from([(id.into(), selected)]),
            };
            let config = encode_project(&project)?;
            let lock = DependencyLock {
                schema_version: checked.lock.schema_version,
                templates: BTreeMap::from([(id.into(), checked.lock.templates[id].clone())]),
                runtime: checked.lock.runtime.clone(),
            };
            let inputs =
                workspace.snapshot(plan, config.as_bytes(), &encode_lock(&lock)?, cancel)?;
            let snapshot_plan = self.plan(Some(workspace.input_dir()), Some(id))?;
            if json_bytes(&snapshot_plan)? != json_bytes(plan)?
                || json_bytes(&self.plan(Some(&checked.root), Some(id))?)? != json_bytes(plan)?
            {
                return Err(Diagnostic::input(
                    "input_changed",
                    "configuration/metadata/template changed during snapshot",
                ));
            }
            let timeout = settings
                .timeout
                .checked_sub(started.elapsed())
                .filter(|d| !d.is_zero())
                .ok_or_else(|| {
                    Diagnostic::new(
                        "build_timeout",
                        1,
                        "build deadline exceeded before execution",
                    )
                })?;
            let pdf = runtime.render(
                RenderRequest {
                    build_id: workspace.id(),
                    input_dir: workspace.input_dir(),
                    plan: &snapshot_plan,
                    pin,
                    timeout,
                },
                cancel,
            )?;
            cancel.check()?;
            validate_pdf_bytes(&pdf.bytes)?;
            let record = BuildRecord {
                schema_version: 1,
                build_id: workspace.id().into(),
                target: id.into(),
                document: plan.document.clone(),
                template: plan.template.clone(),
                runtime: pin.clone(),
                evidence,
                inputs,
                ordered_inputs: plan.ordered_inputs.clone(),
                pdf: digest(&pdf.bytes),
                pdf_verified: pdf.pdf_verified,
                status: "committed".into(),
            };
            workspace.publish(&record, &pdf.bytes, cancel)
        })();
        let cleanup = workspace.finish(operation.as_ref().err());
        match (operation, cleanup) {
            (result, Ok(())) => result,
            (Ok(result), Err(error)) => Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!(
                    "generation committed at {}; {}",
                    result.artifact_dir, error.message
                ),
            )),
            (Err(original), Err(error)) => Err(Diagnostic::new(
                "cleanup_failed",
                5,
                format!("{}; cleanup: {}", original, error.message),
            )),
        }
    }
}
