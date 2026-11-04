//! A typed GPUI -> Cordis transport, not an event bus or a second engine runtime.
//! The host owns this bridge and its lifetime. The UI only owns a sender.
use crate::Command;
use crate::{
    ModelingAddMember, ModelingAddMemberRequested, ModelingCreateDiagram,
    ModelingCreateDiagramRequested, ModelingCreateElement, ModelingCreateElementOrVisualDraft,
    ModelingCreateElementRequested, ModelingCreateRelation,
    ModelingCreateRelationAtViewsOrVisualDraft, ModelingCreateRelationAtViewsRequested,
    ModelingCreateRelationOrVisualDraft, ModelingCreateRelationRequested,
    ModelingCreateVisualDraft, ModelingCreateVisualDraftRequested, ModelingDeleteDiagram,
    ModelingDeleteDiagramRequested, ModelingDeleteElement, ModelingDeleteElementRequested,
    ModelingMoveParameter, ModelingMoveParameterRequested, ModelingMoveView,
    ModelingMoveViewRequested, ModelingPlaceElementView, ModelingPlaceElementViewRequested,
    ModelingQuickEdit, ModelingQuickEditRequested, ModelingRemoveView, ModelingRemoveViewRequested,
    ModelingRename, ModelingRenameDiagram, ModelingRenameDiagramRequested, ModelingRenameRequested,
    ModelingReorderView, ModelingReorderViewRequested, ModelingResizeView,
    ModelingResizeViewRequested, ModelingSetExplorerParent, ModelingSetExplorerParentRequested,
    ModelingSetNameExpression, ModelingSetNameExpressionRequested, ModelingSetOwner,
    ModelingSetOwnerRequested, ModelingSetProperty, ModelingSetPropertyRequested,
    ModelingSetViewLabel, ModelingSetViewLabelRequested, ModelingSetViewRouting,
    ModelingSetViewRoutingRequested, ModelingSetViewStyle, ModelingSetViewStyleRequested,
    ModelingSetViewTiming, ModelingSetViewTimingRequested, ModelingValidateDiagram,
    ModelingValidateDiagramRequested,
};
use bellatrix_project::{
    ProjectClosed, ProjectClosedEvent, ProjectId, ProjectOpened, ProjectOpenedEvent,
    ProjectSave, ProjectSaveRequested, ProjectSaved, ProjectSavedEvent,
    ProjectStateChanged, ProjectStateChangedEvent,
};
use cordis::event::{ListenerRegistration, ListenerRegistrationError, QueryOutcome, observer_sync};
use cordis::{Context, Routing};
use std::{convert::Infallible, fmt::Display, sync::Arc};
use tokio::sync::{mpsc, watch};

#[derive(Clone, Debug)]
pub struct DiagramValidationFailure {
    pub diagram_id: String,
    pub revision: u64,
    pub message: String,
}

