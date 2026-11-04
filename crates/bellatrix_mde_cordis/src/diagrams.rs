//! Diagram lifecycle requests share the same revision-checked mutation boundary as element edits.
use crate::Command;
use crate::{ExecutionEvent, TracePhase, TraceOutcome, ModelingDiagnostic, StructuralDiagramKind};
use bellatrix_mde::{
    MdeExecutionTrace as ExecutionTrace,
};
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use cordis::event::responder;
use cordis::{Context, Event};
use std::{convert::Infallible, io, sync::{Arc, atomic::{AtomicU64, Ordering}}};

#[derive(Clone, Debug)]
pub struct ModelingCreateDiagramRequested {
    pub project_id: ProjectId,
    pub kind: StructuralDiagramKind,
    pub name: String,
    pub model_elements: Vec<String>,
    pub expected_revision: u64,
}
pub struct ModelingCreateDiagram;
impl Event for ModelingCreateDiagram {
    const NAME: &'static str = "bellatrix.mde.create-diagram.v1";
    type Args = ModelingCreateDiagramRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingCreateExplorerDiagramRequested {
    pub diagram: ModelingCreateDiagramRequested,
    pub parent_id: Option<String>,
    pub parent_occurrence: Option<crate::ViewOccurrence>,
}
pub struct ModelingCreateExplorerDiagram;
impl Event for ModelingCreateExplorerDiagram {
    const NAME: &'static str = "bellatrix.mde.create-explorer-diagram.v1";
    type Args = ModelingCreateExplorerDiagramRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingRenameDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub name: String,
    pub expected_revision: u64,
}
pub struct ModelingRenameDiagram;
impl Event for ModelingRenameDiagram {
    const NAME: &'static str = "bellatrix.mde.rename-diagram.v1";
    type Args = ModelingRenameDiagramRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingDeleteDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub expected_revision: u64,
}
pub struct ModelingDeleteDiagram;
impl Event for ModelingDeleteDiagram {
    const NAME: &'static str = "bellatrix.mde.delete-diagram.v1";
    type Args = ModelingDeleteDiagramRequested;
    type Output = Result<u64, String>;
}

/// Validation uses the required stored UMLDI type; callers cannot override it.
#[derive(Clone, Debug)]
pub struct ModelingValidateDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub expected_revision: u64,
}
#[derive(Clone, Debug)]
pub struct DiagramValidationResult {
    pub diagram_id: String,
    pub revision: u64,
    pub diagnostics: Vec<ModelingDiagnostic>,
}
pub struct ModelingValidateDiagram;
impl Event for ModelingValidateDiagram {
    const NAME: &'static str = "bellatrix.mde.validate-diagram.v1";
    type Args = ModelingValidateDiagramRequested;
    type Output = Result<DiagramValidationResult, String>;
}

#[derive(Clone)]
pub struct ModelingValidateSnapshotRequested {
    pub document: Arc<ProjectDocument>,
    pub diagram_id: String,
}
pub struct ModelingValidateSnapshot;
impl Event for ModelingValidateSnapshot {
    const NAME: &'static str = "bellatrix.mde.validate-snapshot.v1";
    type Args = ModelingValidateSnapshotRequested;
    type Output = Result<DiagramValidationResult, String>;
}

/// Events produced by an MDE run, suitable for a UI observer or JSONL export.
#[derive(Clone, Debug)]
pub struct ModelingExecutionTrace {
    pub project_id: ProjectId,
    pub run_id: String,
    pub events: Vec<ExecutionEvent>,
    pub dropped: u64,
}
pub struct ModelingExecutionTraceEvent;
impl Event for ModelingExecutionTraceEvent {
    const NAME: &'static str = "bellatrix.mde.execution-trace.v1";
    type Args = ModelingExecutionTrace;
    type Output = ();
}
static NEXT_VALIDATION_RUN: AtomicU64 = AtomicU64::new(1);

