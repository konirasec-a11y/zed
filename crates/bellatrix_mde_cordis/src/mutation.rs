use crate::{Command, MODELING_NAMESPACE, MdjEditCommand};
use crate::{TraceOutcome, TracePhase};
use bellatrix_mde::MdeExecutionTrace as ExecutionTrace;
use bellatrix_project::{
    ProjectDocument, ProjectId, ProjectReplaceSnapshot, ProjectReplaceSnapshotRequested,
    ProjectService,
};
use cordis::{Context, Routing, event::QueryOutcome};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_MUTATION_RUN: AtomicU64 = AtomicU64::new(1);

pub(crate) async fn execute(
    context: &Context,
    projects: &ProjectService,
    project_id: ProjectId,
    expected_revision: u64,
    command: Command,
) -> Result<u64, String> {
    execute_with_interchange(
        context,
        projects,
        project_id,
        expected_revision,
        command,
        None,
    )
    .await
    .map(|document| document.revision())
}

pub(crate) async fn execute_mdj(
    context: &Context,
    projects: &ProjectService,
    document: Arc<ProjectDocument>,
    command: MdjEditCommand,
) -> Result<Arc<ProjectDocument>, String> {
    execute_with_interchange(
        context,
        projects,
        document.id().clone(),
        document.revision(),
        command.into(),
        Some(document),
    )
    .await
}

async fn execute_with_interchange(
    context: &Context,
    projects: &ProjectService,
    project_id: ProjectId,
    expected_revision: u64,
    command: Command,
    expected_snapshot: Option<Arc<ProjectDocument>>,
) -> Result<Arc<ProjectDocument>, String> {
    let name = command_name(&command);
    let run_id = format!(
        "command-{}-{}",
        std::process::id(),
        NEXT_MUTATION_RUN.fetch_add(1, Ordering::Relaxed)
    );
    let mut trace = ExecutionTrace::new(run_id, Some(expected_revision.to_string()), 1000);
    let forwarder = crate::trace_output::start(context.clone(), &mut trace, project_id.clone());
    trace.emit(
        "cordis",
        "command.execute",
        TracePhase::Start,
        Some(name),
        &[],
        None,
    );
    let result = execute_inner(
        context,
        projects,
        project_id,
        expected_revision,
        command,
        expected_snapshot,
        &mut trace,
    )
    .await;
    let (phase, outcome) = if result.is_ok() {
        (TracePhase::Finish, TraceOutcome::Success)
    } else {
        (TracePhase::Error, TraceOutcome::Error)
    };
    trace.emit(
        "cordis",
        "command.execute",
        phase,
        Some(name),
        &[],
        Some(outcome),
    );
    trace.clear_sink();
    drop(forwarder);
    result
}

async fn execute_inner(
    context: &Context,
    projects: &ProjectService,
    project_id: ProjectId,
    expected_revision: u64,
    command: Command,
    expected_snapshot: Option<Arc<ProjectDocument>>,
    trace: &mut ExecutionTrace,
) -> Result<Arc<ProjectDocument>, String> {
    let document = projects
        .snapshot(&project_id)
        .map_err(|error| error.to_string())?;
    if document.revision() != expected_revision {
        return Err("stale MDE command; refresh project snapshot".into());
    }
    let require_mdj_roundtrip = expected_snapshot.is_some();
    if expected_snapshot
        .as_ref()
        .is_some_and(|expected| !Arc::ptr_eq(expected, &document))
    {
        return Err("stale MDJ document session; the requested snapshot is no longer open".into());
    }
    if require_mdj_roundtrip
        && document
            .sections()
            .keys()
            .any(|namespace| namespace != MODELING_NAMESPACE)
    {
        return Err("MDJ interchange cannot represent other platform sections; save the complete native project instead".into());
    }
    let section_json = document
        .sections()
        .get(MODELING_NAMESPACE)
        .ok_or("MDE section is missing")?
        .to_string();
    let model_id = document.id().as_str().to_owned();
    let model_name = document.name().to_owned();
    let name = command_name(&command);
    trace.emit(
        "kernel",
        "command.apply",
        TracePhase::Start,
        Some(name),
        &[],
        None,
    );
    let (applied, failed_to_join) = match tokio::task::spawn_blocking(move || {
        bellatrix_mde::apply_model_command(
            &model_id,
            &model_name,
            &section_json,
            expected_revision,
            command,
            require_mdj_roundtrip,
        )
    })
    .await
    {
        Ok(result) => (result, false),
        Err(error) => (Err(error.to_string()), true),
    };
    let (phase, outcome) = if applied.is_ok() {
        (TracePhase::Finish, TraceOutcome::Success)
    } else if failed_to_join {
        (TracePhase::Error, TraceOutcome::Error)
    } else {
        (TracePhase::Error, TraceOutcome::Invalid)
    };
    trace.emit(
        "kernel",
        "command.apply",
        phase,
        Some(name),
        &[],
        Some(outcome),
    );
    let state = applied?;
    // Bind publication to the exact source, including native documents: a close
    // and reopen may reuse an ID and revision while changing the model entirely.
    let replaced = context
        .query::<ProjectReplaceSnapshot>(
            Routing::Unscoped,
            ProjectReplaceSnapshotRequested {
                document,
                namespace: MODELING_NAMESPACE.into(),
                state,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    match replaced {
        QueryOutcome::Answer(Ok(document)) => Ok(document),
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
