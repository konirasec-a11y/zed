use crate::{ProjectDocument, ProjectError, ProjectId};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, Weak};

struct OpenProject {
    document: Arc<ProjectDocument>,
    saved_revision: Option<u64>,
    save_lock: Arc<Mutex<()>>,
    external_save: Weak<()>,
}

/// A single external write reservation. Drop every clone after cancellation or failure.
/// The adapter must finish or cancel its I/O before releasing the reservation.
/// Reservation scope is one project/session, not its filesystem destination.
#[derive(Clone, Debug)]
pub struct ProjectSaveTicket {
    document: Arc<ProjectDocument>,
    bytes: Arc<[u8]>,
    path: PathBuf,
    session: Arc<Mutex<()>>,
    reservation: Arc<()>,
}

impl ProjectSaveTicket {
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

#[derive(Default)]
pub struct ProjectService {
    projects: RwLock<BTreeMap<ProjectId, OpenProject>>,
    path_locks: Mutex<BTreeMap<std::path::PathBuf, Arc<Mutex<()>>>>,
}

impl cordis::Service for ProjectService {
    const NAME: &'static str = "bellatrix/bellatrix-project-service/v1";
}

impl ProjectService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Explicit read-only coeffect: cannot mutate a bellatrix-project behind Cordis.
    pub fn snapshot(&self, id: &ProjectId) -> Result<Arc<ProjectDocument>, ProjectError> {
        self.projects
            .read()
            .expect("bellatrix-project lock poisoned")
            .get(id)
            .map(|p| Arc::clone(&p.document))
            .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))
    }

    pub fn list(&self) -> Vec<Arc<ProjectDocument>> {
        self.projects
            .read()
            .expect("bellatrix-project lock poisoned")
            .values()
            .map(|p| Arc::clone(&p.document))
            .collect()
    }

    pub fn is_dirty(&self, id: &ProjectId) -> Result<bool, ProjectError> {
        let map = self
            .projects
            .read()
            .expect("bellatrix-project lock poisoned");
        let project = map
            .get(id)
            .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?;
        Ok(project.saved_revision != Some(project.document.revision()))
    }

    pub(crate) fn open(
        &self,
        document: ProjectDocument,
        from_disk: bool,
    ) -> Result<Arc<ProjectDocument>, ProjectError> {
        document.validate()?;
        let mut projects = self
            .projects
            .write()
            .expect("bellatrix-project lock poisoned");
        if projects.contains_key(document.id()) {
            return Err(ProjectError::AlreadyOpen(document.id().as_str().into()));
        }
        let saved_revision = from_disk.then_some(document.revision());
        let doc = Arc::new(document);
        projects.insert(
            doc.id().clone(),
            OpenProject {
                document: Arc::clone(&doc),
                saved_revision,
                save_lock: Arc::new(Mutex::new(())),
                external_save: Weak::new(),
            },
        );
        Ok(doc)
    }

    pub(crate) fn prepare_external_save(
        &self,
        id: &ProjectId,
        expected_revision: u64,
        path: PathBuf,
    ) -> Result<ProjectSaveTicket, ProjectError> {
        let session = {
            let projects = self
                .projects
                .read()
                .expect("bellatrix-project lock poisoned");
            Arc::clone(
                &projects
                    .get(id)
                    .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?
                    .save_lock,
            )
        };
        // Never block the Cordis executor behind a disk writer.
        let _guard = session
            .try_lock()
            .map_err(|_| ProjectError::SaveInProgress)?;
        let mut projects = self
            .projects
            .write()
            .expect("bellatrix-project lock poisoned");
        let current = projects
            .get_mut(id)
            .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?;
        if !Arc::ptr_eq(&current.save_lock, &session) {
            return Err(ProjectError::SessionChanged(id.as_str().into()));
        }
        if current.external_save.upgrade().is_some() {
            return Err(ProjectError::SaveInProgress);
        }
        if current.document.revision() != expected_revision {
            return Err(ProjectError::Conflict {
                expected: expected_revision,
                current: current.document.revision(),
            });
        }
        let bytes = serde_json::to_vec_pretty(&*current.document)?.into();
        let reservation = Arc::new(());
        current.external_save = Arc::downgrade(&reservation);
        Ok(ProjectSaveTicket {
            document: Arc::clone(&current.document),
            bytes,
            path,
            session: Arc::clone(&session),
            reservation,
        })
    }

    pub(crate) fn acknowledge_external_save(
        &self,
        ticket: &ProjectSaveTicket,
        written_bytes: &[u8],
    ) -> Result<u64, ProjectError> {
        let id = ticket.document.id();
        let mut projects = self
            .projects
            .write()
            .expect("bellatrix-project lock poisoned");
        let current = projects
            .get_mut(id)
            .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?;
        if !Arc::ptr_eq(&current.save_lock, &ticket.session) {
            return Err(ProjectError::SessionChanged(id.as_str().into()));
        }
        if !current
            .external_save
            .upgrade()
            .is_some_and(|reservation| Arc::ptr_eq(&reservation, &ticket.reservation))
        {
            return Err(ProjectError::InvalidSaveTicket);
        }
        // Completion consumes the reservation even when native bytes or semantic state diverged.
        current.external_save = Weak::new();
        current.saved_revision = None;
        if written_bytes != ticket.bytes() {
            return Err(ProjectError::SaveContentMismatch);
        }
        if current.document.revision() != ticket.document.revision() {
            return Err(ProjectError::Conflict {
                expected: ticket.document.revision(),
                current: current.document.revision(),
            });
        }
        current.saved_revision = Some(ticket.document.revision());
        Ok(ticket.document.revision())
    }

    pub(crate) fn replace(
        &self,
        id: &ProjectId,
        namespace: String,
        expected: u64,
        value: Value,
    ) -> Result<Option<Arc<ProjectDocument>>, ProjectError> {
        let mut projects = self
            .projects
            .write()
            .expect("bellatrix-project lock poisoned");
        let current = projects
            .get_mut(id)
            .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?;
        if expected != current.document.revision() {
            return Err(ProjectError::Conflict {
                expected,
                current: current.document.revision(),
            });
        }
        let mut next = (*current.document).clone();
        if !next.replace(namespace, value)? {
            return Ok(None);
        }
        current.document = Arc::new(next);
        Ok(Some(Arc::clone(&current.document)))
    }

    /// A close waits for any in-flight save of *this* bellatrix-project to finish.
    /// The Cordis responder executes it on Tokio's blocking pool.
    pub(crate) fn close(&self, id: &ProjectId, discard_unsaved: bool) -> Result<(), ProjectError> {
        self.close_matching(id, discard_unsaved, None)
    }

    pub(crate) fn close_snapshot(
        &self,
        document: &Arc<ProjectDocument>,
    ) -> Result<(), ProjectError> {
        self.close_matching(document.id(), true, Some(document))
    }

    fn close_matching(
        &self,
        id: &ProjectId,
        discard_unsaved: bool,
        expected: Option<&Arc<ProjectDocument>>,
    ) -> Result<(), ProjectError> {
        let save_lock = {
            let projects = self
                .projects
                .read()
                .expect("bellatrix-project lock poisoned");
            Arc::clone(
                &projects
                    .get(id)
                    .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?
                    .save_lock,
            )
        };
        let _guard = save_lock
            .lock()
            .expect("bellatrix-project save lock poisoned");
        let mut projects = self
            .projects
            .write()
            .expect("bellatrix-project lock poisoned");
        let current = projects
            .get(id)
            .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?;
        if !Arc::ptr_eq(&current.save_lock, &save_lock) {
            return Err(ProjectError::SessionChanged(id.as_str().into()));
        }
        if expected.is_some_and(|document| !Arc::ptr_eq(document, &current.document)) {
            return Err(ProjectError::SessionChanged(id.as_str().into()));
        }
        if current.external_save.upgrade().is_some() {
            return Err(ProjectError::SaveInProgress);
        }
        if !discard_unsaved && current.saved_revision != Some(current.document.revision()) {
            return Err(ProjectError::Unsaved(id.as_str().into()));
        }
        projects.remove(id);
        Ok(())
    }

    /// Called from spawn_blocking, never while polling Cordis apply or GPUI.
    pub(crate) fn save(&self, id: &ProjectId, path: &Path) -> Result<u64, ProjectError> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let name = path
            .file_name()
            .ok_or_else(|| ProjectError::PathConflict(path.display().to_string()))?;
        // Canonical parent makes `dir/../file` and `file` use the same lock.
        let path_key = parent.canonicalize()?.join(name);
        let path_lock = {
            let mut locks = self
                .path_locks
                .lock()
                .expect("bellatrix-project path lock poisoned");
            Arc::clone(
                locks
                    .entry(path_key)
                    .or_insert_with(|| Arc::new(Mutex::new(()))),
            )
        };
        let save_lock = {
            let map = self
                .projects
                .read()
                .expect("bellatrix-project lock poisoned");
            Arc::clone(
                &map.get(id)
                    .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?
                    .save_lock,
            )
        };
        let _project_guard = save_lock
            .lock()
            .expect("bellatrix-project save lock poisoned");
        let _path_guard = path_lock
            .lock()
            .expect("bellatrix-project path lock poisoned");
        let document = {
            let map = self
                .projects
                .read()
                .expect("bellatrix-project lock poisoned");
            let current = map
                .get(id)
                .ok_or_else(|| ProjectError::NotOpen(id.as_str().into()))?;
            if !Arc::ptr_eq(&current.save_lock, &save_lock) {
                return Err(ProjectError::SessionChanged(id.as_str().into()));
            }
            if current.external_save.upgrade().is_some() {
                return Err(ProjectError::SaveInProgress);
            }
            Arc::clone(&current.document)
        };
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(ProjectError::PathConflict(path.display().to_string()));
            }
            Ok(_) => {
                let previous = crate::file::load(path)
                    .map_err(|_| ProjectError::PathConflict(path.display().to_string()))?;
                if previous.id() != id {
                    return Err(ProjectError::PathConflict(path.display().to_string()));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let bytes = serde_json::to_vec_pretty(&*document)?;
        crate::file::atomic_write(path, &bytes)?;
        let mut map = self
            .projects
            .write()
            .expect("bellatrix-project lock poisoned");
        if let Some(current) = map.get_mut(id)
            && Arc::ptr_eq(&current.save_lock, &save_lock)
        {
            current.saved_revision = Some(document.revision());
        }
        Ok(document.revision())
    }
}

