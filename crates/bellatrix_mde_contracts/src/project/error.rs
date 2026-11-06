use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error(
        "invalid bellatrix-project id: expected 1–128 ASCII bytes using letters, digits, or - _ . + / =; IDs are preserved without normalization"
    )]
    InvalidId,
    #[error("invalid engine namespace")]
    InvalidNamespace,
    #[error("unsupported bellatrix-project schema {0}")]
    Schema(u32),
    #[error("bellatrix-project already open: {0}")]
    AlreadyOpen(String),
    #[error("bellatrix-project not open: {0}")]
    NotOpen(String),
    #[error("revision mismatch (expected {expected}, current {current})")]
    Conflict { expected: u64, current: u64 },
    #[error("a project save is already in progress")]
    SaveInProgress,
    #[error("external save ticket was consumed or is not active")]
    InvalidSaveTicket,
    #[error("written bytes differ from the prepared project snapshot")]
    SaveContentMismatch,
    #[error("revision counter exhausted")]
    RevisionOverflow,
    #[error("bellatrix-project contains unsaved changes: {0}")]
    Unsaved(String),
    #[error(
        "destination belongs to another bellatrix-project or is not a Bellatrix bellatrix-project: {0}"
    )]
    PathConflict(String),
    #[error("destination changed while preparing the save; retry or save to another filename: {0}")]
    DestinationChanged(String),
    #[error("destination already exists; choose a new filename: {0}")]
    DestinationExists(String),
    #[error("bellatrix-project session was closed or replaced: {0}")]
    SessionChanged(String),
    #[error("invalid bellatrix-project file: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}
