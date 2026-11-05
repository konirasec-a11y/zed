use crate::{Command, DiagramValidationFailure, DiagramValidationResult, MdeUiCommand};
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use cordis::{Context, Routing, event::QueryOutcome};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{mpsc, watch};

type Completion = watch::Sender<Option<Result<u64, String>>>;
type Validation = Option<Result<DiagramValidationResult, DiagramValidationFailure>>;

#[derive(Clone)]
pub struct MdeSession {
    projects: Arc<ProjectService>,
    project_id: ProjectId,
    identity: Arc<()>,
    submissions: mpsc::Sender<Submission>,
    pending: Arc<AtomicUsize>,
    activity: watch::Sender<()>,
    shutdown: watch::Sender<bool>,
    closed: watch::Sender<bool>,
}

impl MdeSession {
    pub fn try_send(&self, command: MdeUiCommand) -> Result<(), io::Error> {
        self.submit(command).map_err(io::Error::other)
    }
    pub fn submit(&self, command: MdeUiCommand) -> Result<(), String> {
        let ready = (|| {
            if *self.shutdown.borrow() {
                return Err("Document session is closing".to_owned());
            }
            self.snapshot()?;
            let permit = self
                .submissions
                .try_reserve()
                .map_err(|error| error.to_string())?;
            Ok(permit)
        })();
        let permit = match ready {
            Ok(ready) => ready,
            Err(error) => {
                reject(command, &error);
                return Err(error);
            }
        };
        let pending = (!matches!(&command, MdeUiCommand::ValidateDiagram(_))).then(|| {
            self.pending.fetch_add(1, Ordering::AcqRel);
            Pending {
                count: Arc::clone(&self.pending),
                activity: self.activity.clone(),
            }
        });
        let mutating = pending.is_some();
        permit.send(Submission {
            command,
            _pending: pending,
        });
        if mutating {
            self.activity.send_modify(|_| {});
        }
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Arc<ProjectDocument>, String> {
        let document = self
            .projects
            .snapshot(&self.project_id)
            .map_err(|error| error.to_string())?;
        if !Arc::ptr_eq(&document.session_identity(), &self.identity) {
            return Err("The document session was closed or replaced".into());
        }
        Ok(document)
    }
    pub(crate) fn matches_session(&self, identity: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.identity, identity)
    }
    pub fn activity(&self) -> watch::Receiver<()> {
        self.activity.subscribe()
    }
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire) != 0
    }
    pub fn is_closed(&self) -> bool {
        *self.closed.borrow()
    }
    pub fn shutdown(&self) {
        self.shutdown.send_replace(true);
    }
    pub async fn wait_closed(&self) -> Result<(), String> {
        let mut closed = self.closed.subscribe();
        while !*closed.borrow_and_update() {
            closed
                .changed()
                .await
                .map_err(|_| "Document shutdown unavailable".to_owned())?;
        }
        Ok(())
    }
}

