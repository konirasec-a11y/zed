use crate::api::source::MdeUiSource;
use crate::components::quick_tools::ParameterAction;
use crate::components::quick_tools::QuickEditTarget;
use crate::pages::mde_page::ElementViewPlacement;
use crate::pages::mde_page::MdeWorkbenchItem;
use crate::pages::mde_page::element_view_placement;
use crate::view_model::selection;
use crate::view_model::snapshot::MdeSnapshot;
use bellatrix_mde_cordis::ModelingElementKind;
use bellatrix_mde_cordis::ModelingMemberKind;
use bellatrix_mde_cordis::ModelingTool;
use bellatrix_project::ProjectId;
use gpui::Context;
use gpui::Window;

impl MdeWorkbenchItem {
    pub(crate) fn quick_edit_snapshot(
        &mut self,
        target: &QuickEditTarget,
        cx: &mut Context<Self>,
    ) -> Option<MdeSnapshot> {
        if self.source.is_read_only() || self.selected_view_is_inspection_only() {
            return None;
        }
        if self.project_id() != &target.project
            || self.presentation.selected_element() != Some(target.element.as_str())
            || self.presentation.active_diagram() != Some(target.diagram.as_str())
            || target.view.as_deref().is_some_and(|view| {
                self.presentation
                    .selected_canvas_view()
                    .is_some_and(|selected| selected != view)
            })
            || self.presentation.selected_tool().is_some()
        {
            self.feedback = "Quick edit not sent: the active selection changed".into();
            cx.notify();
            return None;
        }
        let snapshot = match self.source.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.feedback = format!("Quick edit not sent: {error}");
                cx.notify();
                return None;
            }
        };
        if snapshot.revision != target.revision {
            self.feedback = "Quick edit not sent: the model changed; try again".into();
            cx.notify();
            return None;
        }
        Some(snapshot)
    }

    pub(crate) fn run_quick_action(
        &mut self,
        action: &str,
        target: &QuickEditTarget,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.quick_edit_snapshot(target, cx) else {
            return;
        };
        let result = self.source.execute_quick_action(
            &target.element,
            action,
            Some(&target.diagram),
            target.view.as_deref(),
            snapshot.revision,
        );
        self.report_command_result(result, cx);
    }

    pub fn add_member_to_selected(&mut self, kind: ModelingMemberKind, cx: &mut Context<Self>) {
        if !self.source.allows_creation() {
            self.feedback = "Member creation is not available for this document".into();
            cx.notify();
            return;
        }
        let Some(owner_id) = self.presentation.selected_element().map(str::to_owned) else {
            self.feedback = "Select a classifier first".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let result = self.source.add_member(&owner_id, kind, snapshot.revision);
        self.report_command_result(result, cx);
    }
    pub(crate) fn run_parameter_action(
        &mut self,
        action: ParameterAction,
        project_id: ProjectId,
        element_id: String,
        expected_revision: u64,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        if self.project_id() != &project_id || self.selected_element() != Some(element_id.as_str())
        {
            self.feedback = "Parameter action not sent: the active selection changed".into();
            cx.notify();
            return;
        }
        let result = self.source.execute_quick_action(
            &element_id,
            action.id(),
            None,
            None,
            expected_revision,
        );
        self.report_command_result(result, cx);
    }

    pub(crate) fn submit_admitted_inspector_property(
        &mut self,
        project_id: ProjectId,
        element_id: String,
        path: String,
        value: serde_json::Value,
        expected_revision: u64,
        cx: &mut Context<Self>,
    ) -> Result<tokio::sync::watch::Receiver<Option<Result<u64, String>>>, String> {
        if self.source.is_read_only() {
            return Err("This document is not currently editable; draft preserved".into());
        }
        // Admission captured the target while its input was active. A later
        // selection change must not redirect or discard that accepted edit.
        if self.project_id() != &project_id {
            return Err("Inspector project changed; draft preserved".into());
        }
        let (completion, receipt) = tokio::sync::watch::channel(None);
        self.source.set_property_tracked(
            &element_id,
            &path,
            value,
            expected_revision,
            completion,
        )?;
        self.feedback = "Updating inspected property…".into();
        cx.notify();
        Ok(receipt)
    }

    pub fn set_selected_property(
        &mut self,
        path: String,
        value: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Some(element_id) = self.presentation.selected_element().map(str::to_owned) else {
            self.feedback = "Select a model element first".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let result = self
            .source
            .set_property(&element_id, &path, value, snapshot.revision);
        self.report_command_result(result, cx);
    }
    pub fn delete_selected_from_model(&mut self, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Some(element_id) = self.presentation.selected_element().map(str::to_owned) else {
            self.feedback = "Select an element to delete".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let result = self.source.delete_element(&element_id, snapshot.revision);
        let sent = result.is_ok();
        self.report_command_result(result, cx);
        if !sent {
            return;
        }
        // Submission only queues the command. Reconciliation clears a selection
        // after deletion succeeds, keeping the inspector available on rejection.
        self.gesture = None;
        cx.notify();
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if let Ok(snapshot) = self.source.snapshot() {
            self.reconcile_selection(&snapshot.section);
        }
        cx.notify();
    }

    pub(crate) fn reconcile_selection(
        &mut self,
        section: &bellatrix_mde_cordis::projection::ModelingProjection,
    ) {
        let selection = selection::reconcile_selection(
            section,
            self.presentation.active_diagram(),
            self.presentation.selected_view(),
            self.presentation.selected_element(),
            self.presentation.editing_id(),
        );
        let change = self.presentation.reconcile(selection);
        self.presentation.reconcile_explorer_selection(section);
        if change.diagram_changed || change.selection_changed {
            self.gesture = None;
        }
        if change.diagram_changed {
            self.drop_point = None;
            self.pan_origin = None;
            self.validation_status = "Not yet validated for this diagram".into();
        }
        if !change.preserve_editor {
            self.inline_editor = None;
            self.inline_draft = None;
        }
    }

    pub(crate) fn report_command_result(
        &mut self,
        result: Result<(), String>,
        cx: &mut Context<Self>,
    ) {
        self.feedback = match result {
            Ok(()) => "Command sent to Cordis".into(),
            Err(error) => format!("Command not sent: {error}"),
        };
        cx.notify();
    }
    pub(crate) fn click_modeling_tool(
        &mut self,
        tool: ModelingTool,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        if !self.allows_modeling_tool(tool, cx) {
            return;
        }
        if let ModelingTool::Relation(kind) = tool {
            self.connect_free_message_endpoint(kind, bellatrix_mde_cordis::Point { x, y }, cx);
        } else {
            self.place_modeling_tool(tool, x, y, cx);
        }
    }
    pub(crate) fn place_modeling_tool(
        &mut self,
        tool: ModelingTool,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        if !self.allows_modeling_tool(tool, cx) {
            return;
        }
        match tool {
            ModelingTool::Element(kind) => self.create_element(kind, x, y, cx),
            ModelingTool::Relation(_) => self.arm_drag_tool(tool, cx),
            ModelingTool::ProvidedInterface | ModelingTool::RequiredInterface => {
                self.create_interface_view(tool, x, y, cx);
            }
        }
    }
    pub(crate) fn create_interface_view(
        &mut self,
        tool: ModelingTool,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let snapshot = match self.source.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.feedback = error;
                cx.notify();
                return;
            }
        };
        let Some(diagram) = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| Some(diagram.id.as_str()) == self.presentation.active_diagram())
        else {
            self.feedback = "Select a diagram to place the Interface presentation".into();
            cx.notify();
            return;
        };
        let result =
            self.source
                .create_interface_presentation(&diagram.id, tool, (x, y), snapshot.revision);
        let sent = result.is_ok();
        self.report_command_result(result, cx);
        if sent {
            self.presentation.cancel_tool();
        }
    }

    pub fn create_element(
        &mut self,
        kind: ModelingElementKind,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        if !self.source.allows_creation() {
            self.feedback = "Element creation is not available for this document".into();
            cx.notify();
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let diagram_id = self
            .presentation
            .active_diagram()
            .and_then(|id| {
                snapshot
                    .section
                    .diagrams
                    .iter()
                    .find(|diagram| diagram.id == id)
            })
            .or_else(|| snapshot.section.diagrams.first())
            .map(|diagram| diagram.id.clone());
        let Some(diagram_id) = diagram_id else {
            self.feedback = "Project has no diagram".into();
            cx.notify();
            return;
        };
        let result = self.source.create_element(
            &diagram_id,
            kind,
            self.presentation.selected_element(),
            (x, y),
            self.presentation.visual_draft_mode(),
            snapshot.revision,
        );
        let sent = result.is_ok();
        self.report_command_result(result, cx);
        if sent {
            self.presentation.cancel_tool();
        }
    }
    pub(crate) fn validate_explorer_drop(
        &self,
        drag: &crate::components::model_explorer::ExplorerElementDrag,
    ) -> Result<(), String> {
        if self.source.is_read_only() || self.source.is_presentation_suspended() {
            return Err("The active document is not editable".into());
        }
        if self.project_id() != &drag.project_id {
            return Err("The dragged element belongs to another project".into());
        }
        let target_diagram_id = drag
            .target_diagram_id
            .as_deref()
            .ok_or("No active diagram was captured for this drag")?;
        if self.active_diagram() != Some(target_diagram_id) {
            return Err(
                "The active diagram changed during the drag; drag the element again".into(),
            );
        }
        let snapshot = self.source.snapshot()?;
        if snapshot.revision != drag.revision {
            return Err("The model changed during the drag; drag the element again".into());
        }
        if !snapshot
            .section
            .elements
            .iter()
            .any(|element| element.id == drag.element_id)
        {
            return Err("The dragged model element no longer exists".into());
        }
        let diagram = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| diagram.id == target_diagram_id)
            .ok_or_else(|| "The target diagram no longer exists".to_owned())?;
        if let Some(view_id) = &drag.source_view_id
            && !diagram.all_views().any(|view| {
                &view.id == view_id && view.subject.as_deref() == Some(drag.element_id.as_str())
            })
        {
            return Err("The dragged occurrence no longer matches its element".into());
        }
        Ok(())
    }

    pub(crate) fn submit_explorer_operation(
        &mut self,
        project_id: &ProjectId,
        expected_revision: u64,
        operation: impl FnOnce(&MdeUiSource) -> Result<(), String>,
        cx: &mut Context<Self>,
    ) -> bool {
        let allowed = if self.source.is_read_only() || self.source.is_presentation_suspended() {
            Err("The active document is not editable".to_owned())
        } else if self.project_id() != project_id {
            Err("The active project changed; reopen the Explorer menu".to_owned())
        } else {
            self.source.snapshot().and_then(|snapshot| {
                if snapshot.revision == expected_revision {
                    Ok(())
                } else {
                    Err("The model changed; reopen the Explorer menu".to_owned())
                }
            })
        };
        if let Err(error) = allowed {
            self.source.record_interaction(
                "explorer.command.rejected",
                None,
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            self.feedback = error;
            cx.notify();
            return false;
        }
        let sent = match operation(&self.source) {
            Ok(()) => {
                self.feedback = "Command sent to Cordis".into();
                true
            }
            Err(error) => {
                self.feedback = format!("Command not sent: {error}");
                false
            }
        };
        cx.notify();
        sent
    }

    pub(crate) fn validate_explorer_move(
        &self,
        project_id: &ProjectId,
        revision: u64,
        target: &crate::components::model_explorer::ExplorerMoveTarget,
        parent_id: Option<&str>,
        parent_occurrence: Option<&bellatrix_mde_cordis::ViewOccurrence>,
    ) -> Result<(), String> {
        if self.source.is_read_only() || self.source.is_presentation_suspended() {
            return Err("The active document is not editable".into());
        }
        if self.project_id() != project_id {
            return Err("The item belongs to another project".into());
        }
        let snapshot = self.source.snapshot()?;
        if snapshot.revision != revision {
            return Err("The model changed; reopen the menu or drag the item again".into());
        }
        let subject = match target {
            crate::components::model_explorer::ExplorerMoveTarget::Element(id)
            | crate::components::model_explorer::ExplorerMoveTarget::Diagram(id) => id,
        };
        if snapshot.section.composed_subjects.contains(subject) {
            return Err("Document composition moves are not yet supported losslessly".into());
        }
        self.source
            .preview_move(
                &snapshot,
                subject,
                parent_id,
                parent_occurrence,
                matches!(
                    target,
                    crate::components::model_explorer::ExplorerMoveTarget::Diagram(_)
                ),
            )
            .unwrap_or_else(|| Err(bellatrix_mde_cordis::MOVE_PREVIEW_PENDING.into()))
    }

    pub(crate) fn move_explorer_item(
        &mut self,
        project_id: &ProjectId,
        revision: u64,
        target: &crate::components::model_explorer::ExplorerMoveTarget,
        parent_id: Option<String>,
        parent_occurrence: Option<bellatrix_mde_cordis::ViewOccurrence>,
        cx: &mut Context<Self>,
    ) -> bool {
        let subject = match target {
            crate::components::model_explorer::ExplorerMoveTarget::Element(id)
            | crate::components::model_explorer::ExplorerMoveTarget::Diagram(id) => id,
        };
        if self
            .source
            .snapshot()
            .is_ok_and(|snapshot| snapshot.section.composed_subjects.contains(subject))
        {
            self.source.record_interaction(
                "explorer.move.rejected",
                Some(subject),
                bellatrix_mde_cordis::TraceOutcome::Unsupported,
            );
            self.feedback = "Document composition moves are not yet supported losslessly".into();
            cx.notify();
            return false;
        }
        // Preview validation is advisory. Dispatch the captured intent so Cordis
        // reports the authoritative kernel rejection as well as successful moves.
        self.submit_explorer_operation(
            project_id,
            revision,
            |source| match target {
                crate::components::model_explorer::ExplorerMoveTarget::Element(element_id) => {
                    source.set_owner(element_id, parent_id, revision)
                }
                crate::components::model_explorer::ExplorerMoveTarget::Diagram(diagram_id) => {
                    source.set_explorer_parent(diagram_id, parent_id, parent_occurrence, revision)
                }
            },
            cx,
        )
    }

    pub(crate) fn validate_explorer_diagram_reparent(
        &self,
        drag: &crate::components::model_explorer::ExplorerDiagramDrag,
        parent_id: Option<&str>,
        parent_occurrence: Option<&bellatrix_mde_cordis::ViewOccurrence>,
    ) -> Result<(), String> {
        if self.source.is_read_only()
            || self.source.is_presentation_suspended()
            || self.project_id() != &drag.project_id
            || self.source.snapshot()?.revision != drag.revision
        {
            return Err("The document changed or is not editable; drag the diagram again".into());
        }
        self.validate_explorer_move(
            &drag.project_id,
            drag.revision,
            &crate::components::model_explorer::ExplorerMoveTarget::Diagram(
                drag.diagram_id.clone(),
            ),
            parent_id,
            parent_occurrence,
        )
    }

    pub(crate) fn validate_explorer_reparent(
        &self,
        drag: &crate::components::model_explorer::ExplorerElementDrag,
        owner_id: &str,
    ) -> Result<(), String> {
        if self.source.is_read_only() || self.source.is_presentation_suspended() {
            return Err("The active document is not editable".into());
        }
        if self.project_id() != &drag.project_id {
            return Err("The dragged element belongs to another project".into());
        }
        let snapshot = self.source.snapshot()?;
        if snapshot.revision != drag.revision {
            return Err("The model changed during the drag; drag the element again".into());
        }
        if snapshot
            .section
            .composed_subjects
            .contains(&drag.element_id)
        {
            return Err("Document composition moves are not yet supported losslessly".into());
        }
        self.validate_explorer_move(
            &drag.project_id,
            drag.revision,
            &crate::components::model_explorer::ExplorerMoveTarget::Element(
                drag.element_id.clone(),
            ),
            Some(owner_id),
            None,
        )
    }

    pub(crate) fn reparent_explorer_element(
        &mut self,
        drag: &crate::components::model_explorer::ExplorerElementDrag,
        owner_id: String,
        cx: &mut Context<Self>,
    ) -> bool {
        self.move_explorer_item(
            &drag.project_id,
            drag.revision,
            &crate::components::model_explorer::ExplorerMoveTarget::Element(
                drag.element_id.clone(),
            ),
            Some(owner_id),
            None,
            cx,
        )
    }

    pub(crate) fn drop_explorer_element_on_diagram(
        &mut self,
        drag: &crate::components::model_explorer::ExplorerElementDrag,
        diagram_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.source.record_interaction(
            "explorer.diagram_drop.received",
            Some(&drag.element_id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        let placement = (|| {
            if self.source.is_read_only() || self.source.is_presentation_suspended() {
                return Err("The active document is not editable".to_owned());
            }
            if self.project_id() != &drag.project_id {
                return Err("The dragged element belongs to another project".to_owned());
            }
            let snapshot = self.source.snapshot()?;
            if snapshot.revision != drag.revision {
                return Err("The model changed during the drag; drag the element again".to_owned());
            }
            let diagram = snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or_else(|| "The target diagram no longer exists".to_owned())?;
            let placement = element_view_placement(diagram, &drag.element_id);
            if matches!(placement, ElementViewPlacement::NewAt(..))
                && (!self.source.allows_creation()
                    || !snapshot.section.tools.allows_new_views(diagram_id))
            {
                return Err("This diagram supports selecting existing views, but not creating a new occurrence yet".into());
            }
            Ok(placement)
        })();
        match placement {
            Err(error) => {
                self.source.record_interaction(
                    "explorer.diagram_drop.rejected",
                    Some(&drag.element_id),
                    bellatrix_mde_cordis::TraceOutcome::Invalid,
                );
                self.feedback = error;
            }
            Ok(ElementViewPlacement::Existing(view_id)) => {
                self.source.record_interaction(
                    "explorer.diagram_drop.existing_view",
                    Some(&view_id),
                    bellatrix_mde_cordis::TraceOutcome::Success,
                );
                self.select_diagram_view(diagram_id.to_owned(), view_id, cx);
                self.feedback = "Selected existing view; model ownership is unchanged".into();
            }
            Ok(ElementViewPlacement::NewAt(x, y)) => {
                self.submit_explorer_operation(
                    &drag.project_id,
                    drag.revision,
                    |source| {
                        source.place_element_view(diagram_id, &drag.element_id, x, y, drag.revision)
                    },
                    cx,
                );
            }
        }
        cx.notify();
    }

    pub(crate) fn drop_explorer_element(
        &mut self,
        drag: &crate::components::model_explorer::ExplorerElementDrag,
        position: (f64, f64),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.validate_explorer_drop(drag) {
            self.source.record_interaction(
                "explorer.canvas_drop.rejected",
                Some(&drag.element_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            self.feedback = error;
            cx.notify();
            return;
        }
        if !position.0.is_finite() || !position.1.is_finite() {
            self.source.record_interaction(
                "explorer.canvas_drop.rejected",
                Some(&drag.element_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            self.feedback = "The canvas drop position is unavailable".into();
            cx.notify();
            return;
        }
        self.show_element_in_active_diagram_at(
            drag.element_id.clone(),
            Some(position),
            Some(drag.revision),
            drag.source_view_id.clone(),
            window,
            cx,
        );
    }

    pub fn show_element_in_active_diagram(
        &mut self,
        element_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_element_in_active_diagram_at(element_id, None, None, None, window, cx);
    }

    pub(crate) fn show_element_in_active_diagram_at(
        &mut self,
        element_id: String,
        position: Option<(f64, f64)>,
        expected_revision: Option<u64>,
        preferred_view_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        if expected_revision.is_some_and(|revision| revision != snapshot.revision) {
            self.feedback = "The model changed during the drag; drag the element again".into();
            cx.notify();
            return;
        }
        let Some(diagram) = self.presentation.active_diagram().and_then(|id| {
            snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == id)
        }) else {
            self.feedback = "Select an active diagram before showing a model element".into();
            cx.notify();
            return;
        };
        let placement = if let Some(view_id) = preferred_view_id {
            if !diagram.all_views().any(|view| {
                view.id == view_id && view.subject.as_deref() == Some(element_id.as_str())
            }) {
                self.feedback = "The dragged occurrence no longer matches its element".into();
                cx.notify();
                return;
            }
            ElementViewPlacement::Existing(view_id)
        } else {
            element_view_placement(diagram, &element_id)
        };
        match placement {
            ElementViewPlacement::Existing(view_id) => {
                if let Some(position) = position
                    && let Some(center) = diagram
                        .all_views()
                        .find(|view| view.id == view_id)
                        .and_then(|view| {
                            crate::components::visual_draft_canvas::resolve_visual_center(
                                &view.geometry,
                            )
                        })
                {
                    // Reveal the existing occurrence at the drop point without moving its geometry.
                    let pan = (
                        f64::from(self.pan.0) + (position.0 - center.x) * f64::from(self.zoom),
                        f64::from(self.pan.1) + (position.1 - center.y) * f64::from(self.zoom),
                    );
                    if pan.0.is_finite()
                        && pan.1.is_finite()
                        && pan.0.abs() <= f64::from(f32::MAX)
                        && pan.1.abs() <= f64::from(f32::MAX)
                    {
                        self.pan = (pan.0 as f32, pan.1 as f32);
                    }
                }
                self.select_view(element_id, view_id, cx);
                self.feedback = "Selected existing view in active diagram".into();
                cx.notify();
            }
            ElementViewPlacement::NewAt(x, y) => {
                if !self.source.allows_creation()
                    || !snapshot.section.tools.allows_new_views(&diagram.id)
                {
                    self.feedback = "No existing view in this diagram; creating a new occurrence is not available yet".into();
                    cx.notify();
                    return;
                }
                let (x, y) = position.unwrap_or((x, y));
                self.select_element(element_id.clone(), cx);
                let result = self.source.place_element_view(
                    &diagram.id,
                    &element_id,
                    x.max(0.0),
                    y.max(0.0),
                    expected_revision.unwrap_or(snapshot.revision),
                );
                self.report_command_result(result, cx);
            }
        }
        self.focus_handle.focus(window, cx);
    }
}
