//! A typed, lifecycle-owned source for Zed Items and Panels.
use crate::workbench_ui::MdeSnapshot;
use bellatrix_mde_cordis::{MdeSession, MdeUiBridge, MdeUiCommand};
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::watch;

#[derive(Clone)]
pub struct MdeUiSource {
    pub projects: Arc<ProjectService>,
    pub project_id: ProjectId,
    pub revisions: watch::Receiver<u64>,
    pub projections: watch::Receiver<Result<Arc<MdeSnapshot>, String>>,
    pub move_previews: Option<bellatrix_mde_cordis::MovePreviewClient>,
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
    session: MdeSession,
    native_blocked: Arc<AtomicBool>,
    presentation_suspended: Arc<AtomicBool>,
}

impl MdeUiSource {
    /// The owner of `bridge` must outlive all Items and Panels.
    pub fn from_bridge(projects: Arc<ProjectService>, bridge: &MdeUiBridge) -> Self {
        Self {
            projects,
            project_id: bridge.project_id().clone(),
            revisions: bridge.changes(),
            projections: bridge.projections(),
            move_previews: Some(bridge.move_previews()),
            feedback: bridge.feedback(),
            validations: bridge.validations(),
            saves: bridge.saves(),
            journal: bridge.execution_journal(),
            session: bridge.session(),
            native_blocked: Arc::new(AtomicBool::new(false)),
            presentation_suspended: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn block_native_edits(&self, blocked: bool) {
        self.native_blocked.store(blocked, Ordering::Release);
    }

    pub fn allows_creation(&self) -> bool {
        !self.is_read_only()
    }

    pub fn document(&self) -> Result<Arc<ProjectDocument>, String> {
        self.session.snapshot()
    }

    pub(crate) fn activity(&self) -> watch::Receiver<()> {
        self.session.activity()
    }

    pub fn is_pending(&self) -> bool {
        self.session.is_pending()
    }

    pub fn snapshot_is_current(&self) -> bool {
        self.session.snapshot().is_ok()
    }

    pub fn is_read_only(&self) -> bool {
        self.is_presentation_suspended()
            || self.native_blocked.load(Ordering::Acquire)
            || !self.snapshot_is_current()
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

    pub(crate) fn presentation_snapshot(&self) -> Result<MdeSnapshot, String> {
        let document = self.session.snapshot()?;
        let projection = self.projections.borrow().clone()?;
        if !projection.matches_session(&document) {
            return Err("The project projection belongs to another document session".into());
        }
        // Revision publication precedes asynchronous projection. Keep the last
        // complete frame visible; editing still requires snapshot() below.
        Ok((*projection).clone())
    }

    pub fn snapshot(&self) -> Result<MdeSnapshot, String> {
        let document = self.session.snapshot()?;
        if let Some(previews) = &self.move_previews {
            previews.set_document(Arc::clone(&document))?;
        }
        let projection = self.projections.borrow().clone()?;
        if !projection.matches_document(&document) {
            return Err("Waiting for the current project projection".into());
        }
        Ok((*projection).clone())
    }

    pub(crate) fn preview_move(
        &self,
        snapshot: &MdeSnapshot,
        subject_id: &str,
        parent_id: Option<&str>,
        parent_occurrence: Option<&bellatrix_mde_cordis::ViewOccurrence>,
        diagram: bool,
    ) -> Option<Result<(), String>> {
        if self.is_read_only() || self.is_presentation_suspended() {
            return Some(Err("The active document is not editable".into()));
        }
        let Some(previews) = &self.move_previews else {
            return Some(Err("Move preview is unavailable for this source".into()));
        };
        let document = self.session.snapshot();
        let document = match document {
            Ok(document) => document,
            Err(error) => return Some(Err(error)),
        };
        if !snapshot.matches_document(&document) {
            return Some(Err("The model changed while preparing this move".into()));
        }
        previews.check(bellatrix_mde_cordis::ModelingPreviewMoveRequested {
            document,
            subject_id: subject_id.to_owned(),
            parent_id: parent_id.map(str::to_owned),
            parent_occurrence: parent_occurrence.cloned(),
            diagram,
        })
    }

    pub fn record_interaction(
        &self,
        action: &'static str,
        identity: Option<&str>,
        outcome: bellatrix_mde_cordis::TraceOutcome,
    ) {
        self.journal
            .record_interaction(self.interaction_revision(), action, identity, outcome);
    }

    fn interaction_revision(&self) -> u64 {
        self.session
            .snapshot()
            .map(|document| document.revision())
            .unwrap_or_else(|_| *self.revisions.borrow())
    }

    pub fn submit(&self, command: MdeUiCommand) -> Result<(), String> {
        let revision = self.interaction_revision();
        let identity = match &command {
            MdeUiCommand::MoveView(request) => Some(request.view_id.as_str()),
            MdeUiCommand::ResizeView(request) => Some(request.view_id.as_str()),
            MdeUiCommand::SetViewPath(request)
            | MdeUiCommand::SetViewPathTracked { request, .. } => Some(request.view_id.as_str()),
            _ => None,
        };
        let action = match &command {
            MdeUiCommand::SetViewLabelTracked { .. } => "command.enqueue.set_view_label_tracked",
            MdeUiCommand::SetViewTimingTracked { .. } => "command.enqueue.set_view_timing_tracked",
            MdeUiCommand::SetViewStyleTracked { .. } => "command.enqueue.set_view_style_tracked",
            MdeUiCommand::SetViewRoutingTracked { .. } => {
                "command.enqueue.set_view_routing_tracked"
            }
            MdeUiCommand::SetViewPathTracked { .. } => "command.enqueue.set_view_path_tracked",
            MdeUiCommand::ReorderViewTracked { .. } => "command.enqueue.reorder_view_tracked",
            MdeUiCommand::SetPropertyTracked { .. } => "command.enqueue.set_property_tracked",
            MdeUiCommand::SetViewLabel(..) => "command.enqueue.set_view_label",
            MdeUiCommand::SetViewStyle(..) => "command.enqueue.set_view_style",
            MdeUiCommand::SetViewRouting(..) => "command.enqueue.set_view_routing",
            MdeUiCommand::SetViewPath(..) => "command.enqueue.set_view_path",
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
            MdeUiCommand::CreateElementVisualDraft(..) => {
                "command.enqueue.create_element_visual_draft"
            }
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
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return Err("This Workbench is not the active MDE document. Select its MDE tab before editing or saving its model.".into());
        }
        if self.is_read_only() && !matches!(&command, MdeUiCommand::ValidateDiagram(_)) {
            self.journal.record_interaction(
                revision,
                action,
                identity,
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return Err("The document is temporarily unavailable for editing; finish its pending write or reopen the native document if its session closed".into());
        }
        let identity = identity.map(str::to_owned);
        let result = self.session.submit(command);
        self.journal.record_interaction(
            revision,
            action,
            identity.as_deref(),
            if result.is_ok() {
                bellatrix_mde_cordis::TraceOutcome::Success
            } else {
                bellatrix_mde_cordis::TraceOutcome::Error
            },
        );
        result
    }
}