pub(crate) struct Submission {
    command: MdeUiCommand,
    _pending: Option<Pending>,
}
struct Pending {
    count: Arc<AtomicUsize>,
    activity: watch::Sender<()>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
        self.activity.send_modify(|_| {});
    }
}
pub(crate) fn create(
    projects: Arc<ProjectService>,
    opened: Arc<ProjectDocument>,
) -> (MdeSession, mpsc::Receiver<Submission>) {
    let (activity, _) = watch::channel(());
    let (submissions, receiver) = mpsc::channel(64);
    let (shutdown, _) = watch::channel(false);
    let (closed, _) = watch::channel(false);
    (
        MdeSession {
            projects,
            project_id: opened.id().clone(),
            identity: opened.session_identity(),
            submissions,
            pending: Arc::new(AtomicUsize::new(0)),
            activity,
            shutdown,
            closed,
        },
        receiver,
    )
}
fn reject(command: MdeUiCommand, reason: &str) {
    match command {
        MdeUiCommand::SetViewLabelTracked { completion, .. }
        | MdeUiCommand::SetViewTimingTracked { completion, .. }
        | MdeUiCommand::SetViewStyleTracked { completion, .. }
        | MdeUiCommand::SetViewRoutingTracked { completion, .. }
        | MdeUiCommand::SetViewPathTracked { completion, .. }
        | MdeUiCommand::ReorderViewTracked { completion, .. }
        | MdeUiCommand::SetPropertyTracked { completion, .. } => {
            completion.send_replace(Some(Err(reason.to_owned())));
        }
        _ => {}
    }
}
fn reject_with_validation(
    command: MdeUiCommand,
    reason: &str,
    validations: &watch::Sender<Validation>,
) {
    if let MdeUiCommand::ValidateDiagram(request) = command {
        validations.send_replace(Some(Err(DiagramValidationFailure {
            diagram_id: request.diagram_id,
            revision: request.expected_revision,
            message: reason.to_owned(),
        })));
    } else {
        reject(command, reason);
    }
}
fn edit(command: MdeUiCommand) -> Result<(ProjectId, u64, Command, Option<Completion>), String> {
    let (command, completion) = match command {
        MdeUiCommand::SetViewLabelTracked {
            request,
            completion,
        } => (MdeUiCommand::SetViewLabel(request), Some(completion)),
        MdeUiCommand::SetViewTimingTracked {
            request,
            completion,
        } => {
            return Ok((
                request.project_id,
                request.expected_revision,
                Command::SetViewTiming {
                    view_id: request.view_id,
                    timing: request.timing,
                },
                Some(completion),
            ));
        }
        MdeUiCommand::SetViewStyleTracked {
            request,
            completion,
        } => (MdeUiCommand::SetViewStyle(request), Some(completion)),
        MdeUiCommand::SetViewRoutingTracked {
            request,
            completion,
        } => (MdeUiCommand::SetViewRouting(request), Some(completion)),
        MdeUiCommand::SetViewPathTracked {
            request,
            completion,
        } => (MdeUiCommand::SetViewPath(request), Some(completion)),
        MdeUiCommand::ReorderViewTracked {
            request,
            completion,
        } => {
            return Ok((
                request.project_id,
                request.expected_revision,
                Command::ReorderView {
                    diagram_id: request.diagram_id,
                    view_id: request.view_id,
                    order: request.order,
                },
                Some(completion),
            ));
        }
        MdeUiCommand::SetPropertyTracked {
            request,
            completion,
        } => (MdeUiCommand::SetProperty(request), Some(completion)),
        command => (command, None),
    };
    let (id, revision, command) = match command {
        MdeUiCommand::SetViewLabel(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetViewLabel {
                view_id: request.view_id,
                label: request.label,
            },
        ),
        MdeUiCommand::SetViewStyle(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetViewStyle {
                diagram_id: request.diagram_id,
                view_id: request.view_id,
                style: request.style,
            },
        ),
        MdeUiCommand::SetViewPath(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetViewPath {
                diagram_id: request.diagram_id,
                view_id: request.view_id,
                geometry: request.geometry,
            },
        ),
        MdeUiCommand::SetViewRouting(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetViewRouting {
                diagram_id: request.diagram_id,
                view_id: request.view_id,
                routing: request.routing,
            },
        ),
        MdeUiCommand::CreateDiagram(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateDiagram {
                kind: request.kind,
                name: request.name,
                model_elements: request.model_elements,
            },
        ),
        MdeUiCommand::RenameDiagram(request) => (
            request.project_id,
            request.expected_revision,
            Command::RenameDiagram {
                diagram_id: request.diagram_id,
                name: request.name,
            },
        ),
        MdeUiCommand::DeleteDiagram(request) => (
            request.project_id,
            request.expected_revision,
            Command::DeleteDiagram {
                diagram_id: request.diagram_id,
            },
        ),
        MdeUiCommand::CreateVisualDraft(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateVisualDraft {
                diagram_id: request.diagram_id,
                notation_kind: request.notation_kind,
                label: request.label,
                geometry: request.geometry,
                properties: request.properties,
                parent_view_id: request.parent_view_id,
                source_view_id: request.source_view_id,
                target_view_id: request.target_view_id,
            },
        ),
        MdeUiCommand::CreateElementOrVisualDraft(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateElementOrVisualDraft {
                diagram_id: request.diagram_id,
                owner_id: request.owner_id,
                kind: request.kind,
                name: request.name,
                x: request.x,
                y: request.y,
            },
        ),
        MdeUiCommand::CreateElementVisualDraft(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateElementVisualDraft {
                diagram_id: request.diagram_id,
                owner_id: request.owner_id,
                kind: request.kind,
                name: request.name,
                x: request.x,
                y: request.y,
            },
        ),
        MdeUiCommand::CreateElement(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateElement {
                diagram_id: request.diagram_id,
                owner_id: request.owner_id,
                kind: request.kind,
                name: request.name,
                x: request.x,
                y: request.y,
            },
        ),
        MdeUiCommand::CreateModelElement(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateModelElement {
                owner_id: request.owner_id,
                kind: request.kind,
                name: request.name,
            },
        ),
        MdeUiCommand::CreateRelationOrVisualDraft(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateRelationOrVisualDraft {
                diagram_id: request.diagram_id,
                owner_id: request.owner_id,
                kind: request.kind,
                from: request.from,
                to: request.to,
            },
        ),
        MdeUiCommand::CreateRelation(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateRelation {
                diagram_id: request.diagram_id,
                owner_id: request.owner_id,
                kind: request.kind,
                from: request.from,
                to: request.to,
            },
        ),
        MdeUiCommand::CreateRelationAtViewsOrVisualDraft(request) => (
            request.project_id,
            request.expected_revision,
            Command::CreateRelationAtViewsOrVisualDraft {
                diagram_id: request.diagram_id,
                owner_id: request.owner_id,
                kind: request.kind,
                source_view_id: request.source_view_id,
                target_view_id: request.target_view_id,
                from: request.from,
                to: request.to,
            },
        ),
        MdeUiCommand::MoveView(request) => (
            request.project_id,
            request.expected_revision,
            Command::MoveView {
                view_id: request.view_id,
                dx: request.dx,
                dy: request.dy,
            },
        ),
        MdeUiCommand::ResizeView(request) => (
            request.project_id,
            request.expected_revision,
            Command::ResizeView {
                view_id: request.view_id,
                width: request.width,
                height: request.height,
            },
        ),
        MdeUiCommand::RemoveView(request) => (
            request.project_id,
            request.expected_revision,
            Command::RemoveView {
                diagram_id: request.diagram_id,
                view_id: request.view_id,
            },
        ),
        MdeUiCommand::PlaceElementView(request) => (
            request.project_id,
            request.expected_revision,
            Command::PlaceElementView {
                diagram_id: request.diagram_id,
                element_id: request.element_id,
                x: request.x,
                y: request.y,
            },
        ),
        MdeUiCommand::SetOwner(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetOwner {
                element_id: request.element_id,
                owner_id: request.owner_id,
            },
        ),
        MdeUiCommand::SetExplorerParent(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetExplorerParent {
                subject_id: request.subject_id,
                parent_id: request.parent_id,
                parent_occurrence: request.parent_occurrence,
            },
        ),
        MdeUiCommand::DeleteElement(request) => (
            request.project_id,
            request.expected_revision,
            Command::DeleteElement {
                element_id: request.element_id,
            },
        ),
        MdeUiCommand::SetProperty(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetProperty {
                element_id: request.element_id,
                path: request.path,
                value: request.value,
            },
        ),
        MdeUiCommand::AddMember(request) => (
            request.project_id,
            request.expected_revision,
            Command::AddMember {
                owner_id: request.owner_id,
                kind: request.kind,
            },
        ),
        MdeUiCommand::MoveParameter(request) => (
            request.project_id,
            request.expected_revision,
            Command::MoveParameter {
                parameter_id: request.parameter_id,
                up: request.up,
            },
        ),
        MdeUiCommand::QuickEdit(request) => (
            request.project_id,
            request.expected_revision,
            Command::QuickEdit {
                diagram_id: request.diagram_id,
                element_id: request.element_id,
                view_id: request.view_id,
                action: request.action,
            },
        ),
        MdeUiCommand::SetNameExpression(request) => (
            request.project_id,
            request.expected_revision,
            Command::SetNameExpression {
                element_id: request.element_id,
                expression: request.expression,
            },
        ),
        MdeUiCommand::Rename(request) => (
            request.project_id,
            request.expected_revision,
            Command::Rename {
                element_id: request.element_id,
                name: request.new_name,
            },
        ),
        MdeUiCommand::CreateExplorerDiagram(request) => (
            request.diagram.project_id,
            request.diagram.expected_revision,
            Command::CreateExplorerDiagram {
                kind: request.diagram.kind,
                name: request.diagram.name,
                model_elements: request.diagram.model_elements,
                parent_id: request.parent_id,
                parent_occurrence: request.parent_occurrence,
            },
        ),
        command => {
            reject(command, "This request is not a model mutation");
            return Err("This request is not a model mutation".into());
        }
    };
    Ok((id, revision, command, completion))
}

