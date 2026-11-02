use crate::{ProjectDocument, ProjectError, ProjectId};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

struct OpenProject {
    document: Arc<ProjectDocument>,
    saved_revision: Option<u64>,
    save_lock: Arc<Mutex<()>>,
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
            },
        );
        Ok(doc)
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
