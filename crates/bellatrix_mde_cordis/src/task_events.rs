//! Generation and export consume a committed bellatrix-project revision; output is an emission.
use crate::project_integration::from_document;
use bellatrix_mde_modeling::{Scope, publish_new, publish_replace};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::event::responder;
use cordis::{Context, Event, Routing};
use std::{convert::Infallible, io, path::PathBuf, sync::Arc};

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
    const NAME: &'static str = "bellatrix.bellatrix-mde-modeling.run.v1";
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
    const NAME: &'static str = "bellatrix.bellatrix-mde-modeling.task-completed.v1";
    type Args = ModelingTaskCompleted;
    type Output = ();
}

pub fn register(ctx: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let _listener = ctx
        .on::<ModelingRun, _>(responder(
            move |ctx: Context, request: ModelingRunRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let project_id = request.project_id.clone();
                    let correlation_id = request.correlation_id;
                    let outcome = execute(projects, request).await;
                    if let Err(error) = ctx
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
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

async fn execute(
    projects: Arc<ProjectService>,
    request: ModelingRunRequested,
) -> Result<ModelingTaskSummary, String> {
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|e| e.to_string())?;
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
                )
            }
            ModelingTaskKind::ExportHtml => bellatrix_mde_html_exporter::plan(&model),
        }
        .map_err(|e| e.to_string())?;
        let count = plan.count();
        let destination = request.destination;
        if request.replace_generated {
            publish_replace(&plan, &destination).map_err(|e| e.to_string())?;
        } else {
            publish_new(&plan, &destination).map_err(|e| e.to_string())?;
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
    .map_err(|e| e.to_string())?
}
