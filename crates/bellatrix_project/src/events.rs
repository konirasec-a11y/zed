use crate::{ProjectDocument, ProjectError, ProjectId, ProjectPublishedFile, ProjectSaveTicket};
use cordis::Event;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

/// Request/response operations are public Cordis Events, not an alternate bus.
#[derive(Clone)]
pub struct ProjectCreateRequested {
    pub document: ProjectDocument,
}
pub struct ProjectCreate;
impl Event for ProjectCreate {
    const NAME: &'static str = "bellatrix.bellatrix-project.create.v1";
    type Args = ProjectCreateRequested;
    type Output = Result<Arc<ProjectDocument>, ProjectError>;
}

#[derive(Clone)]
pub struct ProjectOpenRequested {
    pub path: PathBuf,
}
pub struct ProjectOpen;
impl Event for ProjectOpen {
    const NAME: &'static str = "bellatrix.bellatrix-project.open.v1";
    type Args = ProjectOpenRequested;
    type Output = Result<Arc<ProjectDocument>, ProjectError>;
}

#[derive(Clone)]
pub struct ProjectReplaceRequested {
    pub id: ProjectId,
    pub namespace: String,
    pub expected_revision: u64,
    pub state: Value,
}
pub struct ProjectReplace;
impl Event for ProjectReplace {
    const NAME: &'static str = "bellatrix.bellatrix-project.replace.v1";
    type Args = ProjectReplaceRequested;
    type Output = Result<Arc<ProjectDocument>, ProjectError>;
}

/// Compare the exact immutable snapshot under the replacement lock. This also
/// rejects commands prepared before a project was closed and reopened at revision zero.
#[derive(Clone)]
pub struct ProjectReplaceSnapshotRequested {
    pub document: Arc<ProjectDocument>,
    pub namespace: String,
    pub state: Value,
}
pub struct ProjectReplaceSnapshot;
impl Event for ProjectReplaceSnapshot {
    const NAME: &'static str = "bellatrix.bellatrix-project.replace-snapshot.v1";
    type Args = ProjectReplaceSnapshotRequested;
    type Output = Result<Arc<ProjectDocument>, ProjectError>;
}

#[derive(Clone)]
pub struct ProjectSaveRequested {
    pub document: Arc<ProjectDocument>,
    pub path: PathBuf,
}
pub struct ProjectSave;
impl Event for ProjectSave {
    const NAME: &'static str = "bellatrix.bellatrix-project.save.v1";
    type Args = ProjectSaveRequested;
    type Output = Result<u64, ProjectError>;
}

#[derive(Clone)]
pub struct ProjectCloseRequested {
    pub id: ProjectId,
    pub discard_unsaved: bool,
}
pub struct ProjectClose;
impl Event for ProjectClose {
    const NAME: &'static str = "bellatrix.bellatrix-project.close.v1";
    type Args = ProjectCloseRequested;
    type Output = Result<(), ProjectError>;
}

#[derive(Clone, Debug)]
pub struct ProjectOpened {
    pub document: Arc<ProjectDocument>,
    pub from_disk: bool,
}
pub struct ProjectOpenedEvent;
impl Event for ProjectOpenedEvent {
    const NAME: &'static str = "bellatrix.bellatrix-project.opened.v1";
    type Args = ProjectOpened;
    type Output = ();
}

#[derive(Clone, Debug)]
pub struct ProjectStateChanged {
    pub id: ProjectId,
    pub namespace: String,
    pub previous_revision: u64,
    pub revision: u64,
    pub state: Arc<Value>,
}
pub struct ProjectStateChangedEvent;
impl Event for ProjectStateChangedEvent {
    const NAME: &'static str = "bellatrix.bellatrix-project.state-changed.v1";
    type Args = ProjectStateChanged;
    type Output = ();
}

#[derive(Clone, Debug)]
pub struct ProjectSaved {
    pub document: Arc<ProjectDocument>,
    pub id: ProjectId,
    pub revision: u64,
    pub path: PathBuf,
}
pub struct ProjectSavedEvent;
impl Event for ProjectSavedEvent {
    const NAME: &'static str = "bellatrix.bellatrix-project.saved.v1";
    type Args = ProjectSaved;
    type Output = ();
}

#[derive(Clone, Debug)]
pub struct ProjectClosed {
    pub id: ProjectId,
    pub session: Arc<()>,
}
pub struct ProjectClosedEvent;
impl Event for ProjectClosedEvent {
    const NAME: &'static str = "bellatrix.bellatrix-project.closed.v1";
    type Args = ProjectClosed;
    type Output = ();
}

