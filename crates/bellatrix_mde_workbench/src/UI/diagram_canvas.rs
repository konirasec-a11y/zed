use super::*;
use bellatrix_mde_cordis::presentation_contract::UmlNotation;

impl Render for MdeWorkbenchItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.source.snapshot();
        if let Ok(snapshot) = &snapshot {
            self.reconcile_selection(&snapshot.section);
        }
        let palette = WorkbenchPalette::current(cx);
        let background = palette.canvas;
        let border = palette.border;
        let selection = palette.selection;
        let muted = palette.muted;
        let zoom = self.zoom;
        let pan = self.pan;
        let canvas_origin = std::rc::Rc::new(std::cell::Cell::new(None::<(f64, f64)>));
        let relation_hint = match self.presentation.selected_tool() {
            Some(ModelingTool::Relation(kind)) => snapshot
                .as_ref()
                .ok()
                .and_then(|snapshot| snapshot.section.tools.relation(kind))
                .map(|metadata| {
                    if self.presentation.pending_relation_source().is_some() {
                        format!("{}: click second element (target)", metadata.label)
                    } else {
                        format!("{}: {}", metadata.label, metadata.endpoint_instruction)
                    }
                }),
            _ => None,
        };
        let mut title = format!("MDE · {}", self.project_id().as_str());
        let details: String;
        let mut cards = Vec::new();
        let node_bounds = std::rc::Rc::new(std::cell::RefCell::new(Vec::<[f32; 4]>::new()));
        let mut relationship_controls = Vec::new();
        let mut path_handles = Vec::new();
        let mut unresolved_presentation = false;
        let mut marker_diagnostics = Vec::new();
        match snapshot {
            Ok(snapshot) => {
                let diagram = self.presentation.active_diagram().and_then(|id| {
                    snapshot
                        .section
                        .diagrams
                        .iter()
                        .find(|diagram| diagram.id == id)
                });
                details = format!(
                    "{} elements · {} relations · rev {}",
                    snapshot.section.elements.len(),
                    snapshot.section.relations.len(),
                    snapshot.revision
                );
                if let Some(diagram) = diagram {
                    title = format!("{} / {}", snapshot.name, diagram.name);
                    unresolved_presentation = diagram.all_views().any(|view| matches!(&view.geometry, Geometry::Layout { .. }));
                    let views = canvas_presentation::ordered_views(&diagram.views);
                    for view in views {
                        if matches!(view.geometry, Geometry::Rect { .. } | Geometry::Layout { .. })
                            && canvas_presentation::resolve_bounds(&view.geometry, zoom, pan).is_none()
                        {
                            unresolved_presentation = true;
                            marker_diagnostics.push(format!("View {} has unresolved or non-renderable bounds", view.id));
                            continue;
                        }
                        if let Some(entity) = &view.entity_box {
                            if !entity.diagnostics.is_empty() {
                                unresolved_presentation = true;
                                marker_diagnostics.extend(entity.diagnostics.iter().cloned());
                            }
                        }
                        if let Some(icon) = &view.interface_icon {
                            if !icon.diagnostics.is_empty() {
                                unresolved_presentation = true;
                                marker_diagnostics.extend(icon.diagnostics.iter().cloned());
                            }
                        }
                        if !view.rendered_label.diagnostics.is_empty() {
                            unresolved_presentation = true;
                            marker_diagnostics
                                .extend(view.rendered_label.diagnostics.iter().cloned());
                        }
                        if view.figure.notation == UmlNotation::Unsupported && !matches!(view.geometry, Geometry::Path { .. }) {
                            unresolved_presentation = true;
                            marker_diagnostics.push(format!("View {} has an unsupported native figure; its data is preserved", view.id));
                        }
                        if matches!(view.figure.notation, UmlNotation::Label | UmlNotation::Compartment | UmlNotation::Rectangle)
                            || view.subject.is_none() && view.draft.is_none() && matches!(view.geometry, Geometry::Rect { .. })
                        {
                            if let Some(card) = self.render_presentation_view(&snapshot.section, diagram, view, snapshot.revision, &mut node_bounds.borrow_mut(), cx) {
                                cards.push(card);
                            } else {
                                unresolved_presentation = true;
                                marker_diagnostics.push(format!("View {} has unresolved native bounds", view.id));
                            }
                            continue;
                        }
                        path_handles.extend(self.render_path_handles(
                            &diagram.id,
                            view,
                            snapshot.revision,
                            cx,
                        ));
                        if view.subject.is_none() && view.draft.is_some() {
                            if let Some(card) = self.render_visual_draft(
                                view,
                                diagram,
                                snapshot.revision,
                                &canvas_origin,
                                &node_bounds,
                                cx,
                            ) {
                                cards.push(card);
                            }
                            continue;
                        }
                        let Some(subject) = view.subject.as_deref() else {
                            continue;
                        };
                        let relation = snapshot
                            .section
                            .relations
                            .iter()
                            .find(|relation| relation.id == subject);
                        if let Some(resolved) = &view.relationship {
                            if !resolved.diagnostics.is_empty() {
                                unresolved_presentation = true;
                                marker_diagnostics.extend(resolved.diagnostics.iter().cloned());
                            }
                            // Keep an unroutable draft visible using its stored geometry.
                            let geometry = self.path_preview_geometry(view);
                            if let Geometry::Path { start, segments } = &geometry {
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
                                let padding = 24.0_f32.max(5.0 / zoom);
                                let width = (max_x - min_x + 2.0 * padding).max(48.0) * zoom;
                                let height = (max_y - min_y + 2.0 * padding).max(48.0) * zoom;
                                let place = |point: bellatrix_mde_cordis::Point| {
                                    (
                                        (point.x as f32 - min_x + padding) * zoom,
                                        (point.y as f32 - min_y + padding) * zoom,
                                    )
                                };
                                // Imported coordinates may be finite f64 values outside GPUI's f32 range.
                                if ![
                                    min_x,
                                    min_y,
                                    max_x,
                                    max_y,
                                    width,
                                    height,
                                    (min_x - padding) * zoom + pan.0,
                                    (min_y - padding) * zoom + pan.1,
                                ]
                                .iter()
                                .all(|value| value.is_finite())
                                    || points.iter().any(|point| {
                                        let (x, y) = place(*point);
                                        !x.is_finite() || !y.is_finite()
                                    })
                                {
                                    continue;
                                }
                                if self.presentation.selected_element() == Some(subject)
                                    && self.presentation.selected_canvas_view().map_or_else(
                                        || {
                                            diagram
                                                .views
                                                .iter()
                                                .find(|candidate| {
                                                    candidate.subject.as_deref() == Some(subject)
                                                })
                                                .is_some_and(|candidate| candidate.id == view.id)
                                        },
                                        |selected| selected == view.id,
                                    )
                                    && self.presentation.selected_tool().is_none()
                                    && let Some(relation) = relation
                                {
                                    if let Some(controls) =
                                        quick_edit_view::quick_edit_for_relationship(
                                            &snapshot.section,
                                            relation,
                                            cx.entity(),
                                            self.project_id().clone(),
                                            snapshot.revision,
                                            self.source.is_read_only(),
                                            window,
                                            cx,
                                        )
                                    {
                                        relationship_controls.push(
                                            // Anchor to the canvas viewport, not the path bounds:
                                            // pan/zoom must not put editing controls outside its clip.
                                            div()
                                                .id("mde-relationship-quick-edit-viewport")
                                                .absolute()
                                                .left_0()
                                                .top_0()
                                                .w_full()
                                                .max_w(px(320.0))
                                                .max_h_full()
                                                .overflow_y_scroll()
                                                .on_scroll(|_, _, cx| cx.stop_propagation())
                                                .child(controls)
                                                .into_any_element(),
                                        );
                                    }
                                }
                                let presentation = resolved.presentation.clone();
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
                                let render_style = canvas_presentation::render_style(
                                    view,
                                    bellatrix_design_system::Palette::current(cx),
                                );
                                let glyph = UmlRelationGlyph::new(
                                    resolved.style,
                                    width,
                                    height,
                                    place(*start),
                                    place(end),
                                )
                                .view_style(&render_style)
                                .segments(local_segments)
                                .communication_message(resolved.communication_message)
                                .labels(presentation)
                                .view_scale(zoom)
                                .selected(
                                    self.presentation.selected_element() == Some(subject)
                                        && self
                                            .presentation
                                            .selected_canvas_view()
                                            .is_none_or(|selected| selected == view.id),
                                );
                                let glyph = if let Some((source, target)) = &resolved.markers {
                                    glyph.endpoint_markers(source.clone(), target.clone())
                                } else {
                                    glyph
                                };
                                let edge_owner = cx.weak_entity();
                                let edge_id = subject.to_owned();
                                let edge_view_id = view.id.clone();
                                let edge_diagram_id = diagram.id.clone();
                                let edge_revision = snapshot.revision;
                                let menu_owner = cx.entity();
                                let menu_diagram = diagram.id.clone();
                                let menu_view = view.id.clone();
                                let menu_subject = subject.to_owned();
                                let hit_geometry = geometry.clone();
                                let hit_origin = canvas_origin.clone();
                                let menu_geometry = hit_geometry.clone();
                                let menu_origin = hit_origin.clone();
                                let hit_nodes = node_bounds.clone();
                                let menu_nodes = node_bounds.clone();
                                cards.push(
                                    div()
                                        .id(format!("mde-edge-{}", view.id))
                                        .absolute()
                                        .left(px((min_x - padding) * zoom + pan.0))
                                        .top(px((min_y - padding) * zoom + pan.1))
                                        .w(px(width))
                                        .h(px(height))
                                        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                                            if !relationship_hit_test(&hit_geometry, event.position, hit_origin.get(), pan, zoom, &hit_nodes.borrow()) {
                                                return;
                                            }
                                            if let Err(error) = edge_owner.update(cx, |item, cx| {
                                                let selected = item.presentation.selected_canvas_view() == Some(edge_view_id.as_str());
                                                item.source.record_interaction("canvas.relationship.click", Some(&edge_view_id), bellatrix_mde_cordis::TraceOutcome::Success);
                                                item.select_or_connect_edge(
                                                    edge_id.clone(),
                                                    edge_view_id.clone(),
                                                    cx,
                                                );
                                                if selected && let Some(origin) = hit_origin.get() {
                                                    let (x, y) = canvas_model_position(
                                                        (f64::from(event.position.x), f64::from(event.position.y)),
                                                        origin, pan, zoom,
                                                    );
                                                    if let Some(handle) = path_segment_at_position(&hit_geometry, bellatrix_mde_cordis::Point { x, y }, zoom) {
                                                        item.start_path_edit(edge_diagram_id.clone(), edge_view_id.clone(), edge_revision, handle, event.position, cx);
                                                    }
                                                }
                                                cx.focus_self(window);
                                            }) {
                                                eprintln!("MDE relationship selection failed: {error}");
                                            }
                                            cx.stop_propagation();
                                        })
                                        .child(
                                            ui::right_click_menu(format!(
                                                "mde-edge-menu-{}",
                                                view.id
                                            ))
                                            .trigger(move |_, _, _| glyph)
                                            .trigger_filter(move |position, _, _| {
                                                relationship_hit_test(&menu_geometry, position, menu_origin.get(), pan, zoom, &menu_nodes.borrow())
                                            })
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
                        if matches!(view.geometry, Geometry::Path { .. }) {
                            unresolved_presentation = true;
                            marker_diagnostics.push(format!("View {} has no supported relationship presentation; its path is preserved", view.id));
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
                        let Some((preview_x, preview_y, preview_width, preview_height)) =
                            self.preview_rectangle(&snapshot.section, diagram, view)
                        else { continue; };
                        let local_x = preview_x as f32;
                        let local_y = preview_y as f32;
                        let local_width = preview_width as f32;
                        let mut local_height = preview_height as f32;
                        let notation = view.figure.notation;
                        let authored_contents = view.children.iter().any(|child| child.presentation.internal);
                        if !authored_contents && matches!(
                            notation,
                            UmlNotation::ClassifierRectangle | UmlNotation::PackageTab
                        ) {
                            local_height = local_height.max(element.minimum_header_height as f32);
                        }
                        if !authored_contents && let Some(minimum_height) = element.minimum_interface_height {
                            local_height = local_height.max(minimum_height as f32);
                        }
                        let template_parameters = element
                            .template_parameters
                            .iter()
                            .filter_map(|identifier| {
                                snapshot
                                    .section
                                    .elements
                                    .iter()
                                    .find(|candidate| candidate.id == *identifier)
                            })
                            .collect::<Vec<_>>();
                        let signature_layout = matches!(
                            notation,
                            UmlNotation::ClassifierRectangle | UmlNotation::PackageTab
                        )
                        .then(|| {
                            bellatrix_ui_uml::template_signature_layout(
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
                        let selected_target = self
                            .presentation
                            .selected_element()
                            .and_then(|id| {
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
                                } else if view.member_labels.contains_key(id) {
                                    snapshot
                                        .section
                                        .elements
                                        .iter()
                                        .find(|element| element.id == id)
                                } else {
                                    None
                                }
                            })
                            .filter(|_| {
                                self.presentation
                                    .selected_canvas_view()
                                    .is_none_or(|selected| selected == view.id)
                            });
                        let is_selected = selected_target.is_some();
                        let editing = selected_target.filter(|selected| {
                            self.presentation.editing_element() == Some(selected.id.as_str())
                                && self.inline_editor.is_some()
                        });
                        let (quick_actions, left_actions, visibility_picker) =
                            if let Some(selected) = selected_target.filter(|_| {
                                !self.source.is_read_only()
                                    && self.presentation.selected_tool().is_none()
                            }) {
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
                        let title_diagram = diagram.id.clone();
                        let title_view = view.id.clone();
                        let title_owner = cx.weak_entity();
                        let select_owner = cx.weak_entity();
                        let edit_owner = cx.weak_entity();
                        let member_diagram = diagram.id.clone();
                        let member_view = view.id.clone();
                        let edit_member_diagram = member_diagram.clone();
                        let edit_member_view = member_view.clone();
                        // Spatial containment is presentation only, not semantic ownership validation.
                        let has_nested_views = diagram.views.iter().any(|other| {
                            other.id != view.id && matches!(other.geometry, Geometry::Rect { x: child_x, y: child_y, width: child_width, height: child_height }
                                if child_x >= x && child_y >= y + 20.0 && child_x + child_width <= x + width && child_y + child_height <= y + height)
                        });
                        let render_style = canvas_presentation::render_style(view, bellatrix_design_system::Palette::current(cx));
                        let mut classifier = UmlClassifier::new(element.name.clone())
                            .authored_contents(authored_contents)
                            .show_internal_structure(
                                has_nested_views && element.structured_classifier,
                            )
                            .presentation(view.classifier)
                            .view_style(&render_style)
                            .header(view.figure.header.clone())
                            .view_size(f64::from(local_width), f64::from(local_height), zoom)
                            .class_id(element.id.clone())
                            .selected(is_selected)
                            .on_title_edit(move |window, cx| {
                                let id = title_id.clone();
                                let _ = title_owner.update(cx, |item, cx| {
                                    item.select_diagram_view(
                                        title_diagram.clone(),
                                        title_view.clone(),
                                        cx,
                                    );
                                    item.begin_inline_edit(id, window, cx);
                                });
                            })
                            .on_member_click(move |member_id, window, cx| {
                                let id = member_id.to_owned();
                                let _ = select_owner.update(cx, |item, cx| {
                                    let key =
                                        format!("view:{member_diagram}:{member_view}/member:{id}");
                                    let occurrence = bellatrix_mde_cordis::ViewOccurrence {
                                        diagram_id: member_diagram.clone(),
                                        view_id: member_view.clone(),
                                        member_path: vec![id.clone()],
                                    };
                                    item.select_explorer_element(id, key, Some(occurrence), cx);
                                    cx.focus_self(window);
                                });
                            })
                            .on_member_edit(move |member_id, window, cx| {
                                let id = member_id.to_owned();
                                let _ = edit_owner.update(cx, |item, cx| {
                                    let key = format!(
                                        "view:{edit_member_diagram}:{edit_member_view}/member:{id}"
                                    );
                                    let occurrence = bellatrix_mde_cordis::ViewOccurrence {
                                        diagram_id: edit_member_diagram.clone(),
                                        view_id: edit_member_view.clone(),
                                        member_path: vec![id.clone()],
                                    };
                                    item.select_explorer_element(
                                        id.clone(),
                                        key,
                                        Some(occurrence),
                                        cx,
                                    );
                                    item.begin_inline_edit(id, window, cx);
                                });
                            });
                        for identifier in &element.members {
                            let Some(member) = snapshot
                                .section
                                .elements
                                .iter()
                                .find(|candidate| candidate.id == *identifier)
                            else {
                                continue;
                            };
                            let Some(text) = view.member_labels.get(&member.id).cloned() else {
                                continue;
                            };
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
                            classifier = classifier.member_presentation(&member.id, member.member);
                        }
                        for parameter in template_parameters {
                            classifier = classifier.template_parameter_with_id(
                                parameter.id.clone(),
                                parameter.template_label.clone(),
                            );
                        }
                        if editing.is_some() {
                            if let (Some(id), Some(field)) = (
                                self.presentation.editing_element().map(str::to_owned),
                                self.inline_editor.clone(),
                            ) {
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
                        let figure: gpui::AnyElement = if UmlFigure::supports(&view.figure) || view.interface_icon.is_some() {
                            let edit_owner = cx.weak_entity();
                            let edit_id = element.id.clone();
                            let edit_diagram = diagram.id.clone();
                            let edit_view = view.id.clone();
                            let mut figure =
                                UmlFigure::new(view.figure.clone(), view.display_label.clone())
                                    .interface_icon(view.interface_icon.clone())
                                    .authored_contents(authored_contents)
                                    .nested_views(has_nested_views)
                                    .with_template_parameters_from(&classifier)
                                    .view_style(&render_style)
                                    .selected(is_selected)
                                    .view_size(
                                        f64::from(local_width),
                                        f64::from(local_height),
                                        zoom,
                                    )
                                    .on_title_edit(move |window, cx| {
                                        let id = edit_id.clone();
                                        let _ = edit_owner.update(cx, |item, cx| {
                                            item.select_diagram_view(
                                                edit_diagram.clone(),
                                                edit_view.clone(),
                                                cx,
                                            );
                                            item.begin_inline_edit(id, window, cx)
                                        });
                                    });
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
                        } else if matches!(notation, UmlNotation::ClassifierRectangle) {
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
                        let drag_diagram = diagram.id.clone();
                        let connection_mode = matches!(
                            self.presentation.selected_tool(),
                            Some(ModelingTool::Relation(_))
                        );
                        let connect_owner = cx.weak_entity();
                        let connect_view = view.id.clone();
                        let connection_origin = canvas_origin.clone();
                        let resize_owner = cx.weak_entity();
                        let resize_id = view.id.clone();
                        let element_id = subject.to_owned();
                        node_bounds.borrow_mut().push([
                            local_x * zoom + pan.0,
                            local_y * zoom + pan.1 - signature_top,
                            local_width * zoom + signature_right,
                            local_height * zoom + signature_top + signature_bottom,
                        ]);
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
                                            item.source.record_interaction(
                                                if event.click_count >= 2 {
                                                    "canvas.view.double_click"
                                                } else {
                                                    "canvas.view.click"
                                                },
                                                Some(&drag_id),
                                                bellatrix_mde_cordis::TraceOutcome::Success,
                                            );
                                            if is_selected
                                                && item.presentation.selected_tool().is_none()
                                            {
                                                if let Some(field) = &item.inline_editor {
                                                    field
                                                        .read(cx)
                                                        .focus_handle(cx)
                                                        .focus(window, cx);
                                                    item.gesture = None;
                                                    return;
                                                }
                                            }
                                            if event.click_count >= 2
                                                && item.presentation.selected_tool().is_none()
                                            {
                                                item.select_diagram_view(
                                                    drag_diagram.clone(),
                                                    drag_id.clone(),
                                                    cx,
                                                );
                                                item.begin_inline_edit(
                                                    element_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            } else {
                                                item.start_move(
                                                    drag_id.clone(),
                                                    element_id.clone(),
                                                    revision,
                                                    event.position,
                                                    cx,
                                                );
                                                cx.focus_self(window);
                                            }
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
                                    is_selected && !connection_mode && view.figure.resizable,
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
                                                move |event: &MouseDownEvent, window, cx| {
                                                    if let Err(error) =
                                                        connect_owner.update(cx, |item, cx| {
                                                            item.connect_view_at_pointer(
                                                                connect_view.clone(),
                                                                event.position,
                                                                connection_origin.get(),
                                                                cx,
                                                            );
                                                            cx.focus_self(window);
                                                        })
                                                    {
                                                        eprintln!(
                                                            "MDE connection update failed: {error}"
                                                        );
                                                    }
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
        cards.extend(path_handles);
        cards.extend(relationship_controls);
        let owner = cx.weak_entity();
        let move_owner = cx.weak_entity();
        let up_owner = cx.weak_entity();
        let out_owner = cx.weak_entity();
        let dragging_owner = cx.weak_entity();
        let dropping_owner = cx.weak_entity();
        let element_drop_owner = cx.weak_entity();
        let element_drop_filter = cx.weak_entity();
        let element_drop_origin = canvas_origin.clone();
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
                        item.presentation.is_editing(),
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
                        item.presentation.is_editing(),
                    ) {
                        item.delete_selected_from_model(cx);
                    } else {
                        cx.propagate();
                    }
                }),
            )
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                let plain_enter = event.keystroke.key.eq_ignore_ascii_case("enter")
                    && !event.keystroke.modifiers.modified();
                if event.keystroke.key.eq_ignore_ascii_case("f2") || plain_enter {
                    if let Err(error) = f2_owner.update(cx, |item, cx| {
                        if !item.focus_handle.is_focused(window)
                            || item.presentation.selected_tool().is_some()
                            || item.selected_view_is_inspection_only()
                        {
                            return;
                        }
                        if let Some(field) = &item.inline_editor {
                            field.read(cx).focus_handle(cx).focus(window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        if let Some(id) = item.presentation.selected_element().map(str::to_owned) {
                            item.begin_inline_edit(id, window, cx);
                        } else if let Some(id) = item.presentation.selected_view().map(str::to_owned) {
                            item.begin_view_label_edit(id, window, cx);
                        } else {
                            return;
                        }
                        cx.stop_propagation();
                    }) {
                        eprintln!("MDE quick edit failed: {error}");
                    }
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
                        if item.gesture.take().is_some() {
                            item.feedback = "Canvas drag canceled".into();
                            cx.notify();
                            cx.stop_propagation();
                        } else if !item.presentation.is_editing() && item.presentation.selected_tool().is_some() {
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
            .when(unresolved_presentation, |workbench| {
                workbench.child(
                    div()
                        .id("mde-unresolved-canvas-presentation")
                        .flex_none()
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(border)
                        .child(
                            Label::new(
                                format!("Some presentation data is unresolved. Unsupported figures, layout, styles and resources remain preserved in the document. {}", marker_diagnostics.join("; ")),
                            )
                            .color(ui::Color::Warning),
                        ),
                )
            })
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
                            item.source.record_interaction("canvas.background.click", item.presentation.active_diagram(), bellatrix_mde_cordis::TraceOutcome::Success);
                            if let Some(tool) = item.presentation.selected_tool() {
                                let position =
                                    (f64::from(event.position.x), f64::from(event.position.y));
                                let (x, y) =
                                    canvas_model_position(position, origin, item.pan, item.zoom);
                                item.click_modeling_tool(tool, x, y, cx);
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
                    .can_drop(move |drag, _, cx| {
                        if let Some(drag) = drag.downcast_ref::<hierarchy::ExplorerElementDrag>() {
                            return element_drop_filter.upgrade()
                                .is_some_and(|owner| owner.read(cx).validate_explorer_drop(drag).is_ok());
                        }
                        drag.is::<MdeToolDrag>()
                    })
                    .on_drop(move |drag: &hierarchy::ExplorerElementDrag, window, cx| {
                        let Some(origin) = element_drop_origin.get() else { return; };
                        let position = window.mouse_position();
                        if let Err(error) = element_drop_owner.update(cx, |item, cx| {
                            let point = canvas_model_position(
                                (f64::from(position.x), f64::from(position.y)), origin, item.pan, item.zoom,
                            );
                            item.drop_explorer_element(drag, point, window, cx);
                        }) {
                            eprintln!("MDE existing element drop failed: {error}");
                        }
                    })
                    .on_drop(move |drag: &MdeToolDrag, _, cx| {
                        if let Err(error) = dropping_owner.update(cx, |item, cx| {
                            let label = item.source.snapshot().ok().and_then(|snapshot| {
                                snapshot.section.tools.label(drag.kind).map(str::to_owned)
                            });
                            if &drag.project_id != item.project_id() {
                                item.source.record_interaction("canvas.tool_drop.wrong_project", label.as_deref(), bellatrix_mde_cordis::TraceOutcome::Invalid);
                                return;
                            }
                            if item.source.is_read_only() {
                                item.source.record_interaction("canvas.tool_drop.read_only", label.as_deref(), bellatrix_mde_cordis::TraceOutcome::Invalid);
                                return;
                            }
                            item.source.record_interaction("canvas.tool_drop.accepted", label.as_deref(), bellatrix_mde_cordis::TraceOutcome::Success);
                            let (x, y) = item.drop_point.take().unwrap_or((40.0, 40.0));
                            item.place_modeling_tool(drag.kind, x, y, cx);
                        }) {
                            eprintln!("MDE tool drop failed: {error}");
                        }
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

impl MdeWorkbenchItem {
    fn preview_rectangle(
        &self,
        section: &bellatrix_mde_cordis::projection::ModelingProjection,
        diagram: &bellatrix_mde_cordis::projection::Diagram,
        view: &bellatrix_mde_cordis::projection::View,
    ) -> Option<(f64, f64, f64, f64)> {
        let Geometry::Rect { mut x, mut y, mut width, mut height } = view.geometry else { return None; };
        match &self.gesture {
            Some(CanvasGesture::Move { view_id, dx, dy, .. }) => {
                if view_id == &view.id || diagram.all_views().find(|candidate| &candidate.id == view_id)
                    .is_some_and(|ancestor| contains_view(&ancestor.children, &view.id))
                {
                    x += dx;
                    y += dy;
                }
            }
            Some(CanvasGesture::Resize { view_id, dx, dy, .. }) => {
                if let Some(ancestor) = diagram.all_views().find(|candidate| &candidate.id == view_id) {
                    let minimum = canvas_presentation::minimum_size(section, ancestor);
                    if view_id == &view.id {
                        width = (width + dx).max(minimum.0);
                        height = (height + dy).max(minimum.1);
                    } else if contains_view(&ancestor.children, &view.id) {
                        if let Geometry::Rect { x: left, y: top, width: parent_width, height: parent_height } = ancestor.geometry {
                            if parent_width > 0.0 && parent_height > 0.0 {
                                let horizontal = (parent_width + dx).max(minimum.0) / parent_width;
                                let vertical = (parent_height + dy).max(minimum.1) / parent_height;
                                x = left + (x - left) * horizontal;
                                y = top + (y - top) * vertical;
                                width *= horizontal;
                                height *= vertical;
                            }
                        }
                    }
                }
            }
            _ => {},
        }
        canvas_presentation::resolve_bounds(&Geometry::Rect { x, y, width, height }, self.zoom, self.pan)?;
        Some((x, y, width, height))
    }

    fn render_presentation_view(
        &self,
        section: &bellatrix_mde_cordis::projection::ModelingProjection,
        diagram: &bellatrix_mde_cordis::projection::Diagram,
        view: &bellatrix_mde_cordis::projection::View,
        revision: u64,
        node_bounds: &mut Vec<[f32; 4]>,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let (x, y, width, height) = self.preview_rectangle(section, diagram, view)?;
        let owner_view = diagram.all_views().find(|owner| owner.presentation.name_label.as_deref() == Some(view.id.as_str()));
        let subject = owner_view.and_then(|owner| owner.subject.clone()).or_else(|| view.subject.clone());
        let interaction_owner = owner_view.or_else(|| {
            view.presentation.internal.then(|| diagram.all_views().find(|ancestor|
                contains_view(&ancestor.children, &view.id)
            )).flatten()
        });
        let target_view = interaction_owner.unwrap_or(view);
        let selected = self.presentation.selected_canvas_view() == Some(view.id.as_str());
        let style = canvas_presentation::render_style(view, bellatrix_design_system::Palette::current(cx));
        let label = if view.figure.notation == UmlNotation::Unsupported {
            format!("Unsupported figure · {}", view.id)
        } else {
            view.rendered_label.text.clone().unwrap_or_else(|| view.display_label.clone())
        };
        let title_owner = cx.weak_entity();
        let title_subject = subject.clone();
        let title_diagram = diagram.id.clone();
        let title_view = target_view.id.clone();
        let mut figure = UmlFigure::new(view.figure.clone(), label)
            .interface_icon(view.interface_icon.clone())
            .authored_contents(view.children.iter().any(|child| child.presentation.internal))
            .divider(view.presentation.divider)
            .view_style(&style)
            .view_size(width, height, self.zoom)
            .selected(selected)
            .on_title_edit(move |window, cx| {
                if let Err(error) = title_owner.update(cx, |item, cx| {
                    item.select_diagram_view(title_diagram.clone(), title_view.clone(), cx);
                    if let Some(subject) = &title_subject {
                        item.begin_inline_edit(subject.clone(), window, cx);
                    } else {
                        item.begin_view_label_edit(title_view.clone(), window, cx);
                    }
                }) { eprintln!("MDE label edit failed: {error}"); }
            });
        if subject.is_some() && subject.as_deref() == self.presentation.editing_element() {
            if let Some(field) = self.inline_editor.clone() {
                let confirm = cx.weak_entity();
                let cancel = cx.weak_entity();
                figure = figure.inline_edit(field, move |window, cx| {
                    if let Err(error) = confirm.update(cx, |item, cx| item.commit_inline_edit(window, cx)) {
                        eprintln!("MDE label commit failed: {error}");
                    }
                }, move |window, cx| {
                    if let Err(error) = cancel.update(cx, |item, cx| item.cancel_inline_edit(window, cx)) {
                        eprintln!("MDE label cancel failed: {error}");
                    }
                });
            }
        }
        if subject.is_none() && self.presentation.editing_view() == Some(view.id.as_str()) {
            if let Some(field) = self.inline_editor.clone() {
                let editor_id = field.entity_id();
                let edited_view = view.id.clone();
                let confirm = cx.weak_entity();
                let cancel = cx.weak_entity();
                figure = figure.inline_edit(field, move |window, cx| {
                    if let Err(error) = confirm.update(cx, |item, cx| item.commit_view_label(&edited_view, editor_id, window, cx)) {
                        eprintln!("MDE view label commit failed: {error}");
                    }
                }, move |window, cx| {
                    if let Err(error) = cancel.update(cx, |item, cx| item.cancel_inline_edit(window, cx)) {
                        eprintln!("MDE view label cancel failed: {error}");
                    }
                });
            }
        }
        let owner = cx.weak_entity();
        let diagram_id = diagram.id.clone();
        let view_id = target_view.id.clone();
        let move_subject = target_view.subject.clone();
        let internal = view.presentation.internal;
        let editable_label = view.figure.notation != UmlNotation::Compartment;
        let resize_owner = cx.weak_entity();
        let resize_id = view.id.clone();
        let can_resize = selected && !internal && !self.source.is_read_only();
        if !matches!(view.figure.notation, UmlNotation::Label | UmlNotation::Compartment) {
            node_bounds.push([x as f32 * self.zoom + self.pan.0, y as f32 * self.zoom + self.pan.1,
                width as f32 * self.zoom, height as f32 * self.zoom]);
        }
        Some(div().id(format!("mde-view-{}", view.id)).absolute()
            .left(px(x as f32 * self.zoom + self.pan.0))
            .top(px(y as f32 * self.zoom + self.pan.1))
            .w(px(width as f32 * self.zoom)).h(px(height as f32 * self.zoom))
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if let Err(error) = owner.update(cx, |item, cx| {
                    item.select_diagram_view(diagram_id.clone(), view_id.clone(), cx);
                    if event.click_count >= 2 && editable_label {
                        if let Some(subject) = &subject {
                            item.begin_inline_edit(subject.clone(), window, cx);
                        } else if !internal {
                            item.begin_view_label_edit(view_id.clone(), window, cx);
                        }
                    } else if let Some(subject) = &move_subject {
                        item.start_move(view_id.clone(), subject.clone(), revision, event.position, cx);
                    } else if !internal && !item.source.is_read_only() {
                        item.gesture = Some(CanvasGesture::Move {
                            view_id: view_id.clone(), start: event.position,
                            dx: 0.0, dy: 0.0, revision, began: false,
                        });
                    }
                    cx.focus_self(window);
                }) { eprintln!("MDE presentation selection failed: {error}"); }
                cx.stop_propagation();
            }).child(figure)
            .when(can_resize, |card| card.child(
                div().id(format!("mde-presentation-resize-{}", view.id))
                    .absolute().right(px(-4.0)).bottom(px(-4.0)).w(px(9.0)).h(px(9.0))
                    .bg(WorkbenchPalette::current(cx).selection).cursor_crosshair()
                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                        if let Err(error) = resize_owner.update(cx, |item, cx| {
                            item.start_resize(resize_id.clone(), width, height, revision, event.position, cx);
                        }) { eprintln!("MDE presentation resize failed: {error}"); }
                        cx.stop_propagation();
                    })
            )).into_any_element())
    }

}

impl MdeWorkbenchItem {
    pub(super) fn path_preview_geometry(
        &self,
        view: &bellatrix_mde_cordis::projection::View,
    ) -> Geometry {
        if let Some(CanvasGesture::EditPath {
            view_id,
            geometry,
            handle,
            dx,
            dy,
            ..
        }) = &self.gesture
            && view_id == &view.id
        {
            return edited_path_geometry(geometry, *handle, *dx, *dy);
        }
        view.routed_geometry.as_ref().unwrap_or(&view.geometry).clone()
    }

    fn path_is_editable(&self, view: &bellatrix_mde_cordis::projection::View) -> bool {
        if self.source.is_read_only() || view.relationship.is_none() {
            return false;
        }
        let Geometry::Path { segments, .. } = &view.geometry else {
            return false;
        };
        !segments.is_empty()
    }

    pub(super) fn start_path_edit(
        &mut self,
        diagram_id: String,
        view_id: String,
        revision: u64,
        handle: PathHandle,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.presentation.selected_tool().is_some() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            return;
        };
        if snapshot.revision != revision
            || self.presentation.active_diagram() != Some(diagram_id.as_str())
        {
            return;
        }
        let Some(view) = snapshot.section.diagrams.iter()
            .find(|diagram| diagram.id == diagram_id)
            .and_then(|diagram| diagram.all_views().find(|view| view.id == view_id))
        else {
            return;
        };
        if !self.path_is_editable(view) {
            return;
        }
        let geometry = view.routed_geometry.as_ref().unwrap_or(&view.geometry).clone();
        let change = self.presentation.select_view(view.subject.clone(), view_id.clone());
        self.apply_selection_change(change, cx);
        self.gesture = Some(CanvasGesture::EditPath {
            diagram_id,
            view_id,
            geometry,
            handle,
            start: position,
            dx: 0.0,
            dy: 0.0,
            revision,
            began: false,
        });
        self.feedback = "Editing route · endpoints stay attached · Escape cancels".into();
        cx.notify();
    }

    fn render_path_handles(
        &self,
        diagram_id: &str,
        view: &bellatrix_mde_cordis::projection::View,
        revision: u64,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        if self.presentation.selected_canvas_view() != Some(view.id.as_str())
            || self.presentation.selected_tool().is_some()
            || self.presentation.is_editing()
            || !self.path_is_editable(view)
        {
            return Vec::new();
        }
        let palette = WorkbenchPalette::current(cx);
        let geometry = self.path_preview_geometry(view);
        path_handle_positions(&geometry).into_iter().enumerate().filter_map(|(index, (handle, point))| {
            let x = point.x as f32 * self.zoom + self.pan.0;
            let y = point.y as f32 * self.zoom + self.pan.1;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let owner = cx.weak_entity();
            let diagram_id = diagram_id.to_owned();
            let view_id = view.id.clone();
            let segment = matches!(handle, PathHandle::Segment(_));
            let control = matches!(handle, PathHandle::Control1(_) | PathHandle::Control2(_));
            let size = if segment { 7.0 } else { 9.0 };
            Some(div()
                .id(format!("mde-path-handle-{}-{index}", view.id))
                .absolute()
                .left(px(x - size / 2.0))
                .top(px(y - size / 2.0))
                .size(px(size))
                .border_1()
                .border_color(palette.selection)
                .bg(if segment { palette.surface } else { palette.selection })
                .when(segment || control, |handle| handle.rounded_full())
                .cursor_move()
                .tooltip(ui::Tooltip::text(
                    if segment { "Drag segment; endpoints stay attached" }
                    else if control { "Drag curve control" }
                    else { "Drag route point" }))
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    if let Err(error) = owner.update(cx, |item, cx| {
                        item.start_path_edit(diagram_id.clone(), view_id.clone(), revision, handle, event.position, cx);
                        cx.focus_self(window);
                    }) {
                        eprintln!("MDE route edit failed: {error}");
                    }
                    cx.stop_propagation();
                })
                .into_any_element())
        }).collect()
    }
}

pub(super) fn path_segment_at_position(
    geometry: &Geometry,
    position: bellatrix_mde_cordis::Point,
    zoom: f32,
) -> Option<PathHandle> {
    let Geometry::Path { start, segments } = geometry else {
        return None;
    };
    let tolerance = 5.0 / f64::from(zoom);
    let mut previous = *start;
    for (index, segment) in segments.iter().enumerate() {
        let (hit, end) = match segment {
            PathSegment::Line { to } => (
                relationship_segment_distance(position, previous, *to) <= tolerance,
                *to,
            ),
            PathSegment::Cubic { control1, control2, to } => (
                relationship_cubic_hit(position, [previous, *control1, *control2, *to], tolerance, 0.25 / f64::from(zoom), 0),
                *to,
            ),
        };
        if hit {
            return Some(PathHandle::Segment(index));
        }
        previous = end;
    }
    None
}

fn path_handle_positions(geometry: &Geometry) -> Vec<(PathHandle, bellatrix_mde_cordis::Point)> {
    let Geometry::Path { start, segments } = geometry else {
        return Vec::new();
    };
    let mut handles = Vec::new();
    let mut previous = *start;
    for (index, segment) in segments.iter().enumerate() {
        let end = match segment {
            PathSegment::Line { to } => {
                if (to.x - previous.x).hypot(to.y - previous.y) > f64::EPSILON {
                    handles.push((PathHandle::Segment(index), bellatrix_mde_cordis::Point {
                        x: previous.x / 2.0 + to.x / 2.0,
                        y: previous.y / 2.0 + to.y / 2.0,
                    }));
                }
                *to
            }
            PathSegment::Cubic { control1, control2, to } => {
                handles.push((PathHandle::Control1(index), *control1));
                handles.push((PathHandle::Control2(index), *control2));
                handles.push((PathHandle::Segment(index), bellatrix_mde_cordis::Point {
                    x: previous.x * 0.125 + control1.x * 0.375 + control2.x * 0.375 + to.x * 0.125,
                    y: previous.y * 0.125 + control1.y * 0.375 + control2.y * 0.375 + to.y * 0.125,
                }));
                *to
            }
        };
        if index + 1 < segments.len() {
            handles.push((PathHandle::Vertex(index), end));
        }
        previous = end;
    }
    handles
}

pub(super) fn edited_path_geometry(
    geometry: &Geometry,
    handle: PathHandle,
    dx: f64,
    dy: f64,
) -> Geometry {
    if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
        return geometry.clone();
    }
    let mut edited = geometry.clone();
    let Geometry::Path { start, segments } = &mut edited else {
        return edited;
    };
    let translate = |point: &mut bellatrix_mde_cordis::Point| {
        point.x += dx;
        point.y += dy;
    };
    match handle {
        PathHandle::Vertex(index) if index + 1 < segments.len() => {
            translate_path_vertex(segments, index, dx, dy);
        }
        PathHandle::Control1(index) => {
            if let Some(PathSegment::Cubic { control1, .. }) = segments.get_mut(index) {
                translate(control1);
            }
        }
        PathHandle::Control2(index) => {
            if let Some(PathSegment::Cubic { control2, .. }) = segments.get_mut(index) {
                translate(control2);
            }
        }
        PathHandle::Segment(index) => {
            let previous = if index == 0 { *start } else {
                match segments.get(index - 1) {
                    Some(PathSegment::Line { to } | PathSegment::Cubic { to, .. }) => *to,
                    None => return geometry.clone(),
                }
            };
            match segments.get(index) {
                Some(PathSegment::Cubic { .. }) => {
                    if let Some(PathSegment::Cubic { control1, control2, .. }) = segments.get_mut(index) {
                        translate(control1);
                        translate(control2);
                    }
                }
                Some(PathSegment::Line { to }) => {
                    let end = *to;
                    let (dx, dy) = if (previous.x - end.x).abs() < 0.000001 {
                        (dx, 0.0)
                    } else if (previous.y - end.y).abs() < 0.000001 {
                        (0.0, dy)
                    } else { (dx, dy) };
                    if dx == 0.0 && dy == 0.0 {
                        return geometry.clone();
                    }
                    let shifted = |point: bellatrix_mde_cordis::Point| bellatrix_mde_cordis::Point {
                        x: point.x + dx,
                        y: point.y + dy,
                    };
                    let mut replacement = Vec::new();
                    if index == 0 {
                        // Add a bend instead of detaching the first endpoint.
                        replacement.push(PathSegment::Line { to: shifted(previous) });
                    } else {
                        translate_path_vertex(segments, index - 1, dx, dy);
                    }
                    replacement.push(PathSegment::Line { to: shifted(end) });
                    if index + 1 == segments.len() {
                        replacement.push(PathSegment::Line { to: end });
                    } else if let Some(PathSegment::Cubic { control1, .. }) = segments.get_mut(index + 1) {
                        control1.x += dx;
                        control1.y += dy;
                    }
                    segments.splice(index..=index, replacement);
                }
                None => {}
            }
        }
        PathHandle::Vertex(_) => {}
    }
    edited
}

fn translate_path_vertex(segments: &mut [PathSegment], index: usize, dx: f64, dy: f64) {
    let translate = |point: &mut bellatrix_mde_cordis::Point| {
        point.x += dx;
        point.y += dy;
    };
    match segments.get_mut(index) {
        Some(PathSegment::Line { to }) => translate(to),
        Some(PathSegment::Cubic { control2, to, .. }) => {
            translate(control2);
            translate(to);
        }
        None => return,
    }
    if let Some(PathSegment::Cubic { control1, .. }) = segments.get_mut(index + 1) {
        translate(control1);
    }
}

pub(super) fn relationship_hit_test(
    geometry: &Geometry,
    position: gpui::Point<gpui::Pixels>,
    origin: Option<(f64, f64)>,
    pan: (f32, f32),
    zoom: f32,
    node_bounds: &[[f32; 4]],
) -> bool {
    let Some(origin) = origin else {
        return false;
    };
    if !zoom.is_finite() || zoom <= 0.0 {
        return false;
    }
    let canvas_x = f64::from(position.x) - origin.0;
    let canvas_y = f64::from(position.y) - origin.1;
    // Preserve paint order, but never let a crossing relationship steal a node's click.
    if node_bounds.iter().any(|[x, y, width, height]| {
        canvas_x >= f64::from(*x)
            && canvas_x <= f64::from(*x + *width)
            && canvas_y >= f64::from(*y)
            && canvas_y <= f64::from(*y + *height)
    }) {
        return false;
    }
    let Geometry::Path { start, segments } = geometry else {
        return false;
    };
    let (x, y) = canvas_model_position(
        (f64::from(position.x), f64::from(position.y)),
        origin,
        pan,
        zoom,
    );
    let position = bellatrix_mde_cordis::Point { x, y };
    // Keep the clickable stroke narrow at every zoom; control points are not painted edges.
    let tolerance = 5.0 / f64::from(zoom);
    let flatness = 0.25 / f64::from(zoom);
    let mut previous = *start;
    for segment in segments {
        let (hit, end) = match segment {
            PathSegment::Line { to } => (
                relationship_segment_distance(position, previous, *to) <= tolerance,
                *to,
            ),
            PathSegment::Cubic {
                control1,
                control2,
                to,
            } => (
                relationship_cubic_hit(
                    position,
                    [previous, *control1, *control2, *to],
                    tolerance,
                    flatness,
                    0,
                ),
                *to,
            ),
        };
        if hit {
            return true;
        }
        previous = end;
    }
    false
}

fn relationship_segment_distance(
    position: bellatrix_mde_cordis::Point,
    start: bellatrix_mde_cordis::Point,
    end: bellatrix_mde_cordis::Point,
) -> f64 {
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let length_squared = delta_x * delta_x + delta_y * delta_y;
    let projection = if length_squared > f64::EPSILON {
        (((position.x - start.x) * delta_x + (position.y - start.y) * delta_y) / length_squared)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    (position.x - start.x - projection * delta_x).hypot(position.y - start.y - projection * delta_y)
}

fn relationship_cubic_hit(
    position: bellatrix_mde_cordis::Point,
    points: [bellatrix_mde_cordis::Point; 4],
    tolerance: f64,
    flatness: f64,
    depth: u8,
) -> bool {
    let [start, control1, control2, end] = points;
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    if position.x < min_x - tolerance
        || position.x > max_x + tolerance
        || position.y < min_y - tolerance
        || position.y > max_y + tolerance
    {
        return false;
    }
    if depth >= 16
        || (relationship_segment_distance(control1, start, end) <= flatness
            && relationship_segment_distance(control2, start, end) <= flatness)
    {
        return relationship_segment_distance(position, start, end) <= tolerance;
    }
    let midpoint = |left: bellatrix_mde_cordis::Point, right: bellatrix_mde_cordis::Point| {
        bellatrix_mde_cordis::Point {
            x: left.x + (right.x - left.x) / 2.0,
            y: left.y + (right.y - left.y) / 2.0,
        }
    };
    let first = midpoint(start, control1);
    let middle = midpoint(control1, control2);
    let last = midpoint(control2, end);
    let left = midpoint(first, middle);
    let right = midpoint(middle, last);
    let split = midpoint(left, right);
    relationship_cubic_hit(
        position,
        [start, first, left, split],
        tolerance,
        flatness,
        depth + 1,
    ) || relationship_cubic_hit(
        position,
        [split, right, last, end],
        tolerance,
        flatness,
        depth + 1,
    )
}

fn contains_view(views: &[bellatrix_mde_cordis::projection::View], id: &str) -> bool {
    views.iter().any(|view| view.id == id || contains_view(&view.children, id))
}
