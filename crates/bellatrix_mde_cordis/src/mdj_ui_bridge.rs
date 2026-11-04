//! Restricted native MDJ transport. Generic mutation and save events are never dispatched here.
use crate::MdjEditCommand as Command;
use crate::{
    DiagramValidationFailure, DiagramValidationResult, MdeUiCommand, ModelingApplyMdjEditRequested,
    mdj_interchange::ModelingApplyMdjEditSnapshot,
};
use bellatrix_project::{ProjectDocument, ProjectId};
use cordis::{Context, Routing, event::QueryOutcome};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{mpsc, watch};

type Completion = watch::Sender<Option<Result<u64, String>>>;
type Validation = Option<Result<DiagramValidationResult, DiagramValidationFailure>>;

/// The exact imported snapshot and its confirmed successors. Clones share identity,
/// not a second model. External consumers can read but cannot replace the snapshot.
#[derive(Clone)]
pub struct MdjSession {
    document: Arc<RwLock<Arc<ProjectDocument>>>,
    closed: watch::Receiver<bool>,
    close_signal: watch::Sender<bool>,
    submissions: mpsc::Sender<(MdeUiCommand, Pending)>,
    pending: Arc<AtomicUsize>,
    activity: watch::Sender<u64>,
}

impl MdjSession {
    pub(crate) fn new(
        document: Arc<ProjectDocument>,
        submissions: mpsc::Sender<(MdeUiCommand, Pending)>,
        activity: watch::Sender<u64>,
    ) -> Self {
        let (close_signal, closed) = watch::channel(false);
        Self {
            document: Arc::new(RwLock::new(document)),
            closed,
            close_signal,
            submissions,
            pending: Arc::new(AtomicUsize::new(0)),
            activity,
        }
    }

    /// The only MDJ submission route: dirty/close guards include queued commands.
    pub fn submit(&self, command: MdeUiCommand) -> Result<(), String> {
        let permit = match self.submissions.try_reserve() {
            Ok(permit) => permit,
            Err(error) => {
                let message = error.to_string();
                reject(command, &message);
                return Err(message);
            }
        };
        self.pending.fetch_add(1, Ordering::AcqRel);
        permit.send((
            command,
            Pending {
                count: Arc::clone(&self.pending),
                activity: self.activity.clone(),
            },
        ));
        self.activity.send_modify(|_| {});
        Ok(())
    }

    /// Includes queued and in-flight commands submitted through `submit`.
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire) != 0
    }

    pub fn snapshot(&self) -> Result<Arc<ProjectDocument>, String> {
        self.document
            .read()
            .map(|document| Arc::clone(&document))
            .map_err(|_| "MDJ session snapshot lock poisoned".into())
    }

    /// True only after the in-flight command finished and queued work was rejected.
    pub fn is_closed(&self) -> bool {
        *self.closed.borrow()
    }

    /// Drop the owning bridge, then await this before taking a snapshot for cleanup.
    pub async fn wait_closed(&self) -> Result<(), String> {
        let mut closed = self.closed.clone();
        loop {
            if *closed.borrow_and_update() {
                return Ok(());
            }
            closed
                .changed()
                .await
                .map_err(|_| "MDJ session shutdown unavailable".to_owned())?;
        }
    }

    fn expected(&self, id: &ProjectId, revision: u64) -> Result<Arc<ProjectDocument>, String> {
        let document = self.snapshot()?;
        if document.id() != id {
            return Err("MDJ command belongs to another project".into());
        }
        if document.revision() != revision {
            return Err("MDJ command revision is stale".into());
        }
        Ok(document)
    }

    fn confirm(
        &self,
        expected: &Arc<ProjectDocument>,
        document: Arc<ProjectDocument>,
    ) -> Result<u64, String> {
        let mut current = self
            .document
            .write()
            .map_err(|_| "MDJ session snapshot lock poisoned")?;
        if !Arc::ptr_eq(&current, expected) {
            return Err("MDJ session snapshot changed before confirmation".into());
        }
        if document.id() != expected.id() || document.revision() < expected.revision() {
            return Err("MDJ responder returned an invalid snapshot".into());
        }
        let revision = document.revision();
        *current = document;
        Ok(revision)
    }
}

pub(crate) struct Pending {
    count: Arc<AtomicUsize>,
    activity: watch::Sender<u64>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
        // Wake dirty/close observers even on rejection or a no-op revision.
        self.activity.send_modify(|_| {});
    }
}

