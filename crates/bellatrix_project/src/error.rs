use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("invalid bellatrix-project id")]
    InvalidId,
    #[error("bellatrix-project name must not be empty")]
    EmptyName,
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
    #[error("revision counter exhausted")]
    RevisionOverflow,
    #[error("bellatrix-project contains unsaved changes: {0}")]
    Unsaved(String),
    #[error(
        "destination belongs to another bellatrix-project or is not a Bellatrix bellatrix-project: {0}"
    )]
    PathConflict(String),
    #[error("bellatrix-project session was closed or replaced: {0}")]
    SessionChanged(String),
    #[error("invalid bellatrix-project file: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}