/// Import already-loaded bytes without doing file I/O or replacing an open identity.
#[derive(Clone)]
pub struct ProjectImportRequested {
    pub document: ProjectDocument,
    /// True only when the complete native document was loaded from persisted contents.
    pub is_saved: bool,
}
pub struct ProjectImport;
impl Event for ProjectImport {
    const NAME: &'static str = "bellatrix.bellatrix-project.import.v1";
    type Args = ProjectImportRequested;
    type Output = Result<Arc<ProjectDocument>, ProjectError>;
}

/// Reserve one write for this open document session; other saves and close of this
/// same project are rejected until acknowledgement or every ticket clone is dropped.
/// This is not a destination lock: paths remain opaque, with no filesystem lookup
/// or canonicalization. The adapter must coordinate destination ownership across
/// projects and native/CLI writers, including symlinks and path aliases, and check
/// native buffer conflicts before I/O. Do not treat a ticket as permission to
/// overwrite an existing destination.
#[derive(Clone)]
pub struct ProjectPrepareExternalSaveRequested {
    pub id: ProjectId,
    pub expected_revision: u64,
    pub path: PathBuf,
}
pub struct ProjectPrepareExternalSave;
impl Event for ProjectPrepareExternalSave {
    const NAME: &'static str = "bellatrix.bellatrix-project.prepare-external-save.v1";
    type Args = ProjectPrepareExternalSaveRequested;
    type Output = Result<ProjectSaveTicket, ProjectError>;
}

/// Reserve a verified external encoding of this exact snapshot. The platform
/// treats engine sections and file encodings as opaque: the adapter must prove
/// the complete document is representable before calling this operation.
/// This does not write, authorize overwriting a destination, or mark anything saved.
#[derive(Clone)]
pub struct ProjectPrepareEncodedSaveRequested {
    pub document: Arc<ProjectDocument>,
    pub path: PathBuf,
    pub bytes: Arc<[u8]>,
}
pub struct ProjectPrepareEncodedSave;
impl Event for ProjectPrepareEncodedSave {
    const NAME: &'static str = "bellatrix.bellatrix-project.prepare-encoded-save.v1";
    type Args = ProjectPrepareEncodedSaveRequested;
    type Output = Result<ProjectSaveTicket, ProjectError>;
}

/// Publish reserved bytes under an absent filename. Existing files, directories
/// and symlinks are never replaced. Publication itself does not clear dirty state.
#[derive(Clone)]
pub struct ProjectPublishNewFileRequested {
    pub ticket: ProjectSaveTicket,
}
pub struct ProjectPublishNewFile;
impl Event for ProjectPublishNewFile {
    const NAME: &'static str = "bellatrix.bellatrix-project.publish-new-file.v1";
    type Args = ProjectPublishNewFileRequested;
    type Output = Result<ProjectPublishedFile, ProjectError>;
}

/// Submit only after the adapter has completed its native write successfully.
/// Bytes are an exact receipt, not proof of disk I/O; the adapter owns that guarantee.
/// A valid-session acknowledgement is consumed even on byte/revision mismatch;
/// those failures leave the document dirty and emit no saved notification.
/// Stop or finish I/O before acknowledging or dropping the last ticket clone.
#[derive(Clone)]
pub struct ProjectAcknowledgeExternalSaveRequested {
    pub ticket: ProjectSaveTicket,
    pub written_bytes: Arc<[u8]>,
}
pub struct ProjectAcknowledgeExternalSave;
impl Event for ProjectAcknowledgeExternalSave {
    const NAME: &'static str = "bellatrix.bellatrix-project.acknowledge-external-save.v1";
    type Args = ProjectAcknowledgeExternalSaveRequested;
    type Output = Result<u64, ProjectError>;
}

/// Release a document only if the exact captured snapshot is still open.
/// Changed or reopened identities are preserved; this never performs file I/O.
#[derive(Clone)]
pub struct ProjectCloseSnapshotRequested {
    pub document: Arc<ProjectDocument>,
}
pub struct ProjectCloseSnapshot;
impl Event for ProjectCloseSnapshot {
    const NAME: &'static str = "bellatrix.bellatrix-project.close-snapshot.v1";
    type Args = ProjectCloseSnapshotRequested;
    type Output = Result<(), ProjectError>;
}
