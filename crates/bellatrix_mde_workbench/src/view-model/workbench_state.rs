use super::selection::ReconciledSelection;
use bellatrix_mde::commands::ModelingTool;
use bellatrix_project::ProjectId;

#[derive(Clone)]
pub(super) struct PendingRelationEndpoint {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: Option<String>,
    pub subject_id: Option<String>,
    pub revision: u64,
    pub point: Option<bellatrix_mde::Point>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InlineEditTarget {
    Element(String),
    View(String),
}

pub(super) struct SelectionChange {
    pub diagram_changed: bool,
    pub selection_changed: bool,
    pub preserve_editor: bool,
}

pub(super) struct WorkbenchState {
    pub active_diagram: Option<String>,
    pub selected_element: Option<String>,
    pub selected_view: Option<String>,
    pub selected_tool: Option<ModelingTool>,
    pub visual_draft_mode: bool,
    pub pending_relation_source: Option<PendingRelationEndpoint>,
    pub draft_label_revision: Option<u64>,
    pub draft_label_pending: bool,
    editing_target: Option<InlineEditTarget>,
}

impl WorkbenchState {
    pub fn new(active_diagram: Option<String>) -> Self {
        Self {
            active_diagram,
            selected_element: None,
            selected_view: None,
            selected_tool: None,
            visual_draft_mode: false,
            pending_relation_source: None,
            draft_label_revision: None,
            draft_label_pending: false,
            editing_target: None,
        }
    }

    pub fn select_element(&mut self, id: String) -> SelectionChange {
        let selection_changed = self.selected_view.take().is_some();
        let subject_changed = self.selected_element.as_deref() != Some(id.as_str());
        if subject_changed {
            self.selected_element = Some(id);
            self.clear_edit();
        }
        SelectionChange {
            diagram_changed: false,
            selection_changed: selection_changed || subject_changed,
            preserve_editor: !subject_changed,
        }
    }

    pub fn select_diagram(&mut self, id: String) -> bool {
        let changed = self.active_diagram.as_deref() != Some(id.as_str());
        if changed {
            self.selected_view = None;
        }
        switch_diagram_selection(&mut self.active_diagram, &mut self.selected_element, id);
        self.cancel_tool();
        self.clear_edit();
        changed
    }

    pub fn arm_tool(&mut self, tool: ModelingTool) {
        self.clear_edit();
        self.selected_tool = Some(tool);
        self.pending_relation_source = None;
    }

    pub fn cancel_tool(&mut self) {
        self.selected_tool = None;
        self.pending_relation_source = None;
    }

    pub fn toggle_visual_draft_mode(&mut self) {
        self.visual_draft_mode = !self.visual_draft_mode;
        self.pending_relation_source = None;
    }

    pub fn clear_edit(&mut self) {
        self.editing_target = None;
        self.draft_label_revision = None;
        self.draft_label_pending = false;
    }

    pub fn begin_element_edit(&mut self, id: String) {
        self.clear_edit();
        self.editing_target = Some(InlineEditTarget::Element(id));
    }

    pub fn begin_view_edit(&mut self, id: String, revision: u64) {
        self.clear_edit();
        self.editing_target = Some(InlineEditTarget::View(id));
        self.draft_label_revision = Some(revision);
    }

    pub fn editing_element(&self) -> Option<&str> {
        match &self.editing_target {
            Some(InlineEditTarget::Element(id)) => Some(id),
            _ => None,
        }
    }

    pub fn editing_view(&self) -> Option<&str> {
        match &self.editing_target {
            Some(InlineEditTarget::View(id)) => Some(id),
            _ => None,
        }
    }

    pub fn editing_id(&self) -> Option<&str> {
        self.editing_element().or_else(|| self.editing_view())
    }

    pub fn is_editing(&self) -> bool {
        self.editing_target.is_some()
    }

    pub fn reconcile(&mut self, selection: ReconciledSelection) -> SelectionChange {
        let diagram_changed = selection.active_diagram != self.active_diagram;
        let selection_changed = selection.selected_element != self.selected_element
            || selection.selected_view != self.selected_view;
        // Semantic and visual identifiers may have the same spelling, but are not interchangeable.
        let matching_target = match &self.editing_target {
            Some(InlineEditTarget::Element(id)) => selection.selected_element.as_ref() == Some(id),
            Some(InlineEditTarget::View(id)) => {
                selection.selected_view.as_ref() == Some(id) && selection.selected_element.is_none()
            }
            None => false,
        };
        let preserve_editor = selection.preserve_editor && matching_target;
        if !preserve_editor {
            self.clear_edit();
        }
        if diagram_changed || selection_changed {
            self.pending_relation_source = None;
        }
        if diagram_changed {
            self.cancel_tool();
        }
        self.active_diagram = selection.active_diagram;
        self.selected_element = selection.selected_element;
        self.selected_view = selection.selected_view;
        SelectionChange {
            diagram_changed,
            selection_changed,
            preserve_editor,
        }
    }
}

pub(super) fn switch_diagram_selection(
    active: &mut Option<String>,
    selected: &mut Option<String>,
    id: String,
) {
    if active.as_deref() != Some(id.as_str()) {
        *selected = None;
    }
    *active = Some(id);
}
