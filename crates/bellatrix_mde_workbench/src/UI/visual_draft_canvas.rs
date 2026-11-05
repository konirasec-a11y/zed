//! View-only notation uses the same canvas figures and styles without pretending to be UML.
use super::*;
use bellatrix_mde_cordis::presentation_contract::UmlNotation;
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
        view: &bellatrix_mde_cordis::projection::View,
        diagram: &bellatrix_mde_cordis::projection::Diagram,
        revision: u64,
        canvas_origin: &CanvasOrigin,
        node_bounds: &std::rc::Rc<std::cell::RefCell<Vec<[f32; 4]>>>,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let draft = view.draft.as_ref()?;
        let selection_feedback = draft.semantic_creation_diagnostic.as_ref().map_or_else(
            || "Visual draft · not yet a semantic UML element".to_owned(),
            |diagnostic| format!("Semantic creation rejected; visual draft only: {diagnostic}"),
        );
        let zoom = self.zoom;
        let selected = self.presentation.selected_view() == Some(view.id.as_str());
        let label = view.display_label.clone();
        let geometry = self.path_preview_geometry(view);
        let (mut x, mut y, mut width, mut height, figure) = match &geometry {
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => {
                let figure = if UmlFigure::supports(&view.figure) || view.interface_icon.is_some() {
                    UmlFigure::new(view.figure.clone(), label)
                        .interface_icon(view.interface_icon.clone())
                        .view_style(&view.render_style)
                        .selected(selected)
                        .view_size(*width, *height, zoom)
                        .into_any_element()
                } else if view.figure.notation == UmlNotation::ClassifierRectangle {
                    UmlClassifier::new(label)
                        .header(view.figure.header.clone())
                        .presentation(view.classifier)
                        .view_style(&view.render_style)
                        .selected(selected)
                        .view_size(*width, *height, zoom)
                        .into_any_element()
                } else {
                    div()
                        .size_full()
                        .border_1()
                        .border_color(WorkbenchPalette::current(cx).border)
                        .child(Label::new(format!(
                            "Unsupported notation: {}",
                            draft.notation_kind
                        )))
                        .into_any_element()
                };
                (*x, *y, *width, *height, figure)
            }
            Geometry::Path { start, segments } => {
                let mut points = vec![*start];
                for segment in segments {
                    match segment {
                        PathSegment::Line { to } => points.push(*to),
                        PathSegment::Cubic {
                            control1,
                            control2,
                            to,
                        } => points.extend([*control1, *control2, *to]),
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
                let place = |point: bellatrix_mde_cordis::Point| {
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
                let resolved = view.relationship.as_ref()?;
                let figure = UmlRelationGlyph::new(
                    resolved.style,
                    width as f32 * zoom,
                    height as f32 * zoom,
                    place(*start),
                    place(end),
                )
                .segments(segments)
                .communication_message(resolved.communication_message)
                .labels(resolved.presentation.clone())
                .view_style(&view.render_style)
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
        let hit_geometry = matches!(&geometry, Geometry::Path { .. }).then(|| geometry.clone());
        let hit_nodes = node_bounds.clone();
        let hit_pan = (
            self.pan.0 + (x - original_position.0) as f32 * zoom,
            self.pan.1 + (y - original_position.1) as f32 * zoom,
        );
        let owner = cx.weak_entity();
        let click_origin = canvas_origin.clone();
        let id = view.id.clone();
        let diagram_id = diagram.id.clone();
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
                            item.commit_view_label(&target, editor_id, window, cx)
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
                                geometry,
                                event.position,
                                click_origin.get(),
                                hit_pan,
                                zoom,
                                &hit_nodes.borrow(),
                            )
                        }) {
                            return;
                        }
                        if let Err(error) = owner.update(cx, |item, cx| {
                            if event.click_count >= 2 && item.presentation.selected_tool().is_none()
                            {
                                item.begin_view_label_edit(id.clone(), window, cx);
                                return;
                            } else if matches!(
                                item.presentation.selected_tool(),
                                Some(ModelingTool::Relation(_))
                            ) {
                                item.connect_view_at_pointer(
                                    id.clone(),
                                    event.position,
                                    click_origin.get(),
                                    cx,
                                );
                            } else {
                                let selected =
                                    item.presentation.selected_canvas_view() == Some(id.as_str());
                                let change = item.presentation.select_view(None, id.clone());
                                item.apply_selection_change(change, cx);
                                item.feedback = selection_feedback.clone();
                                if let Some(geometry) = &hit_geometry {
                                    if selected && let Some(origin) = click_origin.get() {
                                        let (x, y) = canvas_model_position(
                                            (
                                                f64::from(event.position.x),
                                                f64::from(event.position.y),
                                            ),
                                            origin,
                                            hit_pan,
                                            zoom,
                                        );
                                        if let Some(handle) =
                                            super::diagram_canvas::path_segment_at_position(
                                                geometry,
                                                bellatrix_mde_cordis::Point { x, y },
                                                zoom,
                                            )
                                        {
                                            item.start_path_edit(
                                                diagram_id.clone(),
                                                id.clone(),
                                                revision,
                                                handle,
                                                event.position,
                                                cx,
                                            );
                                        }
                                    }
                                } else if !item.source.is_read_only() {
                                    item.gesture = Some(CanvasGesture::Move {
                                        view_id: id.clone(),
                                        start: event.position,
                                        dx: 0.0,
                                        dy: 0.0,
                                        revision,
                                        began: false,
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

impl MdeWorkbenchItem {
    pub(super) fn begin_view_label_edit(
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
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.id == view_id && view.subject.is_none())
        else {
            return;
        };
        if self.presentation.editing_view() == Some(view_id.as_str())
            && let Some(field) = self.inline_editor.as_ref()
        {
            field.read(cx).focus_handle(cx).focus(window, cx);
            return;
        }
        let field = cx.new(|cx| InputField::new(window, cx, "Label").label_min_width(px(96.0)));
        field.update(cx, |field, cx| {
            field.set_text(view.label.as_deref().unwrap_or(""), window, cx)
        });
        self.presentation
            .begin_view_edit(view_id, snapshot.revision);
        self.inline_editor = Some(field.clone());
        self.inline_draft = None;
        self.gesture = None;
        field.read(cx).focus_handle(cx).focus(window, cx);
        field.read(cx).editor().clone().select_all(window, cx);
        cx.notify();
    }
    pub(super) fn commit_view_label(
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
            bellatrix_mde_cordis::Point { x, y }
        });
        self.connect_relation_view(kind, view_id, point, cx);
    }
}
pub(super) fn resolve_message_anchor(
    _section: &bellatrix_mde_cordis::projection::ModelingProjection,
    diagram: &bellatrix_mde_cordis::projection::Diagram,
    view_id: &str,
    point: bellatrix_mde_cordis::Point,
) -> Option<bellatrix_mde_cordis::Point> {
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
    let anchor_x = view
        .figure
        .fixed_message_anchor_x
        .unwrap_or_else(|| ((point.x - x) / width).clamp(0.0, 1.0));
    Some(bellatrix_mde_cordis::Point {
        x: anchor_x,
        y: ((point.y - y) / height).clamp(0.0, 1.0),
    })
}

pub(super) fn resolve_visual_center(geometry: &Geometry) -> Option<bellatrix_mde_cordis::Point> {
    use bellatrix_mde_cordis::Point as ModelPoint;
    match geometry {
        Geometry::Layout { .. } => None,
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some(ModelPoint {
            x: x + width / 2.0,
            y: y + height / 2.0,
        }),
        Geometry::Ellipse { cx, cy, .. } => Some(ModelPoint { x: *cx, y: *cy }),
        Geometry::Polygon { points } => {
            if points.is_empty() {
                return None;
            }
            Some(ModelPoint {
                x: points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64,
                y: points.iter().map(|point| point.y).sum::<f64>() / points.len() as f64,
            })
        }
        Geometry::Path { start, segments } => {
            let index = segments.len() / 2;
            let from = index
                .checked_sub(1)
                .and_then(|index| segments.get(index))
                .map(|segment| match segment {
                    PathSegment::Line { to } | PathSegment::Cubic { to, .. } => *to,
                })
                .unwrap_or(*start);
            Some(match segments.get(index) {
                Some(PathSegment::Line { to }) => ModelPoint {
                    x: (from.x + to.x) / 2.0,
                    y: (from.y + to.y) / 2.0,
                },
                Some(PathSegment::Cubic {
                    control1,
                    control2,
                    to,
                }) => ModelPoint {
                    x: (from.x + 3.0 * control1.x + 3.0 * control2.x + to.x) / 8.0,
                    y: (from.y + 3.0 * control1.y + 3.0 * control2.y + to.y) / 8.0,
                },
                None => from,
            })
        }
    }
}
