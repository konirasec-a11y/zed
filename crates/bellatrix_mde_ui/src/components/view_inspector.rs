use crate::layouts::sidebar::MdeRightPanel;
use crate::pages::mde_page::MdeWorkbenchItem;
use bellatrix_mde_cordis::Geometry;
use gpui::Context;
use gpui::Entity;
use gpui::IntoElement;
use gpui::Window;
use gpui::div;
use gpui::prelude::*;
use ui::IconButton;
use ui::IconName;
use ui::Label;
use ui::prelude::*;

impl MdeRightPanel {
    pub(crate) fn render_view_presentation(
        &mut self,
        item: Entity<MdeWorkbenchItem>,
        snapshot: &Option<crate::view_model::snapshot::MdeSnapshot>,
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
                            !crate::view_model::selection::supports_view_style_editing(
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
                crate::components::style_editor::reconcile_style_inputs(
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
        let mut inspector_rows = crate::components::style_editor::render_styles(
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
                presentation_row("View", view_label(view)),
                presentation_row(
                    "Kind",
                    view.presentation.kind.as_deref().unwrap_or("Native view"),
                ),
                presentation_row("Subject", subject_label(view, snapshot.as_ref())),
                presentation_row(
                    if view.rendered_label.bound {
                        "Stored label"
                    } else {
                        "Label"
                    },
                    view.label.as_deref().unwrap_or("Absent"),
                ),
            ];
            if let Some(draft) = &view.draft {
                presentation_rows.push(presentation_row(
                    "Model status",
                    "Visual draft only; no semantic UML element",
                ));
                if let Some(diagnostic) = &draft.semantic_creation_diagnostic {
                    presentation_rows
                        .push(presentation_row("Semantic creation rejected", diagnostic));
                }
            }
            if view.rendered_label.bound {
                presentation_rows.push(presentation_row(
                    "Rendered label",
                    view.rendered_label.text.as_deref().unwrap_or("Absent"),
                ));
            }
            for diagnostic in &view.rendered_label.diagnostics {
                presentation_rows.push(presentation_row("Label projection", diagnostic));
            }
            if let Some(icon) = &view.interface_icon {
                for diagnostic in &icon.diagnostics {
                    presentation_rows.push(presentation_row("Interface icon", diagnostic));
                }
            }
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
                    for (name, value) in [("X", &bounds.x), ("Y", &bounds.y)] {
                        let value = match value {
                            bellatrix_mde_cordis::Authored::Missing => "Absent".to_owned(),
                            bellatrix_mde_cordis::Authored::Null => "Null".to_owned(),
                            bellatrix_mde_cordis::Authored::Value(value) => value.to_string(),
                        };
                        presentation_rows.push(presentation_row(name, &value));
                    }
                    for (name, extent) in [("Width", &bounds.width), ("Height", &bounds.height)] {
                        let value = match extent {
                            bellatrix_mde_cordis::Authored::Missing => "Absent".to_owned(),
                            bellatrix_mde_cordis::Authored::Null => "Null".to_owned(),
                            bellatrix_mde_cordis::Authored::Value(extent) => match extent {
                                bellatrix_mde_cordis::LayoutExtent::Resolved(value) => {
                                    value.to_string()
                                }
                                bellatrix_mde_cordis::LayoutExtent::Unresolved {
                                    original, ..
                                } => format!(
                                    "Unresolved negative extent ({original}); layout required"
                                ),
                            },
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
                    .is_some_and(|diagram| {
                        diagram.interaction_kind.as_deref() == Some("timing")
                            && crate::view_model::selection::supports_view_editing(
                                diagram, &view.id,
                            )
                    });
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
                let key = crate::components::timing_editor::reconcile_timing_inputs(
                    &mut self.timing_drafts,
                    &item,
                    diagram_id,
                    view,
                    snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
                    window,
                    cx,
                );
                if let Some(draft) = self.timing_drafts.get(&key) {
                    inspector_rows.push(crate::components::timing_editor::render_timing_editor(
                        draft, &key, &item, cx,
                    ));
                }
            }
        }
        inspector_rows
    }
}

fn presentation_row(name: &str, value: &str) -> AnyElement {
    crate::components::inspector::compact_row(
        name.to_owned(),
        Label::new(value.to_owned()).size(LabelSize::Small),
    )
}

fn view_label(view: &bellatrix_mde_cordis::projection::View) -> &str {
    view.rendered_label
        .text
        .as_deref()
        .filter(|label| !label.trim().is_empty())
        .or_else(|| (!view.display_label.trim().is_empty()).then_some(view.display_label.as_str()))
        .or_else(|| {
            view.label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
        })
        .unwrap_or("Unnamed view")
}

fn subject_label<'a>(
    view: &bellatrix_mde_cordis::projection::View,
    snapshot: Option<&'a crate::view_model::snapshot::MdeSnapshot>,
) -> &'a str {
    let Some(subject) = view.subject.as_deref() else {
        return "None";
    };
    let Some(snapshot) = snapshot else {
        return "Unresolved reference";
    };
    if let Some(element) = snapshot
        .section
        .elements
        .iter()
        .find(|element| element.id == subject)
    {
        return if element.name.trim().is_empty() {
            "Unnamed element"
        } else {
            &element.name
        };
    }
    if let Some(relation) = snapshot
        .section
        .relations
        .iter()
        .find(|relation| relation.id == subject)
    {
        return relation
            .presentation
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&relation.presentation.type_label);
    }
    "Unresolved reference"
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
    let label = view_label(view);
    let tooltip = format!("Inspect {relationship}: {label}");
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
