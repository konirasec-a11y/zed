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
        self.prepare_save(id, expected_revision, path, None)
    }

    pub(crate) fn prepare_encoded_save(
        &self,
        document: Arc<ProjectDocument>,
        path: PathBuf,
        bytes: Arc<[u8]>,
    ) -> Result<ProjectSaveTicket, ProjectError> {
        self.prepare_save(document.id(), document.revision(), path, Some((&document, bytes)))
    }

    fn prepare_save(
        &self,
        id: &ProjectId,
        expected_revision: u64,
        path: PathBuf,
        encoded: Option<(&Arc<ProjectDocument>, Arc<[u8]>)>,
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
        let bytes = match encoded {
            Some((expected, bytes)) => {
                if !Arc::ptr_eq(expected, &current.document) {
                    return Err(ProjectError::SessionChanged(id.as_str().into()));
                }
                bytes
            }
            None => serde_json::to_vec_pretty(&*current.document)?.into(),
        };
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
        self.replace_matching(id, namespace, expected, value, None)
            .map(|(document, changed)| changed.then_some(document))
    }

    pub(crate) fn replace_snapshot(
        &self,
        document: &Arc<ProjectDocument>,
        namespace: String,
        value: Value,
    ) -> Result<(Arc<ProjectDocument>, bool), ProjectError> {
        self.replace_matching(document.id(), namespace, document.revision(), value, Some(document))
    }

    fn replace_matching(
        &self,
        id: &ProjectId,
        namespace: String,
        expected: u64,
        value: Value,
        snapshot: Option<&Arc<ProjectDocument>>,
    ) -> Result<(Arc<ProjectDocument>, bool), ProjectError> {
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
        if snapshot.is_some_and(|snapshot| !Arc::ptr_eq(snapshot, &current.document)) {
            return Err(ProjectError::SessionChanged(id.as_str().into()));
        }
        let mut next = (*current.document).clone();
        if !next.replace(namespace, value)? {
            return Ok((Arc::clone(&current.document), false));
        }
        current.document = Arc::new(next);
        Ok((Arc::clone(&current.document), true))
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
