use super::selection::ReconciledSelection;
use bellatrix_mde_cordis::ModelingTool;
use bellatrix_project::ProjectId;

#[derive(Clone)]
pub(crate) struct PendingRelationEndpoint {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: Option<String>,
    pub subject_id: Option<String>,
    pub revision: u64,
    pub point: Option<bellatrix_mde_cordis::Point>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InlineEditTarget {
    Element(String),
    View {
        id: String,
        revision: u64,
        pending: bool,
    },
}

pub(crate) struct SelectionChange {
    pub diagram_changed: bool,
    pub selection_changed: bool,
    pub preserve_editor: bool,
}

pub(crate) struct WorkbenchState {
    active_diagram: Option<String>,
    selected_element: Option<String>,
    selected_view: Option<String>,
    selected_explorer: Option<(String, Option<bellatrix_mde_cordis::ViewOccurrence>)>,
    selected_tool: Option<ModelingTool>,
    visual_draft_mode: bool,
    pending_relation_source: Option<PendingRelationEndpoint>,
    editing_target: Option<InlineEditTarget>,
}

impl WorkbenchState {
    pub fn new(active_diagram: Option<String>) -> Self {
        Self {
            active_diagram,
            selected_element: None,
            selected_view: None,
            selected_explorer: None,
            selected_tool: None,
            visual_draft_mode: false,
            pending_relation_source: None,
            editing_target: None,
        }
    }

    pub fn active_diagram(&self) -> Option<&str> {
        self.active_diagram.as_deref()
    }

    pub fn selected_element(&self) -> Option<&str> {
        self.selected_element.as_deref()
    }

    pub fn selected_view(&self) -> Option<&str> {
        self.selected_view.as_deref()
    }

    pub fn selected_explorer_key(&self) -> Option<&str> {
        self.selected_explorer.as_ref().map(|(key, _)| key.as_str())
    }

    pub fn selected_explorer_occurrence(&self) -> Option<&bellatrix_mde_cordis::ViewOccurrence> {
        self.selected_explorer
            .as_ref()
            .and_then(|(_, occurrence)| occurrence.as_ref())
            .filter(|occurrence| Some(occurrence.diagram_id.as_str()) == self.active_diagram())
    }

    pub fn selected_canvas_view(&self) -> Option<&str> {
        self.selected_view().or_else(|| {
            self.selected_explorer_occurrence()
                .map(|occurrence| occurrence.view_id.as_str())
        })
    }

    pub fn select_explorer_element(
        &mut self,
        id: String,
        key: String,
        occurrence: Option<bellatrix_mde_cordis::ViewOccurrence>,
    ) -> SelectionChange {
        let mut change = self.select_element(id);
        self.selected_explorer = Some((key, occurrence));
        change.selection_changed = true;
        change
    }

    pub fn reconcile_explorer_selection(
        &mut self,
        section: &bellatrix_mde_cordis::projection::ModelingProjection,
    ) {
        let Some((key, _)) = &self.selected_explorer else {
            return;
        };
        let rows =
            super::explorer::project_explorer(section, super::explorer::ExplorerSortOrder::Added);
        if rows.iter().any(|row| {
            &row.key == key
                && Some(
                    row.view
                        .as_ref()
                        .and_then(|view| view.subject_id.as_deref())
                        .unwrap_or(row.element_id.as_str()),
                ) == self.selected_element()
                && (row.diagram_context.is_none()
                    || row.diagram_context.as_deref() == self.active_diagram())
        }) {
            // The same Explorer row can acquire a different occurrence after a move.
            self.selected_explorer = Some((
                key.clone(),
                section.explorer.parent_occurrences.get(key).cloned(),
            ));
        } else {
            self.selected_explorer = None;
        }
    }

    pub fn selected_tool(&self) -> Option<ModelingTool> {
        self.selected_tool
    }

    pub fn visual_draft_mode(&self) -> bool {
        self.visual_draft_mode
    }

    pub fn pending_relation_source(&self) -> Option<&PendingRelationEndpoint> {
        self.pending_relation_source.as_ref()
    }

    pub fn select_element(&mut self, id: String) -> SelectionChange {
        self.select_target(Some(id), None)
    }

    pub fn select_view(&mut self, subject: Option<String>, view_id: String) -> SelectionChange {
        self.select_target(subject, Some(view_id))
    }

