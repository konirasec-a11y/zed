//! One lifecycle-owned transport for every native modeling document.
use crate::*;
use bellatrix_project::{
    ProjectClosed, ProjectClosedEvent, ProjectId, ProjectSaveRequested, ProjectSaved,
    ProjectSavedEvent, ProjectService, ProjectStateChanged, ProjectStateChangedEvent,
};
use cordis::{
    Context,
    event::{ListenerRegistration, observer_sync},
};
use std::{convert::Infallible, io, sync::Arc};
use tokio::sync::watch;

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
    SetViewPath(ModelingSetViewPathRequested),
    SetViewPathTracked {
        request: ModelingSetViewPathRequested,
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
    move_previews: crate::MovePreviewClient,
    session: crate::MdeSession,
    changes: watch::Receiver<u64>,
    feedback: watch::Receiver<String>,
    validations:
        watch::Receiver<Option<Result<crate::DiagramValidationResult, DiagramValidationFailure>>>,
    saves: watch::Receiver<Option<u64>>,
    journal: Arc<crate::ExecutionJournal>,
    listeners: Vec<ListenerRegistration>,
    _task: tokio::task::JoinHandle<()>,
}

impl MdeUiBridge {
    pub fn start(context: &Context, project_id: ProjectId) -> Result<Self, io::Error> {
        let projects = context
            .try_service::<ProjectService>()
            .map_err(|error| io::Error::other(error.to_string()))?;
        let document = projects
            .snapshot(&project_id)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let (revision_tx, changes) = watch::channel(document.revision());
        let (feedback_tx, feedback) = watch::channel(String::new());
        let (validation_tx, validations) = watch::channel(None);
        let (save_tx, saves) = watch::channel(None);
        let (session, submissions) = crate::ui_session::create(projects, document);
        let journal = Arc::new(crate::ExecutionJournal::default());
        let mut listeners = Vec::new();
        let registration =
            (|| {
                let observed = session.clone();
                let revisions = revision_tx.clone();
                listeners.push(context.on::<ProjectStateChangedEvent, _>(observer_sync(
                    move |_, changed: ProjectStateChanged| {
                        if observed.snapshot().is_ok_and(|document| {
                            document.id() == &changed.id && document.revision() == changed.revision
                        }) {
                            revisions.send_replace(changed.revision);
                        }
                        Ok::<_, Infallible>(())
                    },
                ))?);
                let observed = session.clone();
                let closed_id = project_id.clone();
                let revisions = revision_tx.clone();
                listeners.push(context.on::<ProjectClosedEvent, _>(observer_sync(
                    move |_, closed: ProjectClosed| {
                        if closed.id == closed_id && observed.matches_session(&closed.session) {
                            observed.shutdown();
                            revisions.send_modify(|_| {});
                        }
                        Ok::<_, Infallible>(())
                    },
                ))?);
                let observed = session.clone();
                listeners.push(context.on::<ProjectSavedEvent, _>(observer_sync(
                    move |_, saved: ProjectSaved| {
                        if observed
                            .snapshot()
                            .is_ok_and(|document| Arc::ptr_eq(&document, &saved.document))
                        {
                            save_tx.send_replace(Some(saved.revision));
                        }
                        Ok::<_, Infallible>(())
                    },
                ))?);
                let trace_journal = Arc::clone(&journal);
                let observed = session.clone();
                listeners.push(context.on::<crate::ModelingExecutionTraceEvent, _>(
                    observer_sync(move |_, trace: crate::ModelingExecutionTrace| {
                        if observed
                            .snapshot()
                            .is_ok_and(|document| document.id() == &trace.project_id)
                        {
                            trace_journal.append(trace);
                        }
                        Ok::<_, Infallible>(())
                    }),
                )?);
                Ok::<_, cordis::event::ListenerRegistrationError>(())
            })();
        if let Err(error) = registration {
            for listener in listeners {
                listener.remove();
            }
            return Err(io::Error::other(error.to_string()));
        }
        let task = tokio::spawn(crate::ui_session::run(
            context.clone(),
            session.clone(),
            submissions,
            feedback_tx,
            validation_tx,
        ));
        let (projections, projection_task) =
            crate::projection::start_feed(context.clone(), changes.clone(), session.clone());
        Ok(Self {
            project_id,
            projections,
            projection_task,
            move_previews: crate::MovePreviewClient::new(
                context.clone(),
                tokio::runtime::Handle::current(),
            ),
            session,
            changes,
            feedback,
            validations,
            saves,
            journal,
            listeners,
            _task: task,
        })
    }
    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
    pub fn sender(&self) -> crate::MdeSession {
        self.session.clone()
    }
    pub fn session(&self) -> crate::MdeSession {
        self.session.clone()
    }
    pub fn projections(&self) -> watch::Receiver<Result<Arc<crate::ModelingSnapshot>, String>> {
        self.projections.clone()
    }
    pub fn move_previews(&self) -> crate::MovePreviewClient {
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
        self.session.shutdown();
        self.projection_task.abort();
        for listener in self.listeners.drain(..) {
            listener.remove();
        }
    }
}
