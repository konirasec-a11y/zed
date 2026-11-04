//! A typed, lifecycle-owned source for Zed Items and Panels.
use crate::workbench_ui::{MdeSnapshot, from_document};
use bellatrix_mde_cordis::{MdeUiBridge, MdeUiCommand};
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use std::sync::{
    Arc,
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
    pub journal: Arc<bellatrix_mde_cordis::ExecutionJournal>,
    read_only_snapshot: Option<Arc<MdeSnapshot>>,
    read_only_document: Option<Arc<ProjectDocument>>,
    presentation_suspended: Arc<AtomicBool>,
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
            journal: bridge.execution_journal(),
            read_only_snapshot: None,
            read_only_document: None,
            presentation_suspended: Arc::new(AtomicBool::new(false)),
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
        let journal = Arc::new(bellatrix_mde_cordis::ExecutionJournal::default());
        Self {
            projects,
            project_id: snapshot.project_id.clone(),
            commands,
            revisions,
            feedback,
            validations,
            saves,
            journal,
            read_only_snapshot: Some(Arc::new(snapshot)),
            read_only_document: Some(document),
            presentation_suspended: Arc::new(AtomicBool::new(false)),
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
        source
    }

    pub(crate) fn set_preview_context(&self, preview: Option<String>) -> bool {
        let suspended = preview.is_some();
        self.presentation_suspended
            .swap(suspended, Ordering::AcqRel)
            != suspended
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

    pub fn record_interaction(
        &self,
        action: &'static str,
        identity: Option<&str>,
        outcome: bellatrix_mde::TraceOutcome,
    ) {
        self.journal
            .record_interaction(self.interaction_revision(), action, identity, outcome);
    }

    fn interaction_revision(&self) -> u64 {
        self.read_only_snapshot.as_ref().map_or_else(
            || {
                self.projects
                    .snapshot(&self.project_id)
                    .map(|document| document.revision())
                    .unwrap_or_else(|_| *self.revisions.borrow())
            },
            |snapshot| snapshot.revision,
        )
    }

    pub fn submit(&self, command: MdeUiCommand) -> Result<(), String> {
        let revision = self.interaction_revision();
        let identity = match &command {
            MdeUiCommand::MoveView(request) => Some(request.view_id.as_str()),
            MdeUiCommand::ResizeView(request) => Some(request.view_id.as_str()),
            _ => None,
        };
        let action = match &command {
            MdeUiCommand::SetViewLabelTracked { .. } => "command.enqueue.set_view_label_tracked",
            MdeUiCommand::SetViewTimingTracked { .. } => "command.enqueue.set_view_timing_tracked",
            MdeUiCommand::SetViewStyleTracked { .. } => "command.enqueue.set_view_style_tracked",
            MdeUiCommand::SetViewRoutingTracked { .. } => {
                "command.enqueue.set_view_routing_tracked"
            }
            MdeUiCommand::ReorderViewTracked { .. } => "command.enqueue.reorder_view_tracked",
            MdeUiCommand::SetPropertyTracked { .. } => "command.enqueue.set_property_tracked",
            MdeUiCommand::SetViewLabel(..) => "command.enqueue.set_view_label",
            MdeUiCommand::SetViewStyle(..) => "command.enqueue.set_view_style",
            MdeUiCommand::SetViewRouting(..) => "command.enqueue.set_view_routing",
            MdeUiCommand::CreateDiagram(..) => "command.enqueue.create_diagram",
            MdeUiCommand::CreateExplorerDiagram(..) => "command.enqueue.create_explorer_diagram",
            MdeUiCommand::RenameDiagram(..) => "command.enqueue.rename_diagram",
            MdeUiCommand::DeleteDiagram(..) => "command.enqueue.delete_diagram",
            MdeUiCommand::ValidateDiagram(..) => "command.enqueue.validate_diagram",
            MdeUiCommand::CreateVisualDraft(..) => "command.enqueue.create_visual_draft",
            MdeUiCommand::CreateElementOrVisualDraft(..) => {
                "command.enqueue.create_element_or_visual_draft"
            }
            MdeUiCommand::CreateElement(..) => "command.enqueue.create_element",
            MdeUiCommand::CreateModelElement(..) => "command.enqueue.create_model_element",
            MdeUiCommand::CreateRelationOrVisualDraft(..) => {
                "command.enqueue.create_relation_or_visual_draft"
            }
            MdeUiCommand::CreateRelationAtViewsOrVisualDraft(..) => {
                "command.enqueue.create_relation_at_views_or_visual_draft"
            }
            MdeUiCommand::CreateRelation(..) => "command.enqueue.create_relation",
            MdeUiCommand::MoveView(..) => "command.enqueue.move_view",
            MdeUiCommand::ResizeView(..) => "command.enqueue.resize_view",
            MdeUiCommand::RemoveView(..) => "command.enqueue.remove_view",
            MdeUiCommand::PlaceElementView(..) => "command.enqueue.place_element_view",
            MdeUiCommand::Rename(..) => "command.enqueue.rename",
            MdeUiCommand::SetOwner(..) => "command.enqueue.set_owner",
            MdeUiCommand::SetExplorerParent(..) => "command.enqueue.set_explorer_parent",
            MdeUiCommand::DeleteElement(..) => "command.enqueue.delete_element",
            MdeUiCommand::SetProperty(..) => "command.enqueue.set_property",
            MdeUiCommand::AddMember(..) => "command.enqueue.add_member",
            MdeUiCommand::MoveParameter(..) => "command.enqueue.move_parameter",
            MdeUiCommand::QuickEdit(..) => "command.enqueue.quick_edit",
            MdeUiCommand::SetNameExpression(..) => "command.enqueue.set_name_expression",
            MdeUiCommand::SaveProject(..) => "command.enqueue.save_project",
        };

        if self.is_presentation_suspended() {
            self.journal.record_interaction(
                revision,
                action,
                identity,
                bellatrix_mde::TraceOutcome::Invalid,
            );
            return Err("This Workbench is not the active MDE document. Select its MDE tab before editing or saving its model.".into());
        }
        if self.is_read_only() {
            self.journal.record_interaction(
                revision,
                action,
                identity,
                bellatrix_mde::TraceOutcome::Invalid,
            );
            return Err("Read-only preview: edit the source in the native editor, then close and reopen the preview".into());
        }
        let permit = match self.commands.try_reserve() {
            Ok(permit) => permit,
            Err(error) => {
                self.journal.record_interaction(
                    revision,
                    action,
                    identity,
                    bellatrix_mde::TraceOutcome::Error,
                );
                return Err(error.to_string());
            }
        };
        // Record before publishing so a fast consumer cannot report completion first.
        // Success means queue capacity was reserved, not that the command was applied.
        self.journal.record_interaction(
            revision,
            action,
            identity,
            bellatrix_mde::TraceOutcome::Success,
        );
        permit.send(command);
        Ok(())
    }
}