#[cfg(test)]
mod external_storage_tests {
    use super::*;

    fn setup(saved: bool) -> (ProjectService, ProjectId) {
        let service = ProjectService::new();
        let id = ProjectId::new("external").expect("id");
        service
            .open(
                ProjectDocument::new(id.clone(), "External").expect("document"),
                saved,
            )
            .expect("open");
        (service, id)
    }
    fn prepare(service: &ProjectService, id: &ProjectId) -> ProjectSaveTicket {
        service
            .prepare_external_save(
                id,
                service.snapshot(id).expect("snapshot").revision(),
                "not-created/project.json".into(),
            )
            .expect("prepare")
    }

    #[test]
    fn import_marks_only_persisted_snapshots_clean_and_rejects_duplicate_identity() {
        let (service, id) = setup(true);
        assert!(!service.is_dirty(&id).expect("dirty"));
        let original = service.snapshot(&id).expect("snapshot");
        assert!(matches!(
            service.open((*original).clone(), false),
            Err(ProjectError::AlreadyOpen(_))
        ));
        assert!(Arc::ptr_eq(
            &original,
            &service.snapshot(&id).expect("snapshot")
        ));
        let (service, id) = setup(false);
        assert!(service.is_dirty(&id).expect("dirty"));
    }

    #[test]
    fn malformed_import_is_not_admitted() {
        let service = ProjectService::new();
        let document: ProjectDocument = serde_json::from_value(
            serde_json::json!({"schema_version":42,"id":"bad","name":"Bad"}),
        )
        .expect("decode");
        assert!(matches!(
            service.open(document, true),
            Err(ProjectError::Schema(42))
        ));
        assert!(service.list().is_empty());
    }

