use super::*;
use bellatrix_mde::Geometry;

impl MdeRightPanel {
    pub(super) fn render_view_presentation(
        &mut self,
        item: Entity<MdeWorkbenchItem>,
        snapshot: &Option<crate::workbench_ui::snapshot::MdeSnapshot>,
        active_diagram: &Option<String>,
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let inspector_project = item.read(cx).project_id().clone();
        let selected_view = item.read(cx).presentation.selected_view().map(str::to_owned);
        let style_target = snapshot.as_ref().and_then(|snapshot| {
            let diagram = snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| Some(diagram.id.as_str()) == active_diagram.as_deref())?;
            let view = diagram
                .views
                .iter()
                .find(|view| Some(view.id.as_str()) == selected_view.as_deref())?;
            let is_edge = matches!(view.geometry, Geometry::Path { .. })
                || snapshot
                    .section
                    .relations
                    .iter()
                    .any(|relation| view.subject.as_deref() == Some(relation.id.as_str()));
            Some((diagram.id.as_str(), view, is_edge))
        });
        let style_target_id = style_target.map(|(diagram_id, view, _)| {
            (
                inspector_project.as_str().to_owned(),
                diagram_id.to_owned(),
                view.id.clone(),
            )
        });
        if !read_only && self.style_target_id != style_target_id {
            if self
                .style_target_id
                .as_ref()
                .and_then(|key| self.style_drafts.get(key))
                .is_some_and(|draft| {
                    draft
                        .inputs
                        .values()
                        .any(|input| input.read(cx).focus_handle(cx).contains_focused(window, cx))
                })
                || self
                    .style_target_id
                    .as_ref()
                    .and_then(|key| self.timing_drafts.get(key))
                    .is_some_and(|draft| draft.has_focused_input(window, cx))
            {
                window.focus(&self.state.focus, cx);
            }
            self.style_target_id = style_target_id.clone();
        }
        if !read_only {
            if let Some((diagram_id, view, _)) = style_target {
                crate::workbench_ui::style_editor::reconcile_style_inputs(
                    &mut self.style_drafts,
                    &item,
                    diagram_id,
                    view,
                    snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
                    window,
                    cx,
                );
            }
        }
        let mut inspector_rows = crate::workbench_ui::style_editor::render_styles(
            style_target,
            item.clone(),
            if read_only {
                None
            } else {
                style_target_id
                    .as_ref()
                    .and_then(|key| self.style_drafts.get(key))
            },
            self.inspector_styles_open,
            window,
            cx,
        );
        if !read_only && let Some((diagram_id, view, _)) = style_target {
            let is_timing_lifeline = snapshot.as_ref().is_some_and(|snapshot| {
                let timing_diagram = snapshot
                    .section
                    .diagrams
                    .iter()
                    .find(|diagram| diagram.id == diagram_id)
                    .is_some_and(|diagram| diagram.interaction_kind.as_deref() == Some("timing"));
                let kind = view
                    .draft
                    .as_ref()
                    .map(|draft| draft.notation_kind.as_str())
                    .or_else(|| {
                        snapshot
                            .section
                            .elements
                            .iter()
                            .find(|element| view.subject.as_deref() == Some(element.id.as_str()))
                            .map(|element| element.kind.as_str())
                    });
                timing_diagram && kind == Some("uml.lifeline")
            });
            if is_timing_lifeline {
                let key = crate::workbench_ui::timing_editor::reconcile_timing_inputs(
                    &mut self.timing_drafts,
                    &item,
                    diagram_id,
                    view,
                    snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
                    window,
                    cx,
                );
                if let Some(draft) = self.timing_drafts.get(&key) {
                    inspector_rows.push(crate::workbench_ui::timing_editor::render_timing_editor(
                        draft, &key, &item, cx,
                    ));
                }
            }
        }
        inspector_rows
    }
}