pub(crate) async fn run(
    context: Context,
    session: MdeSession,
    mut submissions: mpsc::Receiver<Submission>,
    feedback: watch::Sender<String>,
    validations: watch::Sender<Validation>,
) {
    let mut shutdown = session.shutdown.subscribe();
    loop {
        if *shutdown.borrow() {
            break;
        }
        let submission = tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            submission = submissions.recv() => match submission { Some(submission) => submission, None => break },
        };
        let Submission { command, _pending } = submission;
        // The session identity is stable, while queued native commands may target consecutive revisions.
        // Bind each application to the snapshot current when execution begins, then commit with CAS.
        let document = match session.snapshot() {
            Ok(document) => document,
            Err(error) => {
                reject_with_validation(command, &error, &validations);
                feedback.send_replace(error);
                continue;
            }
        };
        if let MdeUiCommand::ValidateDiagram(request) = command {
            let result = if request.project_id != *document.id()
                || request.expected_revision != document.revision()
            {
                Err("Validation request belongs to another document revision".to_owned())
            } else {
                match context
                    .query::<crate::ModelingValidateSnapshot>(
                        Routing::Unscoped,
                        crate::ModelingValidateSnapshotRequested {
                            document,
                            diagram_id: request.diagram_id.clone(),
                        },
                    )
                    .await
                {
                    Ok(QueryOutcome::Answer(result)) => result,
                    Ok(QueryOutcome::Miss) => Err("Validation service unavailable".into()),
                    Err(error) => Err(error.to_string()),
                }
            };
            validations.send_replace(Some(result.map_err(|message| DiagramValidationFailure {
                diagram_id: request.diagram_id,
                revision: request.expected_revision,
                message,
            })));
            continue;
        }
        if let MdeUiCommand::SaveProject(request) = command {
            let result = if !Arc::ptr_eq(&document, &request.document) {
                Err("Save request belongs to another document snapshot".to_owned())
            } else {
                match context
                    .query::<bellatrix_project::ProjectSave>(Routing::Unscoped, request)
                    .await
                {
                    Ok(QueryOutcome::Answer(result)) => result.map_err(|error| error.to_string()),
                    Ok(QueryOutcome::Miss) => Err("Project save service unavailable".into()),
                    Err(error) => Err(error.to_string()),
                }
            };
            feedback.send_replace(match result {
                Ok(revision) => format!("Project saved: revision {revision}"),
                Err(error) => format!("Project save failed: {error}"),
            });
            continue;
        }
        let result = match edit(command) {
            Ok((id, revision, command, completion)) => {
                let result = if id != *document.id() || revision != document.revision() {
                    Err("Command belongs to another document revision".to_owned())
                } else {
                    crate::mutation::execute_snapshot(
                        &context,
                        &session.projects,
                        document,
                        command,
                    )
                    .await
                    .map(|document| document.revision())
                };
                if let Some(completion) = completion {
                    completion.send_replace(Some(result.clone()));
                }
                result
            }
            Err(error) => Err(error),
        };
        feedback.send_replace(match result {
            Ok(revision) => format!("Edit confirmed: revision {revision}"),
            Err(error) => format!("Edit rejected: {error}"),
        });
    }
    submissions.close();
    while let Some(submission) = submissions.recv().await {
        reject_with_validation(
            submission.command,
            "Document closed before the command was applied",
            &validations,
        );
    }
    session.closed.send_replace(true);
    session.activity.send_modify(|_| {});
}
