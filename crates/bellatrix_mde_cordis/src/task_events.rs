//! Tasks consume a committed project revision; generated artifacts are emissions.
use crate::{TraceOutcome, TracePhase};
use bellatrix_mde::MdeExecutionTrace as ExecutionTrace;
pub use bellatrix_mde::contracts::events::{
    ModelingRun, ModelingRunRequested, ModelingTaskCompleted, ModelingTaskCompletedEvent,
    ModelingTaskKind, ModelingTaskSummary,
};
use crate::project::ProjectService;
use cordis::event::responder;
use cordis::{Context, Routing};
use std::{
    convert::Infallible,
    io,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_TASK_RUN: AtomicU64 = AtomicU64::new(1);

pub fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let _listener = context
        .on::<ModelingRun, _>(responder(
            move |context: Context, request: ModelingRunRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let project_id = request.project_id.clone();
                    let correlation_id = request.correlation_id;
                    let mut trace = ExecutionTrace::new(
                        format!(
                            "task-{}-{correlation_id}-{}",
                            std::process::id(),
                            NEXT_TASK_RUN.fetch_add(1, Ordering::Relaxed)
                        ),
                        Some(request.expected_revision.to_string()),
                        10_000,
                    );
                    let action = match &request.task {
                        ModelingTaskKind::GenerateRust { .. } => "generate_rust",
                        ModelingTaskKind::ExportHtml => "export_html",
                    };
                    let forwarder = crate::trace_output::start(
                        context.clone(),
                        &mut trace,
                        request.project_id.clone(),
                    );
                    trace.emit("cordis", action, TracePhase::Start, None, &[], None);
                    let outcome = execute(projects, request).await;
                    if let Err(error) = &outcome {
                        trace.diagnostic("cordis", action, None, error, TraceOutcome::Error);
                    }
                    let (phase, status) = if outcome.is_ok() {
                        (TracePhase::Finish, TraceOutcome::Success)
                    } else {
                        (TracePhase::Error, TraceOutcome::Error)
                    };
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
    let revision = document.revision();
    tokio::task::spawn_blocking(move || {
        let section = document
            .section(crate::MODELING_NAMESPACE)
            .ok_or("modeling section unavailable")?;
        let bytes = bellatrix_mde::encode_model_section(
            document.id().as_str(),
            document.name(),
            &section.to_string(),
            "json",
        )?;
        let model = bellatrix_mde::decode_model_document(&bytes, "json")?;
        let prepared = match request.task {
            ModelingTaskKind::GenerateRust {
                element_ids,
                skeleton,
            } => crate::rust_generation::prepare(&model, element_ids, skeleton),
            ModelingTaskKind::ExportHtml => crate::html_export::prepare(&model),
        }?;
        let count = prepared.artifact_count;
        let destination = request.destination;
        prepared.publish(&destination, request.replace_generated)?;
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
