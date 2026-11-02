use crate::{ProjectDocument, ProjectError, ProjectId};
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

#[derive(Clone)]
pub struct ProjectSaveRequested {
    pub id: ProjectId,
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
}
pub struct ProjectClosedEvent;
impl Event for ProjectClosedEvent {
    const NAME: &'static str = "bellatrix.bellatrix-project.closed.v1";
    type Args = ProjectClosed;
    type Output = ();
}
