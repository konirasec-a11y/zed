use super::*;

impl Render for MdeWorkbenchItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let background = palette.canvas;
        let border = palette.border;
        let selection = palette.selection;
        let muted = palette.muted;
        let zoom = self.zoom;
        let pan = self.pan;
        let relation_hint = match self.selected_tool {
            Some(ModelingTool::Relation(kind)) => Some(if self.pending_relation_source.is_some() {
                format!("{}: click second element (target)", kind.label())
            } else {
                format!("{}: {}", kind.label(), kind.endpoint_instruction())
            }),
            _ => None,
        };
        let mut title = format!("MDE · {}", self.project_id().as_str());
        let details: String;
        let mut cards = Vec::new();
        match self.source.snapshot() {
            Ok(snapshot) => {
                let references = bellatrix_mde::uml::references::ReferenceIndex::new(
                    bellatrix_mde::uml::root::Model::from_section(&snapshot.section),
                );
                let diagram = self
                    .active_diagram
                    .as_deref()
                    .and_then(|id| {
                        snapshot
                            .section
                            .diagrams
                            .iter()
                            .find(|diagram| diagram.id == id)
                    })
                    .or_else(|| snapshot.section.diagrams.first());
                details = format!(
                    "{} elements · {} relations · rev {}",
                    snapshot.section.elements.len(),
                    snapshot.section.relations.len(),
                    snapshot.revision
                );
                if let Some(diagram) = diagram {
                    title = format!("{} / {}", snapshot.name, diagram.name);
                    let graph = RelationshipGraph::new(&snapshot.section, &diagram.id);
                    let mut views = diagram.views.iter().collect::<Vec<_>>();
                    views.sort_by_key(|view| view.z_index);
                    for view in views {
                        if view.subject.is_none() && view.draft.is_some() {
                            if let Some(card) =
                                self.render_visual_draft(view, diagram, snapshot.revision, cx)
                            {
                                cards.push(card);
                            }
                            continue;
                        }
                        let Some(subject) = view.subject.as_deref() else {
                            continue;
                        };
                        if let Some(relation) = snapshot
                            .section
                            .relations
                            .iter()
                            .find(|relation| relation.id == subject)
                        {
                            let routed_geometry = graph.view_path(&view.id);
                            // Keep an unroutable draft visible using its stored geometry.
                            let geometry = routed_geometry.as_ref().unwrap_or(&view.geometry);
                            if let Geometry::Path { start, segments } = geometry {
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
                                if points.len() < 2
                                    || points
                                        .iter()
                                        .any(|point| !point.x.is_finite() || !point.y.is_finite())
                                {
                                    continue;
                                }
                                let Some(end) = points.last().copied() else {
                                    continue;
                                };
                                let min_x = points
                                    .iter()
                                    .map(|point| point.x)
                                    .fold(f64::INFINITY, f64::min)
                                    as f32;
                                let min_y = points
                                    .iter()
                                    .map(|point| point.y)
                                    .fold(f64::INFINITY, f64::min)
                                    as f32;
                                let max_x = points
                                    .iter()
                                    .map(|point| point.x)
                                    .fold(f64::NEG_INFINITY, f64::max)
                                    as f32;
                                let max_y = points
                                    .iter()
                                    .map(|point| point.y)
                                    .fold(f64::NEG_INFINITY, f64::max)
                                    as f32;
                                let padding = 24.0_f32;
                                let width = (max_x - min_x + 2.0 * padding).max(48.0) * zoom;
                                let height = (max_y - min_y + 2.0 * padding).max(48.0) * zoom;
                                let place = |point: bellatrix_mde::Point| {
                                    (
                                        (point.x as f32 - min_x + padding) * zoom,
                                        (point.y as f32 - min_y + padding) * zoom,
                                    )
                                };
                                let presentation = relationship_presentation(relation);
                                let local_segments = segments
                                    .iter()
                                    .map(|segment| match segment {
                                        PathSegment::Line { to } => {
                                            UmlLineSegment::Line { to: place(*to) }
                                        }
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
                                let glyph = UmlRelationGlyph::new(
                                    presentation.style,
                                    width,
                                    height,
                                    place(*start),
                                    place(end),
                                )
                                .view_style(&view.style)
                                .segments(local_segments)
                                .labels(presentation)
                                .view_scale(zoom)
                                .selected(self.selected_element.as_deref() == Some(subject));
                                let edge_owner = cx.weak_entity();
                                let edge_id = relation.id.clone();
                                let edge_view_id = view.id.clone();
                                let menu_owner = cx.entity();
                                let menu_diagram = diagram.id.clone();
                                let menu_view = view.id.clone();
                                let menu_subject = relation.id.clone();
                                cards.push(
                                    div()
                                        .id(format!("mde-edge-{}", view.id))
                                        .absolute()
                                        .left(px((min_x - padding) * zoom + pan.0))
                                        .top(px((min_y - padding) * zoom + pan.1))
                                        .w(px(width))
                                        .h(px(height))
                                        .cursor_pointer()
                                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                            let _ = edge_owner.update(cx, |item, cx| {
                                                item.select_or_connect_edge(
                                                    edge_id.clone(),
                                                    edge_view_id.clone(),
                                                    cx,
                                                );
                                                cx.focus_self(window);
                                            });
                                            cx.stop_propagation();
                                        })
                                        .child(
                                            ui::right_click_menu(format!(
                                                "mde-edge-menu-{}",
                                                view.id
                                            ))
                                            .trigger(move |_, _, _| glyph)
                                            .menu(
                                                move |window, cx| {
                                                    Self::view_context_menu(
                                                        menu_owner.clone(),
                                                        menu_diagram.clone(),
                                                        menu_view.clone(),
                                                        menu_subject.clone(),
                                                        window,
                                                        cx,
                                                    )
                                                },
                                            ),
                                        )
                                        .into_any_element(),
                                );
                            }
                            continue;
                        }
                        let Some(element) = snapshot
                            .section
                            .elements
                            .iter()
                            .find(|element| element.id == subject)
                        else {
                            continue;
                        };
                        let Geometry::Rect {
                            x,
                            y,
                            width,
                            height,
                        } = &view.geometry
                        else {
                            continue;
                        };
                        let (x, y, width, height) = (*x, *y, *width, *height);
                        let revision = snapshot.revision;
                        if ![x, y, width, height].iter().all(|value| value.is_finite()) {
                            continue;
                        }
                        let mut local_x = x as f32;
                        let mut local_y = y as f32;
                        let mut local_width = width as f32;
                        let mut local_height = height as f32;
                        match &self.gesture {
                            Some(CanvasGesture::Move {
                                view_id, dx, dy, ..
                            }) if view_id == &view.id => {
                                local_x += *dx as f32;
                                local_y += *dy as f32;
                            }
                            Some(CanvasGesture::Resize {
                                view_id, dx, dy, ..
                            }) if view_id == &view.id => {
                                let (minimum_width, minimum_height) =
                                    bellatrix_mde::resolve_element_notation(element).minimum_size();
                                local_width = (width + dx).max(minimum_width) as f32;
                                local_height = (height + dy).max(minimum_height) as f32;
                            }
                            _ => {}
                        }
                        let notation = bellatrix_mde::uml_notation(&element.kind);
                        if matches!(
                            notation,
                            bellatrix_mde::UmlNotation::ClassifierRectangle
                                | bellatrix_mde::UmlNotation::PackageTab
                        ) {
                            let applied = element
                                .properties
                                .get("stereotype")
                                .and_then(serde_json::Value::as_str);
                            local_height = local_height.max(
                                bellatrix_mde::uml_header(&element.kind, applied)
                                    .minimum_height(notation)
                                    as f32,
                            );
                        }
                        let semantic_type = UmlType::from_kind(&element.kind);
                        let is_interface = semantic_type == Some(UmlType::Interface);
                        let contents = classifier_contents(&snapshot.section, &element.id);
                        if is_interface {
                            local_height =
                                local_height.max(contents.minimum_interface_height() as f32);
                        }
                        let template_parameters =
                            bellatrix_mde::template_parameters(&snapshot.section, &element.id);
                        let signature_layout = matches!(
                            notation,
                            bellatrix_mde::UmlNotation::ClassifierRectangle
                                | bellatrix_mde::UmlNotation::PackageTab
                        )
                        .then(|| {
                            bellatrix_mde::template_signature_layout(
                                f64::from(local_width),
                                template_parameters.len(),
                            )
                        })
                        .flatten();
                        let (signature_top, signature_right, signature_bottom) = signature_layout
                            .map(|layout| {
                                layout.outsets(f64::from(local_width), f64::from(local_height))
                            })
                            .unwrap_or_default();
                        let signature_top = signature_top as f32 * zoom;
                        let signature_right = signature_right as f32 * zoom;
                        let signature_bottom = signature_bottom as f32 * zoom;
                        let selected_target = self.selected_element.as_deref().and_then(|id| {
                            if id == subject {
                                Some(element)
                            } else if template_parameters
                                .iter()
                                .any(|parameter| parameter.id == id)
                            {
                                template_parameters
                                    .iter()
                                    .find(|parameter| parameter.id == id)
                                    .copied()
                            } else if snapshot.section.relations.iter().any(|relation| {
                                relation.kind == "owns"
                                    && relation.from == element.id
                                    && relation.to == id
                            }) {
                                snapshot
                                    .section
                                    .elements
                                    .iter()
                                    .find(|element| element.id == id)
                            } else {
                                None
                            }
                        });
                        let is_selected = selected_target.is_some();
                        let editing = selected_target.filter(|selected| {
                            self.editing_subject.as_deref() == Some(selected.id.as_str())
                                && self.inline_editor.is_some()
                        });
                        let (quick_actions, left_actions, visibility_picker) =
                            if let Some(selected) = editing {
                                quick_edit_for_element(
                                    selected,
                                    cx.entity(),
                                    self.project_id().clone(),
                                    snapshot.revision,
                                )
                            } else {
                                (None, None, None)
                            };
                        let title_id = element.id.clone();
                        let title_owner = cx.weak_entity();
                        let select_owner = cx.weak_entity();
                        let edit_owner = cx.weak_entity();
                        // Spatial containment is presentation only, not semantic ownership validation.
                        let has_nested_views = diagram.views.iter().any(|other| {
                            other.id != view.id && matches!(other.geometry, Geometry::Rect { x: child_x, y: child_y, width: child_width, height: child_height }
                                if child_x >= x && child_y >= y + 20.0 && child_x + child_width <= x + width && child_y + child_height <= y + height)
                        });
                        let mut classifier = UmlClassifier::new(element.name.clone())
                            .show_internal_structure(
                                has_nested_views
                                    && bellatrix_mde::uml::root::ElementRef::Object(element)
                                        .conforms_to("StructuredClassifier"),
                            )
                            .presentation(bellatrix_mde::classifier_presentation(
                                element,
                                bellatrix_mde::ClassifierPresentationOptions {
                                    // The editor chooses the optional active-Class notation.
                                    show_active_class_sides: true,
                                },
                            ))
                            .view_style(&view.style)
                            .model_kind(&element.kind)
                            .view_size(f64::from(local_width), f64::from(local_height), zoom)
                            .class_id(element.id.clone())
                            .selected(is_selected)
                            .on_title_edit(move |window, cx| {
                                let id = title_id.clone();
                                let _ = title_owner
                                    .update(cx, |item, cx| item.begin_inline_edit(id, window, cx));
                            })
                            .on_member_click(move |member_id, window, cx| {
                                let id = member_id.to_owned();
                                let _ = select_owner.update(cx, |item, cx| {
                                    item.select_element(id, cx);
                                    cx.focus_self(window);
                                });
                            })
                            .on_member_edit(move |member_id, window, cx| {
                                let id = member_id.to_owned();
                                let _ = edit_owner
                                    .update(cx, |item, cx| item.begin_inline_edit(id, window, cx));
                            });
                        if let Some(value) = element
                            .properties
                            .get("stereotype")
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| !value.trim().is_empty())
                        {
                            classifier = classifier.stereotype(value.to_owned());
                        }
                        for relation in &snapshot.section.relations {
                            if relation.kind != "owns" || relation.from != element.id {
                                continue;
                            }
                            let Some(member) = snapshot
                                .section
                                .elements
                                .iter()
                                .find(|candidate| candidate.id == relation.to)
                            else {
                                continue;
                            };
                            let text = bellatrix_mde::format_element_label_for_view(
                                member,
                                &references,
                                &view.style,
                            );
                            match member.kind.as_str() {
                                "rust.field" | "uml.attribute" => {
                                    classifier =
                                        classifier.attribute_with_id(member.id.clone(), text)
                                }
                                "rust.function" | "uml.operation" => {
                                    classifier =
                                        classifier.operation_with_id(member.id.clone(), text)
                                }
                                "uml.reception" => {
                                    classifier =
                                        classifier.reception_with_id(member.id.clone(), text)
                                }
                                _ => {}
                            }
                            classifier = classifier.member_presentation(
                                &member.id,
                                bellatrix_mde::member_presentation(member),
                            );
                        }
                        for parameter in template_parameters {
                            classifier = classifier.template_parameter_with_id(
                                parameter.id.clone(),
                                bellatrix_mde::template_parameter_label_with_references(
                                    &snapshot.section,
                                    parameter,
                                    &references,
                                ),
                            );
                        }
                        if editing.is_some() {
                            if let (Some(id), Some(field)) =
                                (self.editing_subject.clone(), self.inline_editor.clone())
                            {
                                let accept = cx.weak_entity();
                                let cancel = cx.weak_entity();
                                classifier = classifier.inline_edit(
                                    id,
                                    field,
                                    move |window, cx| {
                                        let _ = accept.update(cx, |item, cx| {
                                            item.commit_inline_edit(window, cx)
                                        });
                                    },
                                    move |window, cx| {
                                        let _ = cancel.update(cx, |item, cx| {
                                            item.cancel_inline_edit(window, cx)
                                        });
                                    },
                                );
                            }
                        }
                        let figure: gpui::AnyElement = if UmlFigure::supports(&element.kind) {
                            let edit_owner = cx.weak_entity();
                            let edit_id = element.id.clone();
                            let label = super::visual_draft_canvas::format_figure_label(
                                element,
                                &references,
                                &view.style,
                            );
                            let operator_default =
                                super::visual_draft_canvas::resolve_interaction_operator_default();
                            let mut figure = UmlFigure::new(element.kind.clone(), label)
                                .diagram_presentation(diagram)
                                .nested_views(has_nested_views)
                                .composite_role(
                                    element
                                        .properties
                                        .get("aggregation")
                                        .and_then(serde_json::Value::as_str)
                                        == Some("composite"),
                                )
                                .interaction_operator(
                                    element
                                        .properties
                                        .get("interactionOperator")
                                        .or(operator_default.as_ref())
                                        .and_then(serde_json::Value::as_str),
                                )
                                .slot_labels(bellatrix_mde::collect_instance_slot_labels(
                                    element,
                                    &references,
                                ))
                                .with_template_parameters_from(&classifier)
                                .view_style(&view.style)
                                .selected(is_selected)
                                .view_size(f64::from(local_width), f64::from(local_height), zoom)
                                .on_title_edit(move |window, cx| {
                                    let id = edit_id.clone();
                                    let _ = edit_owner.update(cx, |item, cx| {
                                        item.begin_inline_edit(id, window, cx)
                                    });
                                });
                            if let Some(kind) = element
                                .properties
                                .get("kind")
                                .and_then(serde_json::Value::as_str)
                            {
                                figure = figure.pseudostate_kind(kind);
                            }
                            if let Some(value) = element
                                .properties
                                .get("stereotype")
                                .and_then(serde_json::Value::as_str)
                            {
                                figure = figure.stereotype(value.to_owned());
                            }
                            if editing.is_some_and(|target| target.id == element.id) {
                                if let Some(field) = self.inline_editor.clone() {
                                    let confirm = cx.weak_entity();
                                    let cancel = cx.weak_entity();
                                    figure = figure.inline_edit(
                                        field,
                                        move |window, cx| {
                                            let _ = confirm.update(cx, |item, cx| {
                                                item.commit_inline_edit(window, cx)
                                            });
                                        },
                                        move |window, cx| {
                                            let _ = cancel.update(cx, |item, cx| {
                                                item.cancel_inline_edit(window, cx)
                                            });
                                        },
                                    );
                                }
                            }
                            figure.into_any_element()
                        } else if matches!(
                            notation,
                            bellatrix_mde::UmlNotation::ClassifierRectangle
                        ) {
                            classifier.into_any_element()
                        } else {
                            div()
                                .size_full()
                                .border_1()
                                .border_color(border)
                                .child(Label::new(format!(
                                    "Unsupported notation: {}",
                                    element.kind
                                )))
                                .into_any_element()
                        };
                        let menu_owner = cx.entity();
                        let menu_diagram = diagram.id.clone();
                        let menu_view = view.id.clone();
                        let menu_subject = subject.to_owned();
                        let drag_owner = cx.weak_entity();
                        let drag_id = view.id.clone();
                        let connection_mode =
                            matches!(self.selected_tool, Some(ModelingTool::Relation(_)));
                        let connect_owner = cx.weak_entity();
                        let connect_target = subject.to_owned();
                        let connect_view = view.id.clone();
                        let resize_owner = cx.weak_entity();
                        let resize_id = view.id.clone();
                        let element_id = subject.to_owned();
                        cards.push(
                            div()
                                .id(format!("mde-view-{}", view.id))
                                .absolute()
                                .left(px(local_x * zoom + pan.0))
                                .top(px(local_y * zoom + pan.1 - signature_top))
                                .w(px(local_width * zoom + signature_right))
                                .h(px(local_height * zoom + signature_top + signature_bottom))
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    move |event: &MouseDownEvent, window, cx| {
                                        let _ = drag_owner.update(cx, |item, cx| {
                                            item.start_move(
                                                drag_id.clone(),
                                                element_id.clone(),
                                                revision,
                                                event.position,
                                                cx,
                                            );
                                            cx.focus_self(window);
                                        });
                                        cx.stop_propagation();
                                    },
                                )
                                .child(
                                    ui::right_click_menu(format!("mde-view-menu-{}", view.id))
                                        .trigger(move |_, _, _| {
                                            // The menu's auto-height layout cannot resolve the
                                            // figure's percentage height from the outer card.
                                            div()
                                                .relative()
                                                .w(px(local_width * zoom + signature_right))
                                                .h(px(local_height * zoom
                                                    + signature_top
                                                    + signature_bottom))
                                                .child(
                                                    div()
                                                        .absolute()
                                                        .left_0()
                                                        .top(px(signature_top))
                                                        .w(px(local_width * zoom))
                                                        .h(px(local_height * zoom))
                                                        .child(figure),
                                                )
                                        })
                                        .menu(move |window, cx| {
                                            Self::view_context_menu(
                                                menu_owner.clone(),
                                                menu_diagram.clone(),
                                                menu_view.clone(),
                                                menu_subject.clone(),
                                                window,
                                                cx,
                                            )
                                        }),
                                )
                                .when_some(quick_actions, |card, actions| {
                                    card.child(
                                        div()
                                            .id(format!("mde-quick-actions-{}", view.id))
                                            .absolute()
                                            .left(px(local_width * zoom + signature_right + 6.0))
                                            .top(px(signature_top))
                                            .child(actions),
                                    )
                                })
                                .when_some(left_actions, |card, actions| {
                                    card.child(
                                        div()
                                            .id(format!("mde-quick-left-actions-{}", view.id))
                                            .absolute()
                                            .left(px(-78.0))
                                            .top(px(signature_top + 28.0))
                                            .child(actions),
                                    )
                                })
                                .when_some(visibility_picker, |card, picker| {
                                    card.child(
                                        div()
                                            .id(format!("mde-quick-visibility-{}", view.id))
                                            .absolute()
                                            .left(px(-34.0))
                                            .top(px(signature_top))
                                            .child(picker),
                                    )
                                })
                                .when(
                                    is_selected
                                        && !connection_mode
                                        && semantic_type != Some(UmlType::Port),
                                    |card| {
                                        card.child(
                                            div()
                                                .id(format!("mde-resize-{}", view.id))
                                                .absolute()
                                                .bottom(px(signature_bottom))
                                                .right(px(signature_right))
                                                .w(px(10.0))
                                                .h(px(10.0))
                                                .bg(selection)
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    move |event: &MouseDownEvent, _, cx| {
                                                        let _ =
                                                            resize_owner.update(cx, |item, cx| {
                                                                item.start_resize(
                                                                    resize_id.clone(),
                                                                    width,
                                                                    height,
                                                                    revision,
                                                                    event.position,
                                                                    cx,
                                                                )
                                                            });
                                                        cx.stop_propagation();
                                                    },
                                                ),
                                        )
                                    },
                                )
                                .when(connection_mode, |card| {
                                    card.child(
                                        div()
                                            .id(format!("mde-connection-target-{}", view.id))
                                            .absolute()
                                            .top(px(signature_top))
                                            .right(px(signature_right))
                                            .bottom(px(signature_bottom))
                                            .left_0()
                                            .cursor_crosshair()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                move |_, window, cx| {
                                                    let _ = connect_owner.update(cx, |item, cx| {
                                                        if let Some(ModelingTool::Relation(kind)) =
                                                            item.selected_tool
                                                        {
                                                            if item.should_connect_visual(kind) {
                                                                item.link_visual_draft_endpoint(
                                                                    kind,
                                                                    connect_view.clone(),
                                                                    cx,
                                                                );
                                                            } else {
                                                                item.link_endpoint(
                                                                    kind,
                                                                    connect_target.clone(),
                                                                    cx,
                                                                );
                                                            }
                                                            cx.focus_self(window);
                                                        }
                                                    });
                                                    cx.stop_propagation();
                                                },
                                            ),
                                    )
                                })
                                .into_any_element(),
                        );
                    }
                }
            }
            Err(error) => details = error,
        }
        let owner = cx.weak_entity();
        let move_owner = cx.weak_entity();
        let up_owner = cx.weak_entity();
        let out_owner = cx.weak_entity();
        let dragging_owner = cx.weak_entity();
        let dropping_owner = cx.weak_entity();
        let wheel_owner = cx.weak_entity();
        let pan_start_owner = cx.weak_entity();
        let pan_end_owner = cx.weak_entity();
        let pan_out_owner = cx.weak_entity();
        let f2_owner = cx.weak_entity();
        let member_shortcut_owner = cx.weak_entity();
        let zoom_in_owner = cx.weak_entity();
        let zoom_out_owner = cx.weak_entity();
        let reset_owner = cx.weak_entity();
        let validate_owner = cx.weak_entity();
        let cancel_relation_owner = cx.weak_entity();
        let escape_relation_owner = cx.weak_entity();
        let canvas_origin = std::rc::Rc::new(std::cell::Cell::new(None::<(f64, f64)>));
        let measured_canvas_origin = canvas_origin.clone();
        div()
            .id(format!("mde-workbench-{}", self.project_id().as_str()))
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .key_context("BellatrixMde")
            .on_action(
                cx.listener(|item, _: &integration::RemoveFromDiagram, window, cx| {
                    if canvas_delete_action_allowed(
                        item.focus_handle.is_focused(window),
                        item.editing_subject.is_some(),
                    ) {
                        item.remove_selected_view(cx);
                    } else {
                        cx.propagate();
                    }
                }),
            )
            .on_action(
                cx.listener(|item, _: &integration::DeleteFromModel, window, cx| {
                    if canvas_delete_action_allowed(
                        item.focus_handle.is_focused(window),
                        item.editing_subject.is_some(),
                    ) {
                        item.delete_selected_from_model(cx);
                    } else {
                        cx.propagate();
                    }
                }),
            )
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("f2") {
                    let _ = f2_owner.update(cx, |item, cx| {
                        if let Some(id) = item.selected_element.clone() {
                            item.begin_inline_edit(id, window, cx);
                        } else if let Some(id) = item.selected_view.clone() {
                            item.begin_draft_label_edit(id, window, cx);
                        }
                    });
                    cx.stop_propagation();
                } else if event.keystroke.key.eq_ignore_ascii_case("enter")
                    && event.keystroke.modifiers.control
                {
                    let kind = if event.keystroke.modifiers.shift {
                        ModelingMemberKind::Operation
                    } else {
                        ModelingMemberKind::Attribute
                    };
                    let _ = member_shortcut_owner
                        .update(cx, |item, cx| item.add_member_to_selected(kind, cx));
                    cx.stop_propagation();
                } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    let _ = escape_relation_owner.update(cx, |item, cx| {
                        if item.editing_subject.is_none() && item.selected_tool.is_some() {
                            item.cancel_tool(cx);
                        }
                    });
                }
            })
            .bg(background)
            .text_color(palette.text)
            .child(
                div()
                    .h(px(39.0))
                    .flex_none()
                    .flex()
                    .gap_3()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(border)
                    .child(div().flex_1().min_w_0().child(Label::new(title).truncate()))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .gap_2()
                            .items_center()
                            .child(
                                ui::Button::new("mde-validate-diagram", "Validate").on_click(
                                    move |_, window, cx| {
                                        if let Err(error) = validate_owner
                                            .update(cx, |item, cx| item.validate_diagram(cx))
                                        {
                                            eprintln!("MDE diagram validation failed: {error}");
                                        }
                                        window.dispatch_action(
                                            Box::new(panels::ShowMdeDiagnostics),
                                            cx,
                                        );
                                    },
                                ),
                            )
                            .child(self.render_validation_mode(cx))
                            .when_some(relation_hint, |header, hint| {
                                header.child(
                                    div()
                                        .flex()
                                        .gap_1()
                                        .items_center()
                                        .child(Label::new(hint))
                                        .child(
                                            ui::Button::new("mde-cancel-relation", "Cancel")
                                                .on_click(move |_, _, cx| {
                                                    let _ = cancel_relation_owner
                                                        .update(cx, |item, cx| {
                                                            item.cancel_tool(cx)
                                                        });
                                                }),
                                        ),
                                )
                            })
                            .child(ui::Button::new("mde-zoom-out", "-").on_click(
                                move |_, _, cx| {
                                    let _ = zoom_out_owner
                                        .update(cx, |item, cx| item.set_zoom(item.zoom / 1.2, cx));
                                },
                            ))
                            .child(Label::new(format!("{:.0}%", zoom * 100.0)))
                            .child(
                                ui::Button::new("mde-zoom-in", "+").on_click(move |_, _, cx| {
                                    let _ = zoom_in_owner
                                        .update(cx, |item, cx| item.set_zoom(item.zoom * 1.2, cx));
                                }),
                            )
                            .child(ui::Button::new("mde-reset-view", "Reset view").on_click(
                                move |_, _, cx| {
                                    let _ =
                                        reset_owner.update(cx, |item, cx| item.reset_viewport(cx));
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .id("mde-canvas")
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .on_scroll(move |event: &ScrollEvent, _, cx| {
                        let _ = wheel_owner.update(cx, |item, cx| item.scroll_viewport(event, cx));
                        cx.stop_propagation();
                    })
                    .on_mouse_down(MouseButton::Middle, move |event: &MouseDownEvent, _, cx| {
                        let _ =
                            pan_start_owner.update(cx, |item, _| item.start_pan(event.position));
                        cx.stop_propagation();
                    })
                    .on_mouse_up(MouseButton::Middle, move |_: &MouseUpEvent, _, cx| {
                        let _ = pan_end_owner.update(cx, |item, _| item.pan_origin = None);
                    })
                    .on_mouse_up_out(MouseButton::Middle, move |_: &MouseUpEvent, _, cx| {
                        let _ = pan_out_owner.update(cx, |item, _| item.pan_origin = None);
                    })
                    .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                        let Some(origin) = canvas_origin.get() else {
                            return;
                        };
                        if let Err(error) = owner.update(cx, |item, cx| {
                            if let Some(ModelingTool::Element(kind)) = item.selected_tool {
                                let position =
                                    (f64::from(event.position.x), f64::from(event.position.y));
                                let (x, y) =
                                    canvas_model_position(position, origin, item.pan, item.zoom);
                                item.create_element(kind, x, y, cx);
                            }
                        }) {
                            eprintln!("MDE canvas placement failed: {error}");
                        }
                    })
                    .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                        let _ = move_owner.update(cx, |item, cx| {
                            item.update_pan(event, cx);
                            item.update_gesture(event, cx);
                        });
                    })
                    .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                        let _ = up_owner.update(cx, |item, cx| item.finish_gesture(cx));
                    })
                    .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                        let _ = out_owner.update(cx, |item, cx| item.finish_gesture(cx));
                    })
                    .on_drag_move(move |event: &DragMoveEvent<MdeToolDrag>, _, cx| {
                        let position = (
                            f64::from(event.event.position.x),
                            f64::from(event.event.position.y),
                        );
                        let origin = (
                            f64::from(event.bounds.origin.x),
                            f64::from(event.bounds.origin.y),
                        );
                        if let Err(error) = dragging_owner.update(cx, |item, _| {
                            if item.source.is_read_only() {
                                return;
                            }
                            item.drop_point =
                                Some(canvas_model_position(position, origin, item.pan, item.zoom));
                        }) {
                            eprintln!("MDE canvas drag failed: {error}");
                        }
                    })
                    .on_drop(move |drag: &MdeToolDrag, _, cx| {
                        let _ = dropping_owner.update(cx, |item, cx| {
                            if &drag.project_id != item.project_id() {
                                return;
                            }
                            let (x, y) = item.drop_point.take().unwrap_or((40.0, 40.0));
                            match drag.kind {
                                ModelingTool::Element(kind) => item.create_element(kind, x, y, cx),
                                ModelingTool::Relation(kind) => {
                                    item.arm_drag_tool(ModelingTool::Relation(kind), cx)
                                }
                            }
                        });
                    })
                    .child(
                        gpui::canvas(
                            move |bounds, _, _| {
                                measured_canvas_origin.set(Some((
                                    f64::from(bounds.origin.x),
                                    f64::from(bounds.origin.y),
                                )));
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .children(cards),
            )
            .child(
                div()
                    .h(px(25.0))
                    .flex_none()
                    .px_2()
                    .flex()
                    .gap_3()
                    .items_center()
                    .border_t_1()
                    .border_color(border)
                    .text_color(muted)
                    .child(
                        div().flex_1().min_w_0().child(
                            Label::new(self.feedback.lines().next().unwrap_or_default().to_owned())
                                .truncate(),
                        ),
                    )
                    .child(Label::new(details).truncate()),
            )
    }
}
