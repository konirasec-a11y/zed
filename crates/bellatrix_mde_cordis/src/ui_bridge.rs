//! A typed GPUI -> Cordis transport, not an event bus or a second engine runtime.
//! The host owns this bridge and its lifetime. The UI only owns a sender.
use crate::{
    ModelingAddMember, ModelingAddMemberRequested, ModelingCreateDiagram,
    ModelingCreateDiagramRequested, ModelingCreateElement, ModelingCreateElementRequested,
    ModelingCreateRelation, ModelingCreateRelationRequested, ModelingDeleteDiagram,
    ModelingDeleteDiagramRequested, ModelingDeleteElement, ModelingDeleteElementRequested,
    ModelingMoveView, ModelingMoveViewRequested, ModelingPlaceElementView,
    ModelingPlaceElementViewRequested, ModelingQuickEdit, ModelingQuickEditRequested,
    ModelingRemoveView, ModelingRemoveViewRequested, ModelingRename, ModelingRenameDiagram,
    ModelingRenameDiagramRequested, ModelingRenameRequested, ModelingResizeView,
    ModelingResizeViewRequested, ModelingSetNameExpression, ModelingSetNameExpressionRequested,
    ModelingSetProperty, ModelingSetPropertyRequested, ModelingValidateDiagram,
    ModelingValidateDiagramRequested,
};
use bellatrix_project::{
    ProjectId, ProjectSave, ProjectSaveRequested, ProjectStateChanged, ProjectStateChangedEvent,
};
use cordis::event::{ListenerRegistration, ListenerRegistrationError, QueryOutcome, observer_sync};
use cordis::{Context, Routing};
use std::{convert::Infallible, fmt::Display};
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub enum MdeUiCommand {
    CreateDiagram(ModelingCreateDiagramRequested),
    RenameDiagram(ModelingRenameDiagramRequested),
    DeleteDiagram(ModelingDeleteDiagramRequested),
    ValidateDiagram(ModelingValidateDiagramRequested),
    CreateElement(ModelingCreateElementRequested),
    CreateRelation(ModelingCreateRelationRequested),
    MoveView(ModelingMoveViewRequested),
    ResizeView(ModelingResizeViewRequested),
    RemoveView(ModelingRemoveViewRequested),
    PlaceElementView(ModelingPlaceElementViewRequested),
    Rename(ModelingRenameRequested),
    DeleteElement(ModelingDeleteElementRequested),
    SetProperty(ModelingSetPropertyRequested),
    AddMember(ModelingAddMemberRequested),
    QuickEdit(ModelingQuickEditRequested),
    SetNameExpression(ModelingSetNameExpressionRequested),
    SaveProject(ProjectSaveRequested),
}

