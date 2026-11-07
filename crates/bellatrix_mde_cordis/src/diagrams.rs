//! Diagram lifecycle requests share the same revision-checked mutation boundary as element edits.
use crate::Command;
use crate::{TraceOutcome, TracePhase};
use bellatrix_mde::MdeExecutionTrace as ExecutionTrace;
pub use bellatrix_mde::contracts::events::{
    DiagramValidationResult, ModelingCreateDiagram, ModelingCreateDiagramRequested,
    ModelingCreateExplorerDiagram, ModelingCreateExplorerDiagramRequested, ModelingDeleteDiagram,
    ModelingDeleteDiagramRequested, ModelingExecutionTrace, ModelingExecutionTraceEvent,
    ModelingRenameDiagram, ModelingRenameDiagramRequested, ModelingValidateDiagram,
    ModelingValidateDiagramRequested, ModelingValidateDiagramTraced,
    ModelingValidateDiagramTracedRequested, ModelingValidateSnapshot,
    ModelingValidateSnapshotRequested, TracedDiagramValidationResult,
};
use crate::project::{ProjectId, ProjectService};
use cordis::Context;
use cordis::event::responder;
use std::{
    convert::Infallible,
    io,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_VALIDATION_RUN: AtomicU64 = AtomicU64::new(1);

fn validate_inner(
    projects: &ProjectService,
    request: ModelingValidateDiagramRequested,
    trace: Option<&mut ExecutionTrace>,
) -> Result<DiagramValidationResult, String> {
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision {
        return Err(format!(
            "validation revision changed: requested {}, current {}",
            request.expected_revision,
            document.revision()
        ));
    }
    let section_json = document
        .section(crate::MODELING_NAMESPACE)
        .ok_or("MDE section is missing")?
        .to_string();
    let diagnostics = bellatrix_mde::validate_model_section_diagram(
        document.id().as_str(),
        document.name(),
        &section_json,
        &request.diagram_id,
        trace,
    )?;
    Ok(DiagramValidationResult {
        diagram_id: request.diagram_id,
        revision: document.revision(),
        diagnostics,
    })
}

pub(crate) fn create_diagram_command(
    request: ModelingCreateDiagramRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::CreateDiagram {
            kind: request.kind,
            name: request.name,
            model_elements: request.model_elements,
        },
    )
}

pub(crate) fn create_explorer_diagram_command(
    request: ModelingCreateExplorerDiagramRequested,
) -> (ProjectId, u64, Command) {
    (
        request.diagram.project_id,
        request.diagram.expected_revision,
        Command::CreateExplorerDiagram {
            kind: request.diagram.kind,
            name: request.diagram.name,
            model_elements: request.diagram.model_elements,
            parent_id: request.parent_id,
            parent_occurrence: request.parent_occurrence,
        },
    )
}

pub(crate) fn rename_diagram_command(
    request: ModelingRenameDiagramRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::RenameDiagram {
            diagram_id: request.diagram_id,
            name: request.name,
        },
    )
}