    pub fn clear_selection(&mut self) -> SelectionChange {
        self.select_target(None, None)
    }

    fn select_target(&mut self, subject: Option<String>, view: Option<String>) -> SelectionChange {
        let selection_changed = self.selected_element != subject
            || self.selected_view != view
            || self.selected_explorer.is_some();
        self.selected_explorer = None;
        let preserve_editor = self.edit_matches(subject.as_deref(), view.as_deref());
        if !preserve_editor {
            self.clear_edit();
        }
        if selection_changed {
            self.clear_relation_source();
        }
        self.selected_element = subject;
        self.selected_view = view;
        SelectionChange {
            diagram_changed: false,
            selection_changed,
            preserve_editor,
        }
    }

    pub fn begin_relation(&mut self, source: PendingRelationEndpoint) -> SelectionChange {
        let change = self.select_target(source.subject_id.clone(), source.view_id.clone());
        self.pending_relation_source = Some(source);
        change
    }

    pub fn select_relation_target(
        &mut self,
        subject: Option<String>,
        view: Option<String>,
    ) -> SelectionChange {
        // The second endpoint changes selection without abandoning the first endpoint.
        let source = self.pending_relation_source.take();
        let change = self.select_target(subject, view);
        self.pending_relation_source = source;
        change
    }

    pub fn clear_relation_source(&mut self) {
        self.pending_relation_source = None;
    }

    fn edit_matches(&self, element: Option<&str>, view: Option<&str>) -> bool {
        match &self.editing_target {
            Some(InlineEditTarget::Element(id)) => element == Some(id.as_str()),
            Some(InlineEditTarget::View { id, .. }) => {
                view == Some(id.as_str()) && element.is_none()
            }
            None => false,
        }
    }

    pub fn select_diagram(&mut self, id: String) -> bool {
        let changed = self.active_diagram.as_deref() != Some(id.as_str());
        self.selected_explorer = None;
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
    }

    pub fn begin_element_edit(&mut self, id: String) {
        // Editing the same subject must retain its selected canvas occurrence
        // as well as an Explorer member's presentation anchor.
        if self.selected_element() != Some(id.as_str()) {
            self.select_element(id.clone());
        }
        self.cancel_tool();
        self.editing_target = Some(InlineEditTarget::Element(id));
    }

    pub fn begin_view_edit(&mut self, id: String, revision: u64) {
        self.select_view(None, id.clone());
        self.cancel_tool();
        self.editing_target = Some(InlineEditTarget::View {
            id,
            revision,
            pending: false,
        });
    }

    pub fn draft_label_revision(&self) -> Option<u64> {
        match self.editing_target.as_ref() {
            Some(InlineEditTarget::View { revision, .. }) => Some(*revision),
            _ => None,
        }
    }

    pub fn draft_label_pending(&self) -> bool {
        matches!(
            self.editing_target,
            Some(InlineEditTarget::View { pending: true, .. })
        )
    }

    pub fn view_label_sent(&mut self) {
        if let Some(InlineEditTarget::View { pending, .. }) = &mut self.editing_target {
            *pending = true;
        }
    }

    pub fn view_label_completed(&mut self, latest_revision: Option<u64>) {
        if let Some(InlineEditTarget::View {
            revision, pending, ..
        }) = &mut self.editing_target
        {
            *pending = false;
            if let Some(latest_revision) = latest_revision {
                *revision = latest_revision;
            }
        }
    }

    pub fn editing_element(&self) -> Option<&str> {
        match &self.editing_target {
            Some(InlineEditTarget::Element(id)) => Some(id),
            _ => None,
        }
    }

    pub fn editing_view(&self) -> Option<&str> {
        match &self.editing_target {
            Some(InlineEditTarget::View { id, .. }) => Some(id),
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
        let matching_target = self.edit_matches(
            selection.selected_element.as_deref(),
            selection.selected_view.as_deref(),
        );
        let preserve_editor = selection.preserve_editor && matching_target;
        if !preserve_editor {
            self.clear_edit();
        }
        if diagram_changed || selection_changed {
            self.selected_explorer = None;
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

pub(crate) fn switch_diagram_selection(
    active: &mut Option<String>,
    selected: &mut Option<String>,
    id: String,
) {
    if active.as_deref() != Some(id.as_str()) {
        *selected = None;
    }
    *active = Some(id);
}
