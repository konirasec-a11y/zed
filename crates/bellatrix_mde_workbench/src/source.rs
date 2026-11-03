//! A typed, lifecycle-owned source for Zed Items and Panels.
use crate::{MdeSnapshot, from_document};
use bellatrix_mde_cordis::{MdeUiBridge, MdeUiCommand};
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct MdeUiSource {
    pub projects: Arc<ProjectService>,
    pub project_id: ProjectId,
    pub commands: mpsc::Sender<MdeUiCommand>,
    pub revisions: watch::Receiver<u64>,
    pub feedback: watch::Receiver<String>,
    pub validations: watch::Receiver<
        Option<
            Result<
                bellatrix_mde_cordis::DiagramValidationResult,
                bellatrix_mde_cordis::DiagramValidationFailure,
            >,
        >,
    >,
    pub saves: watch::Receiver<Option<u64>>,
    read_only_snapshot: Option<Arc<MdeSnapshot>>,
    read_only_document: Option<Arc<ProjectDocument>>,
    presentation_suspended: Arc<AtomicBool>,
    presentation_preview: Arc<RwLock<Option<String>>>,
}

impl MdeUiSource {
    /// The owner of `bridge` must outlive all Items and Panels.
    pub fn from_bridge(projects: Arc<ProjectService>, bridge: &MdeUiBridge) -> Self {
        Self {
            projects,
            project_id: bridge.project_id().clone(),
            commands: bridge.sender(),
            revisions: bridge.changes(),
            feedback: bridge.feedback(),
            validations: bridge.validations(),
            saves: bridge.saves(),
            read_only_snapshot: None,
            read_only_document: None,
            presentation_suspended: Arc::new(AtomicBool::new(false)),
            presentation_preview: Arc::new(RwLock::new(None)),
        }
    }

    pub fn read_only(
        projects: Arc<ProjectService>,
        document: Arc<ProjectDocument>,
        snapshot: MdeSnapshot,
    ) -> Self {
        let (commands, receiver) = mpsc::channel(1);
        drop(receiver);
        let (_, revisions) = watch::channel(snapshot.revision);
        let (_, feedback) = watch::channel("Read-only native document preview".to_owned());
        let (_, validations) = watch::channel(None);
        let (_, saves) = watch::channel(None);
        Self {
            projects,
            project_id: snapshot.project_id.clone(),
            commands,
            revisions,
            feedback,
            validations,
            saves,
            read_only_snapshot: Some(Arc::new(snapshot)),
            read_only_document: Some(document),
            presentation_suspended: Arc::new(AtomicBool::new(false)),
            presentation_preview: Arc::new(RwLock::new(None)),
        }
    }

    pub fn snapshot_is_current(&self) -> bool {
        self.read_only_document.as_ref().is_none_or(|expected| {
            self.projects
                .snapshot(&self.project_id)
                .is_ok_and(|current| Arc::ptr_eq(expected, &current))
        })
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only_snapshot.is_some() || self.is_presentation_suspended()
    }

    pub(crate) fn for_workspace(&self) -> Self {
        let mut source = self.clone();
        source.presentation_suspended = Arc::new(AtomicBool::new(false));
        source.presentation_preview = Arc::new(RwLock::new(None));
        source
    }

    pub(crate) fn set_preview_context(&self, preview: Option<String>) -> bool {
        let suspended = preview.is_some();
        *self
            .presentation_preview
            .write()
            .expect("MDE presentation lock poisoned") = preview;
        self.presentation_suspended
            .swap(suspended, Ordering::AcqRel)
            != suspended
    }

    pub(crate) fn preview_context(&self) -> Option<String> {
        self.presentation_preview
            .read()
            .expect("MDE presentation lock poisoned")
            .clone()
    }

    pub(crate) fn is_presentation_suspended(&self) -> bool {
        self.presentation_suspended.load(Ordering::Acquire)
    }

    pub fn snapshot(&self) -> Result<MdeSnapshot, String> {
        if let Some(snapshot) = &self.read_only_snapshot {
            return Ok((**snapshot).clone());
        }
        let document = self
            .projects
            .snapshot(&self.project_id)
            .map_err(|error| error.to_string())?;
        from_document(&document)
    }

    pub fn submit(&self, command: MdeUiCommand) -> Result<(), String> {
        if self.is_presentation_suspended() {
            return Err("This Workbench is not the active MDE document. Select its MDE tab before editing or saving its model.".into());
        }
        if self.is_read_only() {
            return Err("Read-only preview: edit the source in the native editor, then close and reopen the preview".into());
        }
        self.commands
            .try_send(command)
            .map_err(|error| error.to_string())
    }
}