pub(crate) fn delete_diagram_command(
    request: ModelingDeleteDiagramRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::DeleteDiagram {
            diagram_id: request.diagram_id,
        },
    )
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let snapshot_projects = Arc::clone(&projects);
    let _validate_snapshot = context
        .on::<ModelingValidateSnapshot, _>(responder(
            move |_: Context, request: ModelingValidateSnapshotRequested| {
                let projects = Arc::clone(&snapshot_projects);
                async move {
                    let result = tokio::task::spawn_blocking(move || {
                        let current = projects
                            .snapshot(request.document.id())
                            .map_err(|error| error.to_string())?;
                        if !Arc::ptr_eq(&current, &request.document) {
                            return Err(
                                "Validation source is no longer the active project snapshot".into(),
                            );
                        }
                        let result = validate_inner(
                            &projects,
                            ModelingValidateDiagramRequested {
                                project_id: request.document.id().clone(),
                                diagram_id: request.diagram_id,
                                expected_revision: request.document.revision(),
                            },
                            None,
                        );
                        let current = projects
                            .snapshot(request.document.id())
                            .map_err(|error| error.to_string())?;
                        if !Arc::ptr_eq(&current, &request.document) {
                            return Err("Project changed while validating its snapshot".into());
                        }
                        result
                    })
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result);
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let create_projects = Arc::clone(&projects);
    let _create = context
        .on::<ModelingCreateDiagram, _>(responder(
            move |context: Context, request: ModelingCreateDiagramRequested| {
                let projects = Arc::clone(&create_projects);
                async move {
                    let (project_id, expected_revision, command) = create_diagram_command(request);
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        project_id,
                        expected_revision,
                        command,
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let explorer_projects = Arc::clone(&projects);
    let _create_explorer = context
        .on::<ModelingCreateExplorerDiagram, _>(responder(
            move |context: Context, request: ModelingCreateExplorerDiagramRequested| {
                let projects = Arc::clone(&explorer_projects);
                async move {
                    let (project_id, expected_revision, command) =
                        create_explorer_diagram_command(request);
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        project_id,
                        expected_revision,
                        command,
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let rename_projects = Arc::clone(&projects);
    let _rename = context
        .on::<ModelingRenameDiagram, _>(responder(
            move |context: Context, request: ModelingRenameDiagramRequested| {
                let projects = Arc::clone(&rename_projects);
                async move {
                    let (project_id, expected_revision, command) = rename_diagram_command(request);
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        project_id,
                        expected_revision,
                        command,
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let delete_projects = Arc::clone(&projects);
    let _delete = context
        .on::<ModelingDeleteDiagram, _>(responder(
            move |context: Context, request: ModelingDeleteDiagramRequested| {
                let projects = Arc::clone(&delete_projects);
                async move {
                    let (project_id, expected_revision, command) = delete_diagram_command(request);
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        project_id,
                        expected_revision,
                        command,
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let traced_projects = Arc::clone(&projects);
    let _validate_traced = context
        .on::<ModelingValidateDiagramTraced, _>(responder(
            move |_context: Context, traced: ModelingValidateDiagramTracedRequested| {
                let projects = Arc::clone(&traced_projects);
                async move {
                    let mut trace = ExecutionTrace::new(
                        traced.run_id,
                        Some(traced.request.expected_revision.to_string()),
                        traced.capacity.min(10_000),
                    );
                    trace.emit(
                        "cordis",
                        "diagram.validate.request",
                        TracePhase::Start,
                        Some(&traced.request.diagram_id),
                        &[],
                        None,
                    );
                    let result = validate_inner(&projects, traced.request, Some(&mut trace));
                    if let Err(error) = &result {
                        trace.diagnostic(
                            "cordis",
                            "diagram.validate.request",
                            None,
                            error,
                            TraceOutcome::Error,
                        );
                    }
                    let (phase, outcome) = if result.is_ok() {
                        (TracePhase::Finish, TraceOutcome::Success)
                    } else {
                        (TracePhase::Error, TraceOutcome::Error)
                    };
                    trace.emit(
                        "cordis",
                        "diagram.validate.request",
                        phase,
                        None,
                        &[],
                        Some(outcome),
                    );
                    Ok::<_, Infallible>(Some(TracedDiagramValidationResult {
                        result,
                        events: trace.events(),
                        dropped: trace.dropped(),
                    }))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let _validate = context
        .on::<ModelingValidateDiagram, _>(responder(
            move |context: Context, request: ModelingValidateDiagramRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let run_id = format!(
                        "diagram-validation-{}-{}",
                        std::process::id(),
                        NEXT_VALIDATION_RUN.fetch_add(1, Ordering::Relaxed)
                    );
                    let mut trace = ExecutionTrace::new(
                        run_id,
                        Some(request.expected_revision.to_string()),
                        10_000,
                    );
                    let forwarder = crate::trace_output::start(
                        context.clone(),
                        &mut trace,
                        request.project_id.clone(),
                    );
                    trace.emit(
                        "cordis",
                        "diagram.validate.request",
                        TracePhase::Start,
                        Some(&request.diagram_id),
                        &[],
                        None,
                    );
                    let fallback = trace.clone();
                    let (mut trace, result) = match tokio::task::spawn_blocking(move || {
                        let result = validate_inner(&projects, request, Some(&mut trace));
                        (trace, result)
                    })
                    .await
                    {
                        Ok(value) => value,
                        Err(error) => (fallback.clone(), Err(error.to_string())),
                    };
                    drop(fallback);
                    if let Err(error) = &result {
                        trace.diagnostic(
                            "cordis",
                            "diagram.validate.request",
                            None,
                            error,
                            TraceOutcome::Error,
                        );
                    }
                    let (phase, outcome) = if result.is_ok() {
                        (TracePhase::Finish, TraceOutcome::Success)
                    } else {
                        (TracePhase::Error, TraceOutcome::Error)
                    };
                    trace.emit(
                        "cordis",
                        "diagram.validate.request",
                        phase,
                        None,
                        &[],
                        Some(outcome),
                    );
                    trace.clear_sink();
                    drop(forwarder);
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}
