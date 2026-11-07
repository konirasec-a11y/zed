use super::{ProjectAcknowledgeExternalSaveRequested, ProjectDocument};
use std::{
    any::Any,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

/// A single external write reservation. Drop every clone after cancellation or failure.
/// The adapter must finish or cancel its I/O before releasing the reservation.
/// Reservation scope is one project/session, not its filesystem destination.
#[derive(Clone)]
pub struct ProjectSaveTicket {
    document: Arc<ProjectDocument>,
    bytes: Arc<[u8]>,
    path: PathBuf,
    lease: Arc<dyn Any + Send + Sync>,
}

impl fmt::Debug for ProjectSaveTicket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProjectSaveTicket")
            .field("document", &self.document)
            .field("bytes", &self.bytes)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl ProjectSaveTicket {
    /// Transport a runtime-owned lease without exposing its reservation state.
    /// Constructing a transport does not grant save authority: the service must
    /// downcast to its private lease type and validate the live reservation.
    pub fn from_runtime<L: Any + Send + Sync>(
        document: Arc<ProjectDocument>,
        bytes: Arc<[u8]>,
        path: PathBuf,
        lease: L,
    ) -> Self {
        Self {
            document,
            bytes,
            path,
            lease: Arc::new(lease),
        }
    }

    /// Only the issuing runtime knows the private lease type. Returning a borrow
    /// prevents extracting the lease to attach it to a different snapshot.
    pub fn runtime_lease<L: Any + Send + Sync>(&self) -> Option<&L> {
        self.lease.downcast_ref()
    }

    pub fn document(&self) -> &Arc<ProjectDocument> {
        &self.document
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Receipt of publication. A later model change can still make acknowledgement
/// fail; retain the published path then. The underlying runtime lease remains
/// the authority for acknowledgement, rather than this transport wrapper.
#[derive(Clone, Debug)]
pub struct ProjectPublishedFile {
    ticket: ProjectSaveTicket,
    cleanup_warning: Option<String>,
}

impl ProjectPublishedFile {
    pub fn from_published(ticket: ProjectSaveTicket, cleanup_warning: Option<String>) -> Self {
        Self {
            ticket,
            cleanup_warning,
        }
    }

    pub fn path(&self) -> &Path {
        self.ticket.path()
    }
    pub fn document(&self) -> &Arc<ProjectDocument> {
        self.ticket.document()
    }
    pub fn cleanup_warning(&self) -> Option<&str> {
        self.cleanup_warning.as_deref()
    }
    pub fn acknowledge(&self) -> ProjectAcknowledgeExternalSaveRequested {
        ProjectAcknowledgeExternalSaveRequested {
            ticket: self.ticket.clone(),
            written_bytes: Arc::clone(&self.ticket.bytes),
        }
    }
}