struct Edit {
    id: ProjectId,
    revision: u64,
    command: Command,
    completion: Option<Completion>,
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

fn edit(command: MdeUiCommand) -> Result<Edit, String> {
    let (command, completion) = match command {
        MdeUiCommand::SetViewPathTracked {
            request,
            completion,
        } => (MdeUiCommand::SetViewPath(request), Some(completion)),
        MdeUiCommand::SetViewLabelTracked {
            request,
            completion,
        } => (MdeUiCommand::SetViewLabel(request), Some(completion)),
        MdeUiCommand::SetViewStyleTracked {
            request,
            completion,
        } => (MdeUiCommand::SetViewStyle(request), Some(completion)),
        MdeUiCommand::SetPropertyTracked {
            request,
            completion,
        } => (MdeUiCommand::SetProperty(request), Some(completion)),
        command => (command, None),
    };
    let (id, revision, command) = match command {
        MdeUiCommand::SetViewPath(r) => (
            r.project_id,
            r.expected_revision,
            Command::SetViewPath {
                diagram_id: r.diagram_id,
                view_id: r.view_id,
                geometry: r.geometry,
            },
        ),
        MdeUiCommand::SetViewLabel(r) => (
            r.project_id,
            r.expected_revision,
            Command::SetViewLabel {
                view_id: r.view_id,
                label: r.label,
            },
        ),
        MdeUiCommand::SetViewStyle(r) => (
            r.project_id,
            r.expected_revision,
            Command::SetViewStyle {
                diagram_id: r.diagram_id,
                view_id: r.view_id,
                style: r.style,
            },
        ),
        MdeUiCommand::Rename(r) => (
            r.project_id,
            r.expected_revision,
            Command::Rename {
                element_id: r.element_id,
                name: r.new_name,
            },
        ),
        MdeUiCommand::RenameDiagram(r) => (
            r.project_id,
            r.expected_revision,
            Command::RenameDiagram {
                diagram_id: r.diagram_id,
                name: r.name,
            },
        ),
        MdeUiCommand::SetProperty(r) => (
            r.project_id,
            r.expected_revision,
            Command::SetProperty {
                element_id: r.element_id,
                path: r.path,
                value: r.value,
            },
        ),
        MdeUiCommand::SaveProject(_) => {
            return Err(
                "MDJ documents require verified native save; generic ProjectSave is disabled"
                    .into(),
            );
        }
        command => {
            let reason = "This command is not enabled for lossless MDJ editing";
            reject(command, reason);
            return Err(reason.into());
        }
    };
    Ok(Edit {
        id,
        revision,
        command,
        completion,
    })
}

async fn apply(context: &Context, session: &MdjSession, edit: &Edit) -> Result<u64, String> {
    let document = session.expected(&edit.id, edit.revision)?;
    let result = context
        .query::<ModelingApplyMdjEditSnapshot>(
            Routing::Unscoped,
            ModelingApplyMdjEditRequested {
                document: Arc::clone(&document),
                command: edit.command.clone(),
            },
        )
        .await;
    match result {
        Ok(QueryOutcome::Answer(Ok(updated))) => session.confirm(&document, updated),
        Ok(QueryOutcome::Answer(Err(error))) => Err(error),
        Ok(QueryOutcome::Miss) => Err("Cordis MDJ responder unavailable".into()),
        Err(error) => Err(format!("Cordis MDJ dispatch failed: {error}")),
    }
}

pub(crate) async fn run(
    context: Context,
    session: MdjSession,
    mut submissions: mpsc::Receiver<(MdeUiCommand, Pending)>,
    mut shutdown: watch::Receiver<bool>,
    revisions: watch::Sender<u64>,
    feedback: watch::Sender<String>,
    validations: watch::Sender<Validation>,
) {
    loop {
        // Shutdown never cancels an in-flight query: capture its confirmed Arc first.
        let (command, _pending) = tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            command = submissions.recv() => match command { Some(command) => command, None => break },
        };
        if let MdeUiCommand::ValidateDiagram(request) = command {
            // The generic validator resolves by ID/revision, which cannot distinguish
            // a closed and reopened project. Do not report another session's result.
            let message =
                "MDJ validation is disabled until a snapshot-bound validator is available"
                    .to_owned();
            validations.send_replace(Some(Err(DiagramValidationFailure {
                diagram_id: request.diagram_id,
                revision: request.expected_revision,
                message: message.clone(),
            })));
            feedback.send_replace(message);
            continue;
        }
        let result = match edit(command) {
            Ok(edit) => {
                let result = apply(&context, &session, &edit).await;
                if let Ok(revision) = &result {
                    revisions.send_replace(*revision);
                }
                if let Some(completion) = edit.completion {
                    completion.send_replace(Some(result.clone()));
                }
                result
            }
            Err(error) => Err(error),
        };
        feedback.send_replace(match result {
            Ok(revision) => format!("MDJ edit confirmed: revision {revision}"),
            Err(error) => format!("MDJ edit rejected: {error}"),
        });
    }
    submissions.close();
    while let Some((command, _pending)) = submissions.recv().await {
        let message = "MDJ session closed before the command was applied";
        if let MdeUiCommand::ValidateDiagram(request) = command {
            validations.send_replace(Some(Err(DiagramValidationFailure {
                diagram_id: request.diagram_id,
                revision: request.expected_revision,
                message: message.into(),
            })));
        } else {
            reject(command, message);
        }
    }
    session.close_signal.send_replace(true);
}