/// Opt-in validation request that returns bounded, correlated execution events.
#[derive(Clone, Debug)]
pub struct ModelingValidateDiagramTracedRequested {
    pub request: ModelingValidateDiagramRequested,
    pub run_id: String,
    pub capacity: usize,
}
#[derive(Debug)]
pub struct TracedDiagramValidationResult {
    pub result: Result<DiagramValidationResult, String>,
    pub events: Vec<ExecutionEvent>,
    pub dropped: u64,
}
pub struct ModelingValidateDiagramTraced;
impl Event for ModelingValidateDiagramTraced {
    const NAME: &'static str = "bellatrix.mde.validate-diagram-traced.v1";
    type Args = ModelingValidateDiagramTracedRequested;
    type Output = TracedDiagramValidationResult;
}

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
    let section_json = document.section(crate::MODELING_NAMESPACE).ok_or("MDE section is missing")?.to_string();
    let diagnostics = bellatrix_mde::validate_model_section_diagram(document.id().as_str(),
        document.name(), &section_json, &request.diagram_id, trace)?;
    Ok(DiagramValidationResult { diagram_id: request.diagram_id, revision: document.revision(), diagnostics })
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let snapshot_projects = Arc::clone(&projects);
    let _validate_snapshot = context.on::<ModelingValidateSnapshot, _>(responder(
        move |_: Context, request: ModelingValidateSnapshotRequested| {
            let projects = Arc::clone(&snapshot_projects);
            async move {
                let result = tokio::task::spawn_blocking(move || {
                    let current = projects.snapshot(request.document.id())
                        .map_err(|error| error.to_string())?;
                    if !Arc::ptr_eq(&current, &request.document) {
                        return Err("Validation source is no longer the active project snapshot".into());
                    }
                    let result = validate_inner(&projects, ModelingValidateDiagramRequested {
                        project_id: request.document.id().clone(),
                        diagram_id: request.diagram_id,
                        expected_revision: request.document.revision(),
                    }, None);
                    let current = projects.snapshot(request.document.id())
                        .map_err(|error| error.to_string())?;
                    if !Arc::ptr_eq(&current, &request.document) {
                        return Err("Project changed while validating its snapshot".into());
                    }
                    result
                }).await.map_err(|error| error.to_string()).and_then(|result| result);
                Ok::<_, Infallible>(Some(result))
            }
        },
    )).map_err(|error| io::Error::other(error.to_string()))?;
    let create_projects = Arc::clone(&projects);
    let _create = context
        .on::<ModelingCreateDiagram, _>(responder(
            move |context: Context, request: ModelingCreateDiagramRequested| {
                let projects = Arc::clone(&create_projects);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::CreateDiagram {
                            kind: request.kind,
                            name: request.name,
                            model_elements: request.model_elements,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let explorer_projects = Arc::clone(&projects);
    let _create_explorer = context.on::<ModelingCreateExplorerDiagram, _>(responder(
        move |context: Context, request: ModelingCreateExplorerDiagramRequested| {
            let projects = Arc::clone(&explorer_projects);
            async move {
                let diagram = request.diagram;
                let result = crate::mutation::execute(&context, &projects, diagram.project_id, diagram.expected_revision,
                    Command::CreateExplorerDiagram { kind: diagram.kind, name: diagram.name,
                        model_elements: diagram.model_elements, parent_id: request.parent_id,
                        parent_occurrence: request.parent_occurrence },
                ).await;
                Ok::<_, Infallible>(Some(result))
            }
        },
    )).map_err(|error| io::Error::other(error.to_string()))?;
    let rename_projects = Arc::clone(&projects);
    let _rename = context
        .on::<ModelingRenameDiagram, _>(responder(
            move |context: Context, request: ModelingRenameDiagramRequested| {
                let projects = Arc::clone(&rename_projects);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::RenameDiagram {
                            diagram_id: request.diagram_id,
                            name: request.name,
                        },
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
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::DeleteDiagram {
                            diagram_id: request.diagram_id,
                        },
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
                    let mut trace = ExecutionTrace::new(traced.run_id, Some(traced.request.expected_revision.to_string()), traced.capacity.min(10_000));
                    trace.emit("cordis", "diagram.validate.request", TracePhase::Start, Some(&traced.request.diagram_id), &[], None);
                    let result = validate_inner(&projects, traced.request, Some(&mut trace));
                    let (phase, outcome) = if result.is_ok() { (TracePhase::Finish, TraceOutcome::Success) } else { (TracePhase::Error, TraceOutcome::Error) };
                    trace.emit("cordis", "diagram.validate.request", phase, None, &[], Some(outcome));
                    Ok::<_, Infallible>(Some(TracedDiagramValidationResult { result, events: trace.events(), dropped: trace.dropped() }))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let _validate = context
        .on::<ModelingValidateDiagram, _>(responder(
            move |context: Context, request: ModelingValidateDiagramRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let run_id = format!("diagram-validation-{}-{}", std::process::id(), NEXT_VALIDATION_RUN.fetch_add(1, Ordering::Relaxed));
                    let mut trace = ExecutionTrace::new(run_id, Some(request.expected_revision.to_string()), 10_000);
                    let forwarder = crate::trace_output::start(context.clone(), &mut trace, request.project_id.clone());
                    trace.emit("cordis", "diagram.validate.request", TracePhase::Start, Some(&request.diagram_id), &[], None);
                    let fallback = trace.clone();
                    let (mut trace, result) = match tokio::task::spawn_blocking(move || {
                        let result = validate_inner(&projects, request, Some(&mut trace));
                        (trace, result)
                    }).await {
                        Ok(value) => value,
                        Err(error) => (fallback.clone(), Err(error.to_string())),
                    };
                    drop(fallback);
                    let (phase, outcome) = if result.is_ok() { (TracePhase::Finish, TraceOutcome::Success) } else { (TracePhase::Error, TraceOutcome::Error) };
                    trace.emit("cordis", "diagram.validate.request", phase, None, &[], Some(outcome));
                    trace.clear_sink();
                    drop(forwarder);
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}
