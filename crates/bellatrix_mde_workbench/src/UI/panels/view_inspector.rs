use super::*;
use bellatrix_mde_cordis::Geometry;

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
        let selected_view = item
            .read(cx)
            .presentation
            .selected_view()
            .map(str::to_owned);
        let style_target = snapshot.as_ref().and_then(|snapshot| {
            let diagram = snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| Some(diagram.id.as_str()) == active_diagram.as_deref())?;
            let view = diagram
                .all_views()
                .find(|view| Some(view.id.as_str()) == selected_view.as_deref())?;
            let is_edge = matches!(view.geometry, Geometry::Path { .. })
                || snapshot
                    .section
                    .relations
                    .iter()
                    .any(|relation| view.subject.as_deref() == Some(relation.id.as_str()));
            Some((diagram.id.as_str(), view, is_edge))
        });
        let read_only = read_only
            || style_target.is_some_and(|(diagram_id, view, _)| {
                snapshot.as_ref().is_none_or(|snapshot| {
                    snapshot
                        .section
                        .diagrams
                        .iter()
                        .find(|diagram| diagram.id == diagram_id)
                        .is_none_or(|diagram| {
                            !crate::workbench_ui::selection::supports_view_editing(
                                diagram, &view.id,
                            )
                        })
                })
            });
        let style_target_id = style_target.map(|(diagram_id, view, _)| {
            (
                inspector_project.as_str().to_owned(),
                diagram_id.to_owned(),
                view.id.clone(),
            )
        });
        let editable_target_id = style_target_id.clone().filter(|_| !read_only);
        if self.style_target_id != editable_target_id {
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
            self.style_target_id = editable_target_id;
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
            read_only,
            self.inspector_styles_open,
            window,
            cx,
        );
        if let Some((diagram_id, view, _)) = style_target {
            let mut presentation_rows = vec![
                presentation_row("View", &view.id),
                presentation_row(
                    "Kind",
                    view.presentation.kind.as_deref().unwrap_or("Native view"),
                ),
                presentation_row("Subject", view.subject.as_deref().unwrap_or("None")),
                presentation_row("Label", view.label.as_deref().unwrap_or("Absent")),
            ];
            if read_only {
                presentation_rows.push(presentation_row("Editing", "Read-only presentation"));
                if let Some(notice) = style_target_id
                    .as_ref()
                    .and_then(|key| self.timing_drafts.get(key))
                    .and_then(|draft| draft.inspection_notice())
                {
                    presentation_rows.push(presentation_row("Timing draft", notice));
                }
            }
            match &view.geometry {
                Geometry::Layout { bounds } => {
                    presentation_rows.push(presentation_row("Geometry", "Authored layout"));
                    for (name, value) in [
                        ("X", &bounds.x),
                        ("Y", &bounds.y),
                        ("Width", &bounds.width),
                        ("Height", &bounds.height),
                    ] {
                        let value = match value {
                            bellatrix_mde_cordis::Authored::Missing => "Absent".to_owned(),
                            bellatrix_mde_cordis::Authored::Null => "Null".to_owned(),
                            bellatrix_mde_cordis::Authored::Value(value) => value.to_string(),
                        };
                        presentation_rows.push(presentation_row(name, &value));
                    }
                }
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    presentation_rows.push(presentation_row("Geometry", "Rectangle"));
                    for (name, value) in [("X", x), ("Y", y), ("Width", width), ("Height", height)]
                    {
                        presentation_rows.push(presentation_row(name, &value.to_string()));
                    }
                }
                Geometry::Ellipse { cx, cy, rx, ry } => {
                    presentation_rows.push(presentation_row("Geometry", "Ellipse"));
                    for (name, value) in [
                        ("Center X", cx),
                        ("Center Y", cy),
                        ("Radius X", rx),
                        ("Radius Y", ry),
                    ] {
                        presentation_rows.push(presentation_row(name, &value.to_string()));
                    }
                }
                Geometry::Polygon { points } => {
                    presentation_rows.push(presentation_row("Geometry", "Polygon"));
                    presentation_rows.push(presentation_row("Points", &points.len().to_string()));
                }
                Geometry::Path { start, segments } => {
                    presentation_rows.push(presentation_row("Geometry", "Path"));
                    presentation_rows.push(presentation_row("Start X", &start.x.to_string()));
                    presentation_rows.push(presentation_row("Start Y", &start.y.to_string()));
                    presentation_rows
                        .push(presentation_row("Segments", &segments.len().to_string()));
                }
            }
            if let Some(diagram) = snapshot.as_ref().and_then(|snapshot| {
                snapshot
                    .section
                    .diagrams
                    .iter()
                    .find(|diagram| diagram.id == diagram_id)
            }) {
                let parent = diagram
                    .all_views()
                    .find(|parent| parent.children.iter().any(|child| child.id == view.id));
                if let Some(parent) = parent {
                    presentation_rows.push(view_navigation(
                        &item,
                        diagram_id,
                        parent,
                        "Parent",
                        IconName::ChevronUp,
                    ));
                }
                for child in &view.children {
                    presentation_rows.push(view_navigation(
                        &item,
                        diagram_id,
                        child,
                        "Subview",
                        IconName::ChevronRight,
                    ));
                }
            }
            presentation_rows.append(&mut inspector_rows);
            inspector_rows = presentation_rows;
        }
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

fn presentation_row(name: &str, value: &str) -> AnyElement {
    crate::workbench_ui::property_editor::compact_row(
        name.to_owned(),
        Label::new(value.to_owned()).size(LabelSize::Small),
    )
}

fn view_navigation(
    item: &Entity<MdeWorkbenchItem>,
    diagram_id: &str,
    view: &bellatrix_mde_cordis::projection::View,
    relationship: &str,
    icon: IconName,
) -> AnyElement {
    let owner = item.clone();
    let diagram_id = diagram_id.to_owned();
    let view_id = view.id.clone();
    let label = view
        .label
        .as_deref()
        .filter(|label| !label.is_empty())
        .unwrap_or(&view.id);
    let tooltip = format!("Inspect {relationship}: {label} · {}", view.id);
    div()
        .px_3()
        .py_1()
        .flex()
        .items_center()
        .gap_1()
        .child(
            IconButton::new(format!("mde-inspect-view-{}", view.id), icon)
                .aria_label(tooltip.clone())
                .tooltip(ui::Tooltip::text(tooltip))
                .on_click(move |_, _, cx| {
                    owner.update(cx, |workbench, cx| {
                        workbench.select_diagram_view(diagram_id.clone(), view_id.clone(), cx)
                    });
                }),
        )
        .child(
            Label::new(label.to_owned())
                .size(LabelSize::Small)
                .truncate(),
        )
        .into_any_element()
}