pub struct MdeUiBridge {
    project_id: ProjectId,
    commands: mpsc::Sender<MdeUiCommand>,
    changes: watch::Receiver<u64>,
    feedback: watch::Receiver<String>,
    _listener: ListenerRegistration,
    task: tokio::task::JoinHandle<()>,
}
impl MdeUiBridge {
    pub fn start(ctx: &Context, project_id: ProjectId) -> Result<Self, ListenerRegistrationError> {
        let (command_tx, mut command_rx) = mpsc::channel::<MdeUiCommand>(64);
        let (revision_tx, changes) = watch::channel(0_u64);
        let (feedback_tx, feedback) = watch::channel(String::new());
        let id = project_id.clone();
        let listener = ctx.on::<ProjectStateChangedEvent, _>(observer_sync(
            move |_, changed: ProjectStateChanged| {
                if changed.id == id {
                    revision_tx.send_replace(changed.revision);
                }
                Ok::<_, Infallible>(())
            },
        ))?;
        let cordis = ctx.clone();
        let task = tokio::spawn(async move {
            while let Some(request) = command_rx.recv().await {
                let message = match request {
                    MdeUiCommand::ValidateDiagram(request) => describe_validation(
                        cordis
                            .query::<ModelingValidateDiagram>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::CreateDiagram(request) => describe(
                        "Diagram created",
                        cordis
                            .query::<ModelingCreateDiagram>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::RenameDiagram(request) => describe(
                        "Diagram renamed",
                        cordis
                            .query::<ModelingRenameDiagram>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::DeleteDiagram(request) => describe(
                        "Diagram deleted",
                        cordis
                            .query::<ModelingDeleteDiagram>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::CreateElement(request) => describe(
                        "Element created",
                        cordis
                            .query::<ModelingCreateElement>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::CreateRelation(request) => describe(
                        "Relationship created",
                        cordis
                            .query::<ModelingCreateRelation>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::MoveView(request) => describe(
                        "View moved",
                        cordis
                            .query::<ModelingMoveView>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::PlaceElementView(request) => describe(
                        "Existing element placed in diagram",
                        cordis
                            .query::<ModelingPlaceElementView>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::RemoveView(request) => describe(
                        "View removed from diagram",
                        cordis
                            .query::<ModelingRemoveView>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::ResizeView(request) => describe(
                        "View resized",
                        cordis
                            .query::<ModelingResizeView>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::Rename(request) => describe(
                        "Element renamed",
                        cordis
                            .query::<ModelingRename>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::DeleteElement(request) => describe(
                        "Element deleted",
                        cordis
                            .query::<ModelingDeleteElement>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SetProperty(request) => describe(
                        "Property updated",
                        cordis
                            .query::<ModelingSetProperty>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::AddMember(request) => describe(
                        "Member created",
                        cordis
                            .query::<ModelingAddMember>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::QuickEdit(request) => describe(
                        "UML Quick Edit",
                        cordis
                            .query::<ModelingQuickEdit>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SetNameExpression(request) => describe(
                        "UML name updated",
                        cordis
                            .query::<ModelingSetNameExpression>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SaveProject(request) => describe(
                        "Project saved",
                        cordis
                            .query::<ProjectSave>(Routing::Unscoped, request)
                            .await,
                    ),
                };
                feedback_tx.send_replace(message);
            }
        });
        Ok(Self {
            project_id,
            commands: command_tx,
            changes,
            feedback,
            _listener: listener,
            task,
        })
    }
    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
    pub fn sender(&self) -> mpsc::Sender<MdeUiCommand> {
        self.commands.clone()
    }
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.changes.clone()
    }
    pub fn feedback(&self) -> watch::Receiver<String> {
        self.feedback.clone()
    }
}
impl Drop for MdeUiBridge {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn describe<E: Display, D: Display>(
    title: &str,
    result: Result<QueryOutcome<Result<u64, E>>, D>,
) -> String {
    match result {
        Ok(QueryOutcome::Answer(Ok(revision))) => format!("{title}: revision {revision}"),
        Ok(QueryOutcome::Answer(Err(error))) => format!("{title} rejected: {error}"),
        Ok(QueryOutcome::Miss) => "Cordis responder unavailable".into(),
        Err(error) => format!("Cordis dispatch failed: {error}"),
    }
}

fn describe_validation<D: Display>(
    result: Result<QueryOutcome<Result<Vec<bellatrix_mde::StructuralDiagnostic>, String>>, D>,
) -> String {
    match result {
        Ok(QueryOutcome::Answer(Ok(diagnostics))) if diagnostics.is_empty() =>
            "Diagram validation: no issues found in the implemented checks (not full UML conformance)".into(),
        Ok(QueryOutcome::Answer(Ok(diagnostics))) => {
            let details = diagnostics.iter().map(|diagnostic| format!("{:?} {} [{}]: {}",
                diagnostic.severity, diagnostic.code, diagnostic.subject_id, diagnostic.message)).collect::<Vec<_>>().join("\n");
            format!("Diagram validation: {} diagnostic(s)\n{details}", diagnostics.len())
        }
        Ok(QueryOutcome::Answer(Err(error))) => format!("Diagram validation failed: {error}"),
        Ok(QueryOutcome::Miss) => "Cordis responder unavailable".into(),
        Err(error) => format!("Cordis dispatch failed: {error}"),
    }
}
