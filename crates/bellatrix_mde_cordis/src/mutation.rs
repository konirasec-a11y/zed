use crate::project_integration::from_document;
use bellatrix_mde::{
    ExecutionTrace, TracePhase, TraceOutcome, MODELING_NAMESPACE, ModelingSection,
    commands::{self, Command},
};
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::{Context, Routing, event::QueryOutcome};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_MUTATION_RUN: AtomicU64 = AtomicU64::new(1);

pub(crate) async fn execute(
    context: &Context,
    projects: &ProjectService,
    project_id: ProjectId,
    expected_revision: u64,
    command: Command,
) -> Result<u64, String> {
    let name = command_name(&command);
    let run_id = format!("command-{}-{}", std::process::id(), NEXT_MUTATION_RUN.fetch_add(1, Ordering::Relaxed));
    let mut trace = ExecutionTrace::new(run_id, Some(expected_revision.to_string()), 1000);
    let forwarder = crate::trace_output::start(context.clone(), &mut trace, project_id.clone());
    trace.emit("cordis", "command.execute", TracePhase::Start, Some(name), &[], None);
    let result = execute_inner(context, projects, project_id, expected_revision, command, &mut trace).await;
    let (phase, outcome) = if result.is_ok() { (TracePhase::Finish, TraceOutcome::Success) } else { (TracePhase::Error, TraceOutcome::Error) };
    trace.emit("cordis", "command.execute", phase, Some(name), &[], Some(outcome));
    trace.clear_sink();
    drop(forwarder);
    result
}

async fn execute_inner(
    context: &Context, projects: &ProjectService, project_id: ProjectId,
    expected_revision: u64, command: Command, trace: &mut ExecutionTrace,
) -> Result<u64, String> {
    let document = projects
        .snapshot(&project_id)
        .map_err(|error| error.to_string())?;
    if document.revision() != expected_revision {
        return Err("stale MDE command; refresh project snapshot".into());
    }
    let section = from_document(&document)?;
    let source = section.to_model(document.id().as_str(), document.name());
    let name = command_name(&command);
    trace.emit("kernel", "command.apply", TracePhase::Start, Some(name), &[], None);
    let (applied, failed_to_join) = match tokio::task::spawn_blocking(move || commands::apply(&source, &command, expected_revision)).await {
        Ok(result) => (result, false),
        Err(error) => (Err(error.to_string()), true),
    };
    let (phase, outcome) = if applied.is_ok() { (TracePhase::Finish, TraceOutcome::Success) } else if failed_to_join { (TracePhase::Error, TraceOutcome::Error) } else { (TracePhase::Error, TraceOutcome::Invalid) };
    trace.emit("kernel", "command.apply", phase, Some(name), &[], Some(outcome));
    let updated = applied?;
    let state = serde_json::to_value(ModelingSection::from_model(&updated))
        .map_err(|error| error.to_string())?;
    match context
        .query::<ProjectReplace>(
            Routing::Unscoped,
            ProjectReplaceRequested {
                id: project_id,
                namespace: MODELING_NAMESPACE.into(),
                expected_revision,
                state,
            },
        )
        .await
        .map_err(|error| error.to_string())?
    {
        QueryOutcome::Answer(Ok(document)) => Ok(document.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService unavailable".into()),
    }
}

fn command_name(command: &Command) -> &'static str {
    match command {
        Command::AdoptRootModel => "AdoptRootModel",
        Command::SetOwner { .. } => "SetOwner",
        Command::SetExplorerParent { .. } => "SetExplorerParent",
        Command::CreateExplorerDiagram { .. } => "CreateExplorerDiagram",
        Command::CreateDiagram { .. } => "CreateDiagram",
        Command::RenameDiagram { .. } => "RenameDiagram",
        Command::DeleteDiagram { .. } => "DeleteDiagram",
        Command::CreateModelElement { .. } => "CreateModelElement",
        Command::CreateElement { .. } => "CreateElement",
        Command::CreateElementOrVisualDraft { .. } => "CreateElementOrVisualDraft",
        Command::CreateRelationOrVisualDraft { .. } => "CreateRelationOrVisualDraft",
        Command::CreateRelationAtViewsOrVisualDraft { .. } => "CreateRelationAtViewsOrVisualDraft",
        Command::CreateRelationAtViews { .. } => "CreateRelationAtViews",
        Command::CreateRelation { .. } => "CreateRelation",
        Command::AddMember { .. } => "AddMember",
        Command::MoveParameter { .. } => "MoveParameter",
        Command::MigrateAssociation { .. } => "MigrateAssociation",
        Command::SetRedefinedElements { .. } => "SetRedefinedElements",
        Command::SetSubsettedProperties { .. } => "SetSubsettedProperties",
        Command::SetNamespaceImport { .. } => "SetNamespaceImport",
        Command::SetAssociationEndNavigation { .. } => "SetAssociationEndNavigation",
        Command::SetAssociationMemberEnds { .. } => "SetAssociationMemberEnds",
        Command::EditAssociationEnds { .. } => "EditAssociationEnds",
        Command::QuickEdit { .. } => "QuickEdit",
        Command::CreateVisualDraft { .. } => "CreateVisualDraft",
        Command::PlaceElementView { .. } => "PlaceElementView",
        Command::RemoveView { .. } => "RemoveView",
        Command::ReorderView { .. } => "ReorderView",
        Command::SetViewLabel { .. } => "SetViewLabel",
        Command::SetViewTiming { .. } => "SetViewTiming",
        Command::SetViewEndpoints { .. } => "SetViewEndpoints",
        Command::SetViewStyle { .. } => "SetViewStyle",
        Command::SetViewRouting { .. } => "SetViewRouting",
        Command::MoveView { .. } => "MoveView",
        Command::ResizeView { .. } => "ResizeView",
        Command::DeleteElement { .. } => "DeleteElement",
        Command::SetProperty { .. } => "SetProperty",
        Command::SetNameExpression { .. } => "SetNameExpression",
        Command::Rename { .. } => "Rename",
    }
}