    #[test]
    fn exact_receipt_is_single_use_and_performs_no_disk_io() {
        let (service, id) = setup(false);
        let ticket = prepare(&service, &id);
        let document: ProjectDocument = serde_json::from_slice(ticket.bytes()).expect("bytes");
        assert_eq!(&document, ticket.document().as_ref());
        assert!(service.is_dirty(&id).expect("dirty"));
        assert_eq!(
            service
                .acknowledge_external_save(&ticket, ticket.bytes())
                .expect("ack"),
            0
        );
        assert!(!service.is_dirty(&id).expect("dirty"));
        assert!(matches!(
            service.acknowledge_external_save(&ticket, ticket.bytes()),
            Err(ProjectError::InvalidSaveTicket)
        ));
    }

    #[test]
    fn abandoning_every_clone_releases_reservation_without_marking_saved() {
        let (service, id) = setup(false);
        let ticket = prepare(&service, &id);
        let clone = ticket.clone();
        drop(ticket);
        assert!(matches!(
            service.prepare_external_save(&id, 0, "unused".into()),
            Err(ProjectError::SaveInProgress)
        ));
        drop(clone);
        let _next = prepare(&service, &id);
        assert!(service.is_dirty(&id).expect("dirty"));
    }

    #[test]
    fn changed_revision_rejects_prepare_and_ack_without_losing_edits() {
        let (service, id) = setup(false);
        assert!(matches!(
            service.prepare_external_save(&id, 1, "unused".into()),
            Err(ProjectError::Conflict { .. })
        ));
        let ticket = prepare(&service, &id);
        service
            .replace(
                &id,
                "example.engine".into(),
                0,
                serde_json::json!({"new":true}),
            )
            .expect("replace");
        assert!(matches!(
            service.acknowledge_external_save(&ticket, ticket.bytes()),
            Err(ProjectError::Conflict {
                expected: 0,
                current: 1
            })
        ));
        assert!(service.is_dirty(&id).expect("dirty"));
        assert_eq!(service.snapshot(&id).expect("snapshot").revision(), 1);
        let next = prepare(&service, &id);
        service
            .acknowledge_external_save(&next, next.bytes())
            .expect("current receipt");
    }

