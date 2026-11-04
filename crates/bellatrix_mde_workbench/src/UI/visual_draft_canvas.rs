//! View-only notation uses the same canvas figures and styles without pretending to be UML.
use super::*;
pub(super) type CanvasOrigin = std::rc::Rc<std::cell::Cell<Option<(f64, f64)>>>;

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

    pub(super) fn select_or_connect_edge(
        &mut self,
        subject: String,
        view_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(ModelingTool::Relation(kind)) = self.presentation.selected_tool() {
            self.connect_relation_view(kind, view_id, None, cx);
        } else {
            self.select_view(subject, view_id, cx);
        }
    }

    pub(super) fn render_visual_draft(
        &self,
        view: &bellatrix_mde::View,
        diagram: &bellatrix_mde::Diagram,
        revision: u64,
        canvas_origin: &CanvasOrigin,
        node_bounds: &std::rc::Rc<std::cell::RefCell<Vec<[f32; 4]>>>,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let draft = view.draft.as_ref()?;
        let zoom = self.zoom;
        let selected = self.presentation.selected_view() == Some(view.id.as_str());
        let label = view.label.clone().unwrap_or_default();
        let (mut x, mut y, mut width, mut height, figure) = match &view.geometry {
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => {
                let interface_presentation = draft
                    .properties
                    .get("interface_presentation")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| matches!(*value, "provided" | "required"));
                let figure = if UmlFigure::supports(&draft.notation_kind)
                    || (draft.notation_kind == "uml.interface" && interface_presentation.is_some())
                {
                    UmlFigure::new(draft.notation_kind.clone(), label)
                        .interface_presentation(interface_presentation)
                        .diagram_presentation(diagram)
                        .timing_presentation(view.timing.as_ref())
                        .composite_role(
                            draft
                                .properties
                                .get("aggregation")
                                .and_then(serde_json::Value::as_str)
                                == Some("composite"),
                        )
                        .interaction_operator(
                            resolve_interaction_operator(&draft.properties).as_deref(),
                        )
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
                let mut points = vec![*start];
                for segment in segments {
                    match segment {
                        PathSegment::Line { to } => points.push(*to),
                        PathSegment::Cubic { control1, control2, to } => points.extend([*control1, *control2, *to]),
                    }
                }
                let padding = 24.0_f64.max(5.0 / f64::from(zoom));
                let end = *points.last()?;
                let min_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::INFINITY, f64::min)
                    - padding;
                let min_y = points
                    .iter()
                    .map(|point| point.y)
                    .fold(f64::INFINITY, f64::min)
                    - padding;
                let width = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::NEG_INFINITY, f64::max)
                    - min_x
                    + padding;
                let height = points
                    .iter()
                    .map(|point| point.y)
                    .fold(f64::NEG_INFINITY, f64::max)
                    - min_y
                    + padding;
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
                let mut label_properties = draft.properties.clone();
                if !label.is_empty() {
                    label_properties.insert("name".into(), serde_json::json!(label));
                }
                let presentation = bellatrix_mde::RelationshipPresentation {
                    name: bellatrix_mde::format_relationship_label(
                        &draft.notation_kind,
                        &label_properties,
                    ),
                    source_labels: Default::default(),
                    target_labels: Default::default(),
                    label: label.clone(),
                    notation: draft.notation_kind.clone(),
                    style: bellatrix_mde::resolve_relationship_style(
                        &draft.notation_kind,
                        &draft.properties,
                    ),
                    source_role: "source",
                    target_role: "target",
                };
                let figure = UmlRelationGlyph::new(
                    presentation.style,
                    width as f32 * zoom,
                    height as f32 * zoom,
                    place(*start),
                    place(end),
                )
                .segments(segments)
                .diagram_presentation(diagram, &draft.notation_kind)
                .labels(presentation)
                .view_style(&view.style)
                .view_scale(zoom)
                .selected(selected)
                .into_any_element();
                (min_x, min_y, width, height, figure)
            }
            _ => return None,
        };
        let original_position = (x, y);
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
        if matches!(&view.geometry, Geometry::Rect { .. }) {
            node_bounds.borrow_mut().push([
                x as f32 * zoom + self.pan.0,
                y as f32 * zoom + self.pan.1,
                width as f32 * zoom,
                height as f32 * zoom,
            ]);
        }
        let hit_geometry = matches!(&view.geometry, Geometry::Path { .. }).then(|| view.geometry.clone());
        let hit_nodes = node_bounds.clone();
        let hit_pan = (
            self.pan.0 + (x - original_position.0) as f32 * zoom,
            self.pan.1 + (y - original_position.1) as f32 * zoom,
        );
        let owner = cx.weak_entity();
        let click_origin = canvas_origin.clone();
        let id = view.id.clone();
        let draft_editor = self
            .inline_editor
            .clone()
            .filter(|_| self.presentation.editing_view() == Some(view.id.as_str()))
            .map(|field| {
                let confirm = cx.weak_entity();
                let cancel = cx.weak_entity();
                let editor_id = field.entity_id();
                let target = view.id.clone();
                let cancel_target = target.clone();
                bellatrix_ui_uml::UmlInlineNameEditor::new(
                    field,
                    move |window, cx| {
                        if let Err(error) = confirm.update(cx, |item, cx| {
                            item.commit_draft_label(&target, editor_id, window, cx)
                        }) {
                            eprintln!("MDE draft label update failed: {error}");
                        }
                    },
                    move |window, cx| {
                        if let Err(error) = cancel.update(cx, |item, cx| {
                            if item.presentation.editing_view() == Some(cancel_target.as_str())
                                && item
                                    .inline_editor
                                    .as_ref()
                                    .is_some_and(|field| field.entity_id() == editor_id)
                            {
                                item.cancel_inline_edit(window, cx);
                            }
                        }) {
                            eprintln!("MDE draft label cancel failed: {error}");
                        }
                    },
                )
            });
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
                .when(hit_geometry.is_none(), |element| element.cursor_pointer())
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, window, cx| {
                        if hit_geometry.as_ref().is_some_and(|geometry| {
                            !super::diagram_canvas::relationship_hit_test(
                                geometry, event.position, click_origin.get(), hit_pan, zoom, &hit_nodes.borrow(),
                            )
                        }) {
                            return;
                        }
                        if let Err(error) = owner.update(cx, |item, cx| {
                            if event.click_count >= 2 && item.presentation.selected_tool().is_none() {
                                item.begin_draft_label_edit(id.clone(), window, cx);
                            } else if matches!(item.presentation.selected_tool(), Some(ModelingTool::Relation(_)))
                            {
                                item.connect_view_at_pointer(
                                    id.clone(),
                                    event.position,
                                    click_origin.get(),
                                    cx,
                                );
                            } else {
                                let change = item.presentation.select_view(None, id.clone());
                                item.apply_selection_change(change, cx);
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
                .when_some(draft_editor, |element, editor| {
                    element.child(div().absolute().top_0().left_0().w_full().child(editor))
                })
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
}

pub(super) fn format_figure_label(
    element: &bellatrix_mde::Element,
    references: &bellatrix_mde::uml::references::ReferenceIndex<'_>,
    style: &bellatrix_mde::Style,
) -> String {
    if element.kind == "uml.interaction_operand" {
        bellatrix_mde::format_operand_guard(element, references)
    } else if element.kind == "uml.constraint" {
        format!("{{{}}}", element.name)
    } else if matches!(
        element.kind.as_str(),
        "uml.port"
            | "uml.part"
            | "uml.note"
            | "uml.comment"
            | "uml.instance_specification"
            | "uml.collaboration_use"
    ) {
        bellatrix_mde::format_element_label_for_view(element, references, style)
    } else {
        element.name.clone()
    }
}

pub(super) fn resolve_interaction_operator(
    properties: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    properties
        .get("interactionOperator")
        .cloned()
        .or_else(resolve_interaction_operator_default)
        .and_then(|value| value.as_str().map(str::to_owned))
}

pub(super) fn resolve_interaction_operator_default() -> Option<serde_json::Value> {
    use bellatrix_mde::uml::schema;
    schema::uml_property("CombinedFragment", "interactionOperator")
        .ok()
        .flatten()
        .and_then(|property| schema::effective_default(property).ok().flatten())
        .and_then(schema::DefaultValue::scalar_json)
}

impl MdeWorkbenchItem {
    pub(super) fn begin_draft_label_edit(
        &mut self,
        view_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            return;
        };
        let Some(view) = snapshot
            .section
            .diagrams
            .iter()
            .flat_map(|diagram| &diagram.views)
            .find(|view| view.id == view_id && view.draft.is_some())
        else {
            return;
        };
        if self.presentation.editing_view() == Some(view_id.as_str()) && self.inline_editor.is_some()
        {
            return;
        }
        let field = cx.new(|cx| InputField::new(window, cx, "Label").label_min_width(px(96.0)));
        field.update(cx, |field, cx| {
            field.set_text(view.label.as_deref().unwrap_or(""), window, cx)
        });
        self.presentation.begin_view_edit(view_id, snapshot.revision);
        self.inline_editor = Some(field.clone());
        self.inline_draft = None;
        self.gesture = None;
        field.read(cx).focus_handle(cx).focus(window, cx);
        field.read(cx).editor().clone().select_all(window, cx);
        cx.notify();
    }
    fn commit_draft_label(
        &mut self,
        target: &str,
        editor_id: gpui::EntityId,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only()
            || self.presentation.draft_label_pending()
            || self.presentation.editing_view() != Some(target)
            || !self
                .inline_editor
                .as_ref()
                .is_some_and(|field| field.entity_id() == editor_id)
        {
            return;
        }
        let (Some(view_id), Some(field), Some(expected_revision)) = (
            self.presentation.editing_view().map(str::to_owned),
            self.inline_editor.clone(),
            self.presentation.draft_label_revision(),
        ) else {
            return;
        };
        let label = field.read(cx).text(cx).to_string();
        let project_id = self.project_id().clone();
        let (completion, mut receipt) = tokio::sync::watch::channel(None);
        let request = bellatrix_mde_cordis::ModelingSetViewLabelRequested {
            project_id: project_id.clone(),
            view_id: view_id.clone(),
            label: Some(label.clone()),
            expected_revision,
        };
        if let Err(error) = self.source.submit(MdeUiCommand::SetViewLabelTracked {
            request,
            completion,
        }) {
            self.feedback = format!("Label not sent; draft preserved: {error}");
            cx.notify();
            return;
        }
        self.presentation.view_label_sent();
        self.feedback = "Saving view label…".into();
        cx.spawn(async move |owner, cx| {
            let waiting = receipt.borrow().is_none();
            if waiting && let Err(error) = receipt.changed().await {
                eprintln!("MDE label receipt unavailable: {error}");
            }
            let result = receipt
                .borrow()
                .clone()
                .unwrap_or_else(|| Err("Label receipt unavailable; draft preserved".into()));
            if let Err(error) = owner.update(cx, |item, cx| {
                let same_editor = item.project_id() == &project_id
                    && item.presentation.editing_view() == Some(view_id.as_str())
                    && item
                        .inline_editor
                        .as_ref()
                        .is_some_and(|current| current.entity_id() == field.entity_id());
                if !same_editor {
                    return;
                }
                match result {
                    Ok(revision) => {
                        item.presentation.view_label_completed(Some(revision));
                        if field.read(cx).text(cx).as_str() == label.as_str() {
                            item.presentation.clear_edit();
                            item.inline_editor = None;
                        }
                        item.feedback = "View label saved".into();
                    }
                    Err(error) => {
                        item.presentation.view_label_completed(None);
                        if let Ok(latest) = item.source.snapshot()
                            && latest.section.diagrams.iter().flat_map(|diagram| &diagram.views).any(|view| view.id == view_id && view.draft.is_some())
                        {
                            item.presentation.view_label_completed(Some(latest.revision));
                            item.feedback = format!("Label not saved; text preserved: {error}. Press Enter to replace the current label at revision {}", latest.revision);
                        } else {
                            item.feedback = format!("Label not saved; text preserved: {error}");
                        }
                    }
                }
                cx.notify();
            }) {
                eprintln!("MDE label receipt update failed: {error}");
            }
        })
        .detach();
        cx.notify();
    }
}

impl MdeWorkbenchItem {
    pub(super) fn connect_view_at_pointer(
        &mut self,
        view_id: String,
        position: Point<Pixels>,
        origin: Option<(f64, f64)>,
        cx: &mut Context<Self>,
    ) {
        let Some(ModelingTool::Relation(kind)) = self.presentation.selected_tool() else {
            return;
        };
        let point = origin.map(|origin| {
            let (x, y) = canvas_model_position(
                (f64::from(position.x), f64::from(position.y)),
                origin,
                self.pan,
                self.zoom,
            );
            bellatrix_mde::Point { x, y }
        });
        self.connect_relation_view(kind, view_id, point, cx);
    }
}
pub(super) fn resolve_message_anchor(
    section: &bellatrix_mde::ModelingSection,
    diagram: &bellatrix_mde::Diagram,
    view_id: &str,
    point: bellatrix_mde::Point,
) -> Option<bellatrix_mde::Point> {
    let view = diagram.views.iter().find(|view| view.id == view_id)?;
    let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = view.geometry
    else {
        return None;
    };
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let kind = view
        .draft
        .as_ref()
        .map(|draft| draft.notation_kind.as_str())
        .or_else(|| {
            section
                .elements
                .iter()
                .find(|element| view.subject.as_deref() == Some(element.id.as_str()))
                .map(|element| element.kind.as_str())
        });
    let anchor_x = if kind == Some("uml.lifeline")
        && diagram
            .interaction_kind
            .as_deref()
            .is_none_or(|kind| kind == "sequence")
    {
        0.5
    } else {
        ((point.x - x) / width).clamp(0.0, 1.0)
    };
    Some(bellatrix_mde::Point {
        x: anchor_x,
        y: ((point.y - y) / height).clamp(0.0, 1.0),
    })
}
