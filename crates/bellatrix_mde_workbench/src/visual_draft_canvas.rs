//! View-only notation uses the same canvas figures and styles without pretending to be UML.
use super::*;

impl MdeWorkbenchItem {
    pub(super) fn render_validation_mode(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.weak_entity();
        ui::Button::new("mde-validation-mode", self.validation_mode_label()).on_click(
            move |_, _, cx| {
                if let Err(error) = owner.update(cx, |item, cx| item.cycle_validation_mode(cx)) {
                    eprintln!("MDE visual view update failed: {error}");
                }
            },
        )
    }

    pub(super) fn is_pending_visual_endpoint(&self) -> bool {
        self.pending_relation_source.as_ref().is_some_and(|id| {
            self.source.snapshot().ok().is_some_and(|snapshot| {
                snapshot
                    .section
                    .diagrams
                    .iter()
                    .any(|diagram| diagram.views.iter().any(|view| &view.id == id))
            })
        })
    }

    pub(super) fn render_visual_draft(
        &self,
        view: &bellatrix_mde::View,
        diagram: &bellatrix_mde::Diagram,
        revision: u64,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let draft = view.draft.as_ref()?;
        let zoom = self.zoom;
        let selected = self.selected_view.as_deref() == Some(view.id.as_str());
        let label = view.label.clone().unwrap_or_default();
        let (mut x, mut y, mut width, mut height, figure) = match &view.geometry {
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => {
                let figure = if UmlFigure::supports(&draft.notation_kind) {
                    UmlFigure::new(draft.notation_kind.clone(), label)
                        .diagram_presentation(diagram)
                        .pseudostate_kind(
                            draft
                                .properties
                                .get("kind")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("initial"),
                        )
                        .view_style(&view.style)
                        .selected(selected)
                        .view_size(*width, *height, zoom)
                        .into_any_element()
                } else {
                    UmlClassifier::new(label)
                        .model_kind(&draft.notation_kind)
                        .view_style(&view.style)
                        .selected(selected)
                        .view_size(*width, *height, zoom)
                        .into_any_element()
                };
                (*x, *y, *width, *height, figure)
            }
            Geometry::Path { start, segments } => {
                let points = std::iter::once(*start)
                    .chain(segments.iter().map(|segment| match segment {
                        PathSegment::Line { to } | PathSegment::Cubic { to, .. } => *to,
                    }))
                    .collect::<Vec<_>>();
                let end = *points.last()?;
                let min_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::INFINITY, f64::min)
                    - 24.0;
                let min_y = points
                    .iter()
                    .map(|point| point.y)
                    .fold(f64::INFINITY, f64::min)
                    - 24.0;
                let width = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::NEG_INFINITY, f64::max)
                    - min_x
                    + 24.0;
                let height = points
                    .iter()
                    .map(|point| point.y)
                    .fold(f64::NEG_INFINITY, f64::max)
                    - min_y
                    + 24.0;
                let place = |point: bellatrix_mde::Point| {
                    (
                        ((point.x - min_x) as f32) * zoom,
                        ((point.y - min_y) as f32) * zoom,
                    )
                };
                let segments = segments
                    .iter()
                    .map(|segment| match segment {
                        PathSegment::Line { to } => UmlLineSegment::Line { to: place(*to) },
                        PathSegment::Cubic {
                            control1,
                            control2,
                            to,
                        } => UmlLineSegment::Cubic {
                            control1: place(*control1),
                            control2: place(*control2),
                            to: place(*to),
                        },
                    })
                    .collect();
                let figure = UmlRelationGlyph::new(
                    bellatrix_mde::relationship_style_for_notation(&draft.notation_kind),
                    width as f32 * zoom,
                    height as f32 * zoom,
                    place(*start),
                    place(end),
                )
                .segments(segments)
                .view_style(&view.style)
                .view_scale(zoom)
                .selected(selected)
                .into_any_element();
                (min_x, min_y, width, height, figure)
            }
            _ => return None,
        };
        match &self.gesture {
            Some(CanvasGesture::Move {
                view_id, dx, dy, ..
            }) if view_id == &view.id => {
                x += dx;
                y += dy;
            }
            Some(CanvasGesture::Resize {
                view_id, dx, dy, ..
            }) if view_id == &view.id => {
                width = (width + dx).max(8.0);
                height = (height + dy).max(8.0);
            }
            _ => {}
        }
        let owner = cx.weak_entity();
        let id = view.id.clone();
        let resize_owner = cx.weak_entity();
        let resize_id = view.id.clone();
        Some(
            div()
                .id(format!("mde-draft-{}", view.id))
                .absolute()
                .left(px(x as f32 * zoom + self.pan.0))
                .top(px(y as f32 * zoom + self.pan.1))
                .w(px(width as f32 * zoom))
                .h(px(height as f32 * zoom))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, window, cx| {
                        if let Err(error) = owner.update(cx, |item, cx| {
                            if let Some(ModelingTool::Relation(kind)) = item.selected_tool {
                                item.link_visual_draft_endpoint(kind, id.clone(), cx);
                            } else {
                                item.selected_element = None;
                                item.selected_view = Some(id.clone());
                                item.feedback =
                                    "Visual draft · not yet a semantic UML element".into();
                                if !item.source.is_read_only() {
                                    item.gesture = Some(CanvasGesture::Move {
                                        view_id: id.clone(),
                                        start: event.position,
                                        dx: 0.0,
                                        dy: 0.0,
                                        revision,
                                    });
                                }
                                cx.notify();
                            }
                            cx.focus_self(window);
                        }) {
                            eprintln!("MDE visual view update failed: {error}");
                        }
                        cx.stop_propagation();
                    },
                )
                .child(figure)
                .when(
                    selected && matches!(view.geometry, Geometry::Rect { .. }),
                    |element| {
                        element.child(
                            div()
                                .absolute()
                                .right_0()
                                .bottom_0()
                                .size(px(10.0))
                                .border_1()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    move |event: &MouseDownEvent, _, cx| {
                                        if let Err(error) = resize_owner.update(cx, |item, cx| {
                                            item.start_resize(
                                                resize_id.clone(),
                                                width,
                                                height,
                                                revision,
                                                event.position,
                                                cx,
                                            )
                                        }) {
                                            eprintln!("MDE visual view update failed: {error}");
                                        }
                                        cx.stop_propagation();
                                    },
                                ),
                        )
                    },
                )
                .into_any_element(),
        )
    }

    pub(super) fn link_visual_draft_endpoint(
        &mut self,
        kind: ModelingRelationKind,
        target: String,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let from = match choose_relation_endpoint(&mut self.pending_relation_source, &target) {
            RelationClick::First => {
                self.feedback = "Visual connection: select another view".into();
                cx.notify();
                return;
            }
            RelationClick::Repeat => return,
            RelationClick::Connect(from) => from,
        };
        let Ok(snapshot) = self.source.snapshot() else {
            return;
        };
        let Some(diagram) = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| Some(diagram.id.as_str()) == self.active_diagram.as_deref())
        else {
            return;
        };
        let from = diagram
            .views
            .iter()
            .find(|view| view.id == from || view.subject.as_deref() == Some(from.as_str()))
            .map(|view| view.id.clone())
            .unwrap_or(from);
        let target = diagram
            .views
            .iter()
            .find(|view| view.id == target || view.subject.as_deref() == Some(target.as_str()))
            .map(|view| view.id.clone())
            .unwrap_or(target);
        let locate = |id: &str| {
            diagram
                .views
                .iter()
                .find(|view| view.id == id)
                .and_then(|view| match view.geometry {
                    Geometry::Rect {
                        x,
                        y,
                        width,
                        height,
                    } => Some(bellatrix_mde::Point {
                        x: x + width / 2.0,
                        y: y + height / 2.0,
                    }),
                    _ => None,
                })
        };
        let (Some(start), Some(to)) = (locate(&from), locate(&target)) else {
            return;
        };
        self.submit(
            MdeUiCommand::CreateVisualDraft(
                bellatrix_mde_cordis::ModelingCreateVisualDraftRequested {
                    project_id: self.project_id().clone(),
                    diagram_id: diagram.id.clone(),
                    notation_kind: kind.model_kind().into(),
                    label: None,
                    geometry: Geometry::Path {
                        start,
                        segments: vec![PathSegment::Line { to }],
                    },
                    properties: std::collections::BTreeMap::new(),
                    parent_view_id: None,
                    source_view_id: Some(from),
                    target_view_id: Some(target),
                    expected_revision: snapshot.revision,
                },
            ),
            cx,
        );
        self.selected_tool = None;
    }
}

pub(super) fn format_figure_label(
    element: &bellatrix_mde::Element,
    references: &bellatrix_mde::uml::references::ReferenceIndex<'_>,
    style: &bellatrix_mde::Style,
) -> String {
    if element.kind == "uml.interaction_operand" {
        String::new()
    } else if element.kind == "uml.constraint" {
        format!("{{{}}}", element.name)
    } else if matches!(
        element.kind.as_str(),
        "uml.port" | "uml.part" | "uml.note" | "uml.comment" | "uml.instance_specification"
    ) {
        bellatrix_mde::format_element_label_for_view(element, references, style)
    } else {
        element.name.clone()
    }
}

pub(super) fn resolve_interaction_operator_default() -> Option<serde_json::Value> {
    use bellatrix_mde::uml::schema;
    schema::uml_property("CombinedFragment", "interactionOperator")
        .ok()
        .flatten()
        .and_then(|property| schema::effective_default(property).ok().flatten())
        .and_then(schema::DefaultValue::scalar_json)
}
