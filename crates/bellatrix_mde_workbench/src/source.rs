//! A typed, lifecycle-owned source for Zed Items and Panels.
use crate::{MdeSnapshot, from_document};
use bellatrix_mde_cordis::{MdeUiBridge, MdeUiCommand};
use bellatrix_project::{ProjectId, ProjectService};
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct MdeUiSource {
    pub projects: Arc<ProjectService>,
    pub project_id: ProjectId,
    pub commands: mpsc::Sender<MdeUiCommand>,
    pub revisions: watch::Receiver<u64>,
    pub feedback: watch::Receiver<String>,
}

impl MdeUiSource {
    /// The owner of `bridge` must outlive all Items and Panels.
    pub fn from_bridge(
        projects: Arc<ProjectService>,
        bridge: &MdeUiBridge,
    ) -> Self {
        Self {
            projects,
            project_id: bridge.project_id().clone(),
            commands: bridge.sender(),
            revisions: bridge.changes(),
            feedback: bridge.feedback(),
        }
    }

    pub fn snapshot(&self) -> Result<MdeSnapshot, String> {
        let document = self.projects.snapshot(&self.project_id)
            .map_err(|error| error.to_string())?;
        from_document(&document)
    }

    pub fn submit(&self, command: MdeUiCommand) -> Result<(), String> {
        self.commands.try_send(command).map_err(|error| error.to_string())
    }
}