    #[test]
    fn wrong_bytes_consume_ticket_and_cannot_claim_clean_even_for_previously_saved_state() {
        let (service, id) = setup(true);
        let ticket = prepare(&service, &id);
        assert!(matches!(
            service.acknowledge_external_save(&ticket, b"wrong"),
            Err(ProjectError::SaveContentMismatch)
        ));
        assert!(service.is_dirty(&id).expect("dirty"));
        assert!(matches!(
            service.acknowledge_external_save(&ticket, ticket.bytes()),
            Err(ProjectError::InvalidSaveTicket)
        ));
        let _next = prepare(&service, &id);
    }

    #[test]
    fn foreign_service_and_reopened_session_cannot_acknowledge_old_ticket() {
        let (service, id) = setup(false);
        let ticket = prepare(&service, &id);
        let (other, _) = setup(false);
        assert!(matches!(
            other.acknowledge_external_save(&ticket, ticket.bytes()),
            Err(ProjectError::SessionChanged(_))
        ));
        assert!(matches!(
            service.close(&id, true),
            Err(ProjectError::SaveInProgress)
        ));
        service
            .acknowledge_external_save(&ticket, ticket.bytes())
            .expect("finish before close");
        service.close(&id, true).expect("close");
        service
            .open((*ticket.document()).as_ref().clone(), false)
            .expect("reopen");
        let next = prepare(&service, &id);
        assert!(matches!(
            service.acknowledge_external_save(&ticket, ticket.bytes()),
            Err(ProjectError::SessionChanged(_))
        ));
        assert!(service.is_dirty(&id).expect("dirty"));
        service
            .acknowledge_external_save(&next, next.bytes())
            .expect("new session ack");
    }

    #[test]
    fn external_paths_are_opaque_and_do_not_reserve_cross_project_destinations() {
        let (service, id) = setup(false);
        let other_id = ProjectId::new("other").expect("id");
        service
            .open(
                ProjectDocument::new(other_id.clone(), "Other").expect("document"),
                false,
            )
            .expect("open");
        let path = PathBuf::from("unresolved/../shared.json");
        let first = service
            .prepare_external_save(&id, 0, path.clone())
            .expect("first project");
        let second = service
            .prepare_external_save(&other_id, 0, path.clone())
            .expect("other project");
        // Both tickets are valid: resolving aliases and denying the conflicting
        // destination is the native adapter's responsibility before either write.
        assert_eq!(first.path(), path);
        assert_eq!(second.path(), path);
        assert!(service.is_dirty(&id).expect("dirty"));
        assert!(service.is_dirty(&other_id).expect("dirty"));
    }

    #[test]
    fn native_and_disk_save_reservations_do_not_overlap() {
        let (service, id) = setup(false);
        let ticket = prepare(&service, &id);
        let target = std::env::temp_dir().join(format!(
            "bellatrix-no-external-write-{}.json",
            std::process::id()
        ));
        assert!(matches!(
            service.save(&id, &target),
            Err(ProjectError::SaveInProgress)
        ));
        assert!(!target.exists());
        drop(ticket);
        let session = {
            let projects = service.projects.read().expect("projects");
            Arc::clone(&projects.get(&id).expect("project").save_lock)
        };
        let guard = session.lock().expect("disk writer");
        assert!(matches!(
            service.prepare_external_save(&id, 0, target),
            Err(ProjectError::SaveInProgress)
        ));
        drop(guard);
        let _next = prepare(&service, &id);
    }
    #[test]
    fn snapshot_cleanup_is_atomic_and_preserves_changed_or_reopened_identity() {
        let (service, id) = setup(false);
        let imported = service.snapshot(&id).expect("imported");
        service
            .replace(
                &id,
                "example.engine".into(),
                0,
                serde_json::json!({"changed": true}),
            )
            .expect("edit");
        assert!(matches!(
            service.close_snapshot(&imported),
            Err(ProjectError::SessionChanged(_))
        ));
        assert_eq!(service.snapshot(&id).expect("retained").revision(), 1);
        service.close(&id, true).expect("explicit close");
        service.open((*imported).clone(), false).expect("reopened");
        assert!(matches!(
            service.close_snapshot(&imported),
            Err(ProjectError::SessionChanged(_))
        ));
        let reopened = service.snapshot(&id).expect("new session");
        let ticket = prepare(&service, &id);
        assert!(matches!(
            service.close_snapshot(&reopened),
            Err(ProjectError::SaveInProgress)
        ));
        drop(ticket);
        service
            .close_snapshot(&reopened)
            .expect("canceled preview cleanup");
        assert!(matches!(
            service.snapshot(&id),
            Err(ProjectError::NotOpen(_))
        ));
    }
}