#[derive(Clone)]
pub enum MdeUiCommand {
    SetViewLabel(ModelingSetViewLabelRequested),
    SetViewLabelTracked {
        request: ModelingSetViewLabelRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    SetViewTimingTracked {
        request: ModelingSetViewTimingRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    SetViewStyle(ModelingSetViewStyleRequested),
    SetViewStyleTracked {
        request: ModelingSetViewStyleRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    SetViewRouting(ModelingSetViewRoutingRequested),
    SetViewRoutingTracked {
        request: ModelingSetViewRoutingRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    ReorderViewTracked {
        request: ModelingReorderViewRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    CreateDiagram(ModelingCreateDiagramRequested),
    CreateExplorerDiagram(crate::ModelingCreateExplorerDiagramRequested),
    RenameDiagram(ModelingRenameDiagramRequested),
    DeleteDiagram(ModelingDeleteDiagramRequested),
    ValidateDiagram(ModelingValidateDiagramRequested),
    CreateVisualDraft(ModelingCreateVisualDraftRequested),
    CreateElementOrVisualDraft(ModelingCreateElementRequested),
    CreateElement(ModelingCreateElementRequested),
    CreateModelElement(crate::ModelingCreateModelElementRequested),
    CreateRelationOrVisualDraft(ModelingCreateRelationRequested),
    CreateRelationAtViewsOrVisualDraft(ModelingCreateRelationAtViewsRequested),
    CreateRelation(ModelingCreateRelationRequested),
    MoveView(ModelingMoveViewRequested),
    ResizeView(ModelingResizeViewRequested),
    RemoveView(ModelingRemoveViewRequested),
    PlaceElementView(ModelingPlaceElementViewRequested),
    Rename(ModelingRenameRequested),
    SetOwner(ModelingSetOwnerRequested),
    SetExplorerParent(ModelingSetExplorerParentRequested),
    DeleteElement(ModelingDeleteElementRequested),
    SetProperty(ModelingSetPropertyRequested),
    SetPropertyTracked {
        request: ModelingSetPropertyRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    AddMember(ModelingAddMemberRequested),
    MoveParameter(ModelingMoveParameterRequested),
    QuickEdit(ModelingQuickEditRequested),
    SetNameExpression(ModelingSetNameExpressionRequested),
    SaveProject(ProjectSaveRequested),
}

pub struct MdeUiBridge {
    project_id: ProjectId,
    projections: watch::Receiver<Result<Arc<crate::ModelingSnapshot>, String>>,
    projection_task: tokio::task::JoinHandle<()>,
    move_previews: crate::move_preview::MovePreviewClient,
    commands: mpsc::Sender<MdeUiCommand>,
    changes: watch::Receiver<u64>,
    feedback: watch::Receiver<String>,
    validations:
        watch::Receiver<Option<Result<crate::DiagramValidationResult, DiagramValidationFailure>>>,
    saves: watch::Receiver<Option<u64>>,
    journal: Arc<crate::ExecutionJournal>,
    _listeners: Vec<ListenerRegistration>,
    task: tokio::task::JoinHandle<()>,
    mdj_session: Option<crate::MdjSession>,
    mdj_shutdown: Option<watch::Sender<bool>>,
}
impl MdeUiBridge {
    pub fn start(ctx: &Context, project_id: ProjectId) -> Result<Self, ListenerRegistrationError> {
        let (command_tx, mut command_rx) = mpsc::channel::<MdeUiCommand>(64);
        let (revision_tx, changes) = watch::channel(0_u64);
        let (feedback_tx, feedback) = watch::channel(String::new());
        let (validation_tx, validations) = watch::channel(None);
        let (save_tx, saves) = watch::channel(None);
        let id = project_id.clone();
        let state_revision_tx = revision_tx.clone();
        let listener = ctx.on::<ProjectStateChangedEvent, _>(observer_sync(
            move |_, changed: ProjectStateChanged| {
                if changed.id == id {
                    state_revision_tx.send_replace(changed.revision);
                }
                Ok::<_, Infallible>(())
            },
        ))?;
        let opened_id = project_id.clone();
        let opened_revision_tx = revision_tx.clone();
        let opened_listener = match ctx.on::<ProjectOpenedEvent, _>(observer_sync(
            move |_, opened: ProjectOpened| {
                if opened.document.id() == &opened_id {
                    opened_revision_tx.send_replace(opened.document.revision());
                }
                Ok::<_, Infallible>(())
            },
        )) {
            Ok(listener) => listener,
            Err(error) => {
                listener.remove();
                return Err(error);
            }
        };
        let closed_id = project_id.clone();
        let closed_listener = match ctx.on::<ProjectClosedEvent, _>(observer_sync(
            move |_, closed: ProjectClosed| {
                if closed.id == closed_id {
                    revision_tx.send_modify(|_| {});
                }
                Ok::<_, Infallible>(())
            },
        )) {
            Ok(listener) => listener,
            Err(error) => {
                listener.remove();
                opened_listener.remove();
                return Err(error);
            }
        };
        let save_id = project_id.clone();
        let save_listener =
            match ctx.on::<ProjectSavedEvent, _>(observer_sync(move |_, saved: ProjectSaved| {
                if saved.id == save_id {
                    save_tx.send_replace(Some(saved.revision));
                }
                Ok::<_, Infallible>(())
            })) {
                Ok(listener) => listener,
                Err(error) => {
                    listener.remove();
                    opened_listener.remove();
                    closed_listener.remove();
                    return Err(error);
                }
            };
        let journal = Arc::new(crate::ExecutionJournal::default());
        let trace_journal = Arc::clone(&journal);
        let trace_project_id = project_id.clone();
        let trace_listener = match ctx.on::<crate::ModelingExecutionTraceEvent, _>(observer_sync(
            move |_, trace: crate::ModelingExecutionTrace| {
                if trace.project_id == trace_project_id {
                    trace_journal.append(trace);
                }
                Ok::<_, Infallible>(())
            },
        )) {
            Ok(listener) => listener,
            Err(error) => {
                listener.remove();
                opened_listener.remove();
                closed_listener.remove();
                save_listener.remove();
                return Err(error);
            }
        };
        let cordis = ctx.clone();
        let task = tokio::spawn(async move {
            while let Some(request) = command_rx.recv().await {
                let message = match request {
                    MdeUiCommand::SetViewLabel(request) => describe(
                        "View label updated",
                        cordis
                            .query::<ModelingSetViewLabel>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SetViewLabelTracked {
                        request,
                        completion,
                    } => {
                        let result = cordis
                            .query::<ModelingSetViewLabel>(Routing::Unscoped, request)
                            .await;
                        let receipt = match &result {
                            Ok(QueryOutcome::Answer(Ok(revision))) => Ok(*revision),
                            Ok(QueryOutcome::Answer(Err(error))) => Err(error.to_string()),
                            Ok(QueryOutcome::Miss) => Err("Cordis responder unavailable".into()),
                            Err(error) => Err(error.to_string()),
                        };
                        completion.send_replace(Some(receipt));
                        describe("View label updated", result)
                    }
                    MdeUiCommand::SetViewTimingTracked {
                        request,
                        completion,
                    } => {
                        let result = cordis
                            .query::<ModelingSetViewTiming>(Routing::Unscoped, request)
                            .await;
                        let receipt = match &result {
                            Ok(QueryOutcome::Answer(Ok(revision))) => Ok(*revision),
                            Ok(QueryOutcome::Answer(Err(error))) => Err(error.to_string()),
                            Ok(QueryOutcome::Miss) => Err("Cordis responder unavailable".into()),
                            Err(error) => Err(error.to_string()),
                        };
                        completion.send_replace(Some(receipt));
                        describe("View timing updated", result)
                    }
                    MdeUiCommand::SetViewStyle(request) => describe(
                        "View style updated",
                        cordis
                            .query::<ModelingSetViewStyle>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SetViewStyleTracked {
                        request,
                        completion,
                    } => {
                        let result = cordis
                            .query::<ModelingSetViewStyle>(Routing::Unscoped, request)
                            .await;
                        let receipt = match &result {
                            Ok(QueryOutcome::Answer(Ok(revision))) => Ok(*revision),
                            Ok(QueryOutcome::Answer(Err(error))) => Err(error.to_string()),
                            Ok(QueryOutcome::Miss) => Err("Cordis responder unavailable".into()),
                            Err(error) => Err(error.to_string()),
                        };
                        completion.send_replace(Some(receipt));
                        describe("View style updated", result)
                    }
                    MdeUiCommand::SetViewRouting(request) => describe(
                        "View routing updated",
                        cordis
                            .query::<ModelingSetViewRouting>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SetViewRoutingTracked {
                        request,
                        completion,
                    } => {
                        let result = cordis
                            .query::<ModelingSetViewRouting>(Routing::Unscoped, request)
                            .await;
                        let receipt = match &result {
                            Ok(QueryOutcome::Answer(Ok(revision))) => Ok(*revision),
                            Ok(QueryOutcome::Answer(Err(error))) => Err(error.to_string()),
                            Ok(QueryOutcome::Miss) => Err("Cordis responder unavailable".into()),
                            Err(error) => Err(error.to_string()),
                        };
                        completion.send_replace(Some(receipt));
                        describe("View routing updated", result)
                    }
                    MdeUiCommand::ReorderViewTracked {
                        request,
                        completion,
                    } => {
                        let result = cordis
                            .query::<ModelingReorderView>(Routing::Unscoped, request)
                            .await;
                        let receipt = match &result {
                            Ok(QueryOutcome::Answer(Ok(revision))) => Ok(*revision),
                            Ok(QueryOutcome::Answer(Err(error))) => Err(error.to_string()),
                            Ok(QueryOutcome::Miss) => Err("Cordis responder unavailable".into()),
                            Err(error) => Err(error.to_string()),
                        };
                        completion.send_replace(Some(receipt));
                        describe("View order updated", result)
                    }
                    MdeUiCommand::ValidateDiagram(request) => {
                        let diagram_id = request.diagram_id.clone();
                        let revision = request.expected_revision;
                        let result = cordis
                            .query::<ModelingValidateDiagram>(Routing::Unscoped, request)
                            .await;
                        let outcome = match result {
                            Ok(QueryOutcome::Answer(Ok(validation))) => Ok(validation),
                            Ok(QueryOutcome::Answer(Err(error))) => Err(error),
                            Ok(QueryOutcome::Miss) => Err("Cordis responder unavailable".into()),
                            Err(error) => Err(format!("Cordis dispatch failed: {error}")),
                        }
                        .map_err(|message| DiagramValidationFailure {
                            diagram_id,
                            revision,
                            message,
                        });
                        validation_tx.send_replace(Some(outcome));
                        continue;
                    }
                    MdeUiCommand::CreateExplorerDiagram(request) => describe(
                        "Diagram created",
                        cordis
                            .query::<crate::ModelingCreateExplorerDiagram>(
                                Routing::Unscoped,
                                request,
                            )
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
                    MdeUiCommand::CreateElementOrVisualDraft(request) => describe(
                        "Element or visual draft created",
                        cordis
                            .query::<ModelingCreateElementOrVisualDraft>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::CreateModelElement(request) => describe(
                        "Model element created",
                        cordis
                            .query::<crate::ModelingCreateModelElement>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::CreateElement(request) => describe(
                        "Element created",
                        cordis
                            .query::<ModelingCreateElement>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::CreateRelationAtViewsOrVisualDraft(request) => describe(
                        "Relationship or visual draft created",
                        cordis
                            .query::<ModelingCreateRelationAtViewsOrVisualDraft>(
                                Routing::Unscoped,
                                request,
                            )
                            .await,
                    ),
                    MdeUiCommand::CreateRelationOrVisualDraft(request) => describe(
                        "Relationship or visual draft created",
                        cordis
                            .query::<ModelingCreateRelationOrVisualDraft>(
                                Routing::Unscoped,
                                request,
                            )
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
                    MdeUiCommand::CreateVisualDraft(request) => describe(
                        "Visual draft created",
                        cordis
                            .query::<ModelingCreateVisualDraft>(Routing::Unscoped, request)
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
                    MdeUiCommand::SetExplorerParent(request) => describe(
                        "Explorer placement updated",
                        cordis
                            .query::<ModelingSetExplorerParent>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SetOwner(request) => describe(
                        "Element ownership updated",
                        cordis
                            .query::<ModelingSetOwner>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::DeleteElement(request) => describe(
                        "Element deleted",
                        cordis
                            .query::<ModelingDeleteElement>(Routing::Unscoped, request)
                            .await,
                    ),
                    MdeUiCommand::SetPropertyTracked {
                        request,
                        completion,
                    } => {
                        let result = cordis
                            .query::<ModelingSetProperty>(Routing::Unscoped, request)
                            .await;
                        let receipt = match &result {
                            Ok(QueryOutcome::Answer(Ok(revision))) => Ok(*revision),
                            Ok(QueryOutcome::Answer(Err(error))) => Err(error.to_string()),
                            Ok(QueryOutcome::Miss) => Err("Cordis responder unavailable".into()),
                            Err(error) => Err(error.to_string()),
                        };
                        completion.send_replace(Some(receipt));
                        describe("Property updated", result)
                    }
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
                    MdeUiCommand::MoveParameter(request) => describe(
                        "Parameter order updated",
                        cordis
                            .query::<ModelingMoveParameter>(Routing::Unscoped, request)
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
        let (projections, projection_task) = crate::projection::start_feed(
            ctx.clone(), project_id.clone(), changes.clone(), None);
        Ok(Self {
            projections, projection_task,
            move_previews: crate::move_preview::MovePreviewClient::new(ctx.clone(), tokio::runtime::Handle::current()),
            project_id,
            commands: command_tx,
            changes,
            feedback,
            validations,
            saves,
            journal,
            _listeners: vec![listener, opened_listener, closed_listener, save_listener, trace_listener],
            task,
            mdj_session: None,
            mdj_shutdown: None,
        })
    }
    /// Start a restricted MDJ session. Its snapshot advances only from an acknowledged
    /// transactional MDJ edit, never from a project-id lookup or a generic event.
    pub fn start_mdj(
        ctx: &Context,
        document: Arc<bellatrix_project::ProjectDocument>,
    ) -> Result<Self, ListenerRegistrationError> {
        let project_id = document.id().clone();
        // Keep the native accessor type stable, but make raw MDJ sends fail closed.
        // All accepted MDJ submissions must carry a pending guard via MdjSession.
        let (command_tx, command_rx) = mpsc::channel(1);
        drop(command_rx);
        let (revision_tx, changes) = watch::channel(document.revision());
        let (feedback_tx, feedback) = watch::channel(String::new());
        let (validation_tx, validations) = watch::channel(None);
        // Generic saved events carry no session identity and must not acknowledge MDJ I/O.
        let (_, saves) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (submission_tx, submission_rx) = mpsc::channel(64);
        let session = crate::MdjSession::new(document, submission_tx, revision_tx.clone());
        let task = tokio::spawn(crate::mdj_ui_bridge::run(
            ctx.clone(),
            session.clone(),
            submission_rx,
            shutdown_rx,
            revision_tx,
            feedback_tx,
            validation_tx,
        ));
        let (projections, projection_task) = crate::projection::start_feed(
            ctx.clone(), project_id.clone(), changes.clone(), Some(session.clone()));
        Ok(Self {
            projections, projection_task,
            move_previews: crate::move_preview::MovePreviewClient::new(ctx.clone(), tokio::runtime::Handle::current()),
            project_id,
            commands: command_tx,
            changes,
            feedback,
            validations,
            saves,
            journal: Arc::new(crate::ExecutionJournal::default()),
            _listeners: Vec::new(),
            task,
            mdj_session: Some(session),
            mdj_shutdown: Some(shutdown_tx),
        })
    }

    /// Retain this handle across bridge shutdown; await `wait_closed` before cleanup.
    pub fn mdj_session(&self) -> Option<crate::MdjSession> {
        self.mdj_session.clone()
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
    /// Native commands only. MDJ bridges return a closed sender; use MdjSession::submit.
    pub fn sender(&self) -> mpsc::Sender<MdeUiCommand> {
        self.commands.clone()
    }
    pub fn projections(&self) -> watch::Receiver<Result<Arc<crate::ModelingSnapshot>, String>> {
        self.projections.clone()
    }
    pub fn move_previews(&self) -> crate::move_preview::MovePreviewClient {
        self.move_previews.clone()
    }
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.changes.clone()
    }
    pub fn validations(
        &self,
    ) -> watch::Receiver<Option<Result<crate::DiagramValidationResult, DiagramValidationFailure>>>
    {
        self.validations.clone()
    }
    pub fn saves(&self) -> watch::Receiver<Option<u64>> {
        self.saves.clone()
    }
    pub fn execution_journal(&self) -> Arc<crate::ExecutionJournal> {
        Arc::clone(&self.journal)
    }
    pub fn feedback(&self) -> watch::Receiver<String> {
        self.feedback.clone()
    }
}
impl Drop for MdeUiBridge {
    fn drop(&mut self) {
        self.projection_task.abort();
        if let Some(shutdown) = &self.mdj_shutdown {
            // Do not cancel an edit between its commit and delivery of the new Arc.
            shutdown.send_replace(true);
        } else {
            self.task.abort();
        }
        for listener in self._listeners.drain(..) {
            listener.remove();
        }
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
