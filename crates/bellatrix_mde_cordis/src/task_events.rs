//! Tasks consume a committed project revision; generated artifacts are emissions.
use crate::project_integration::from_document;
use bellatrix_mde::{ExecutionTrace, TraceOutcome, TracePhase, Scope, publish_new, publish_replace};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::event::responder;
use cordis::{Context, Event, Routing};
use std::{convert::Infallible, io, path::PathBuf, sync::{Arc, atomic::{AtomicU64, Ordering}}};

static NEXT_TASK_RUN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub enum ModelingTaskKind {
    GenerateRust { scope: Scope, skeleton: bool },
    ExportHtml,
}
#[derive(Clone, Debug)]
pub struct ModelingRunRequested {
    pub project_id: ProjectId,
    pub correlation_id: u64,
    pub expected_revision: u64,
    pub destination: PathBuf,
    pub replace_generated: bool,
    pub task: ModelingTaskKind,
}
pub struct ModelingRun;
impl Event for ModelingRun {
    const NAME: &'static str = "bellatrix.mde.run.v1";
    type Args = ModelingRunRequested;
    type Output = Result<ModelingTaskSummary, String>;
}
#[derive(Clone, Debug)]
pub struct ModelingTaskSummary {
    pub project_id: ProjectId,
    pub correlation_id: u64,
    pub revision: u64,
    pub artifact_count: usize,
    pub destination: PathBuf,
}
#[derive(Clone, Debug)]
pub struct ModelingTaskCompleted {
    pub project_id: ProjectId,
    pub correlation_id: u64,
    pub outcome: Result<ModelingTaskSummary, String>,
}
pub struct ModelingTaskCompletedEvent;
impl Event for ModelingTaskCompletedEvent {
    const NAME: &'static str = "bellatrix.mde.task-completed.v1";
    type Args = ModelingTaskCompleted;
    type Output = ();
}
pub fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let _listener = context
        .on::<ModelingRun, _>(responder(
            move |context: Context, request: ModelingRunRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let project_id = request.project_id.clone();
                    let correlation_id = request.correlation_id;
                    let mut trace = ExecutionTrace::new(format!("task-{}-{correlation_id}-{}", std::process::id(), NEXT_TASK_RUN.fetch_add(1, Ordering::Relaxed)), Some(request.expected_revision.to_string()), 10_000);
                    let action = match &request.task {
                        ModelingTaskKind::GenerateRust { .. } => "generate_rust",
                        ModelingTaskKind::ExportHtml => "export_html",
                    };
                    let forwarder = crate::trace_output::start(context.clone(), &mut trace, request.project_id.clone());
                    trace.emit("cordis", action, TracePhase::Start, None, &[], None);
                    let outcome = execute(projects, request).await;
                    let (phase, status) = if outcome.is_ok() { (TracePhase::Finish, TraceOutcome::Success) } else { (TracePhase::Error, TraceOutcome::Error) };
                    trace.emit("cordis", action, phase, None, &[], Some(status));
                    trace.clear_sink();
                    drop(forwarder);
                    if let Err(error) = context
                        .emit_parallel::<ModelingTaskCompletedEvent>(
                            Routing::Unscoped,
                            ModelingTaskCompleted {
                                project_id,
                                correlation_id,
                                outcome: outcome.clone(),
                            },
                        )
                        .await
                    {
                        eprintln!("Bellatrix task observer failure: {error}");
                    }
                    Ok::<_, Infallible>(Some(outcome))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}
async fn execute(
    projects: Arc<ProjectService>,
    request: ModelingRunRequested,
) -> Result<ModelingTaskSummary, String> {
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision {
        return Err(format!(
            "stale bellatrix-project revision: current is {}",
            document.revision()
        ));
    }
    let section = from_document(&document)?;
    let model = section.to_model(document.id().as_str(), document.name());
    let revision = document.revision();
    tokio::task::spawn_blocking(move || {
        let plan = match request.task {
            ModelingTaskKind::GenerateRust { scope, skeleton } => {
                bellatrix_mde_rust_generator::plan_with_options(
                    &model,
                    &scope,
                    bellatrix_mde_rust_generator::GenerationOptions { skeleton },
                ).map_err(|error| error.to_string())
            }
            ModelingTaskKind::ExportHtml => crate::html_export::plan(&model),
        }?;
        let count = plan.count();
        let destination = request.destination;
        if request.replace_generated {
            publish_replace(&plan, &destination).map_err(|error| error.to_string())?;
        } else {
            publish_new(&plan, &destination).map_err(|error| error.to_string())?;
        }
        Ok(ModelingTaskSummary {
            project_id: request.project_id,
            correlation_id: request.correlation_id,
            revision,
            artifact_count: count,
            destination,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}
