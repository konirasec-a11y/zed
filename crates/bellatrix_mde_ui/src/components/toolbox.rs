use crate::components::pane_header::PaneHeader;
use crate::layouts::sidebar::MDE_LEFT_PRIORITY;
use crate::layouts::sidebar::PanelState;
use crate::layouts::sidebar::ToggleMdeLeft;
use crate::pages::mde_page::MdeToolDrag;
use crate::pages::mde_page::MdeWorkbenchItem;
use crate::pages::mde_page::WorkbenchPalette;
use crate::routes::workspace::OpenMdeSymbols;
use crate::routes::workspace::OpenMdeWorkbench;
use bellatrix_mde_cordis::ModelingElementKind;
use bellatrix_mde_cordis::ModelingRelationKind;
use bellatrix_mde_cordis::ModelingTool;
use gpui::Action;
use gpui::App;
use gpui::Context;
use gpui::Entity;
use gpui::EventEmitter;
use gpui::FocusHandle;
use gpui::Focusable;
use gpui::IntoElement;
use gpui::MouseButton;
use gpui::MouseDownEvent;
use gpui::MouseMoveEvent;
use gpui::MouseUpEvent;
use gpui::Pixels;
use gpui::Render;
use gpui::Subscription;
use gpui::Window;
use gpui::div;
use gpui::prelude::*;
use gpui::px;
use std::collections::HashSet;
use ui::IconName;
use ui::Label;
use ui::TreeViewItem;
use ui::prelude::*;
use ui_input::ErasedEditorEvent;
use ui_input::InputField;
use workspace::dock::DockPosition;
use workspace::dock::Panel;
use workspace::dock::PanelEvent;

pub struct MdeLeftPanel {
    pub(crate) state: PanelState,
    diagrams_open: bool,
    toolbox_open: bool,
    search_open: bool,
    library_open: bool,
    pub(crate) search_input: Entity<InputField>,
    expanded_tool_groups: HashSet<usize>,
    _search_listener: Subscription,
}
impl MdeLeftPanel {
    pub fn new(
        item: Entity<MdeWorkbenchItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| {
            InputField::new(window, cx, "Search symbols…")
                .start_icon(IconName::MagnifyingGlass)
                .label_min_width(px(0.0))
        });
        let owner = cx.weak_entity();
        let editor = search_input.read(cx).editor().clone();
        let search_listener = editor.subscribe(
            Box::new(move |event, _, cx| {
                if event == ErasedEditorEvent::BufferEdited {
                    if let Err(error) = owner.update(cx, |_, cx| cx.notify()) {
                        eprintln!("MDE toolbox search failed: {error}");
                    }
                }
            }),
            window,
            cx,
        );
        Self {
            state: PanelState::new(item, DockPosition::Left, 216.0, cx),
            diagrams_open: true,
            toolbox_open: true,
            search_open: false,
            library_open: false,
            search_input,
            expanded_tool_groups: HashSet::new(),
            _search_listener: search_listener,
        }
    }
}
impl EventEmitter<PanelEvent> for MdeLeftPanel {}
impl Focusable for MdeLeftPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.state.focus.clone()
    }
}
impl Panel for MdeLeftPanel {
    fn persistent_name() -> &'static str {
        "MDE Working Diagrams & Toolbox"
    }
    fn panel_key() -> &'static str {
        "bellatrix.mde.left.v1"
    }
    fn position(&self, _: &Window, _: &App) -> DockPosition {
        self.state.position
    }
    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }
    fn set_position(&mut self, position: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        if self.position_is_valid(position) {
            self.state.position = position;
            cx.notify();
        }
    }
    fn default_size(&self, _: &Window, _: &App) -> Pixels {
        px(246.0)
    }
    fn min_size(&self, _: &Window, _: &App) -> Option<Pixels> {
        Some(px(190.0))
    }
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
        Some(IconName::BellatrixMdeTools)
    }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Bellatrix MDE - Working Diagrams & Toolbox (left dock)")
    }
    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleMdeLeft)
    }
    fn starts_open(&self, _: &Window, _: &App) -> bool {
        true
    }
    fn activation_priority(&self) -> u32 {
        MDE_LEFT_PRIORITY
    }
}
impl Render for MdeLeftPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(item) = self.state.item.as_ref().and_then(gpui::WeakEntity::upgrade) else {
            return div()
                .p_3()
                .child(Label::new("Select an MDE document to inspect its model."))
                .into_any_element();
        };
        let read_only = item.read(cx).source().is_read_only();
        let palette = WorkbenchPalette::current(cx);
        let (active, chosen, snapshot, visual_draft_mode) = {
            let state = item.read(cx);
            (
                state.active_diagram().map(str::to_owned),
                state.selected_tool(),
                state.source().presentation_snapshot().ok(),
                state.presentation.visual_draft_mode(),
            )
        };
        let catalog = snapshot
            .as_ref()
            .map(|snapshot| snapshot.section.tools.clone())
            .unwrap_or_default();
        let profile = active
            .as_deref()
            .and_then(|id| catalog.profiles.get(id))
            .cloned();
        let creation_available = item.read(cx).source().allows_creation();
        let native_tools = creation_available
            && profile
                .as_ref()
                .is_none_or(|profile| !profile.available_tools.is_empty());
        let chosen = chosen.filter(|tool| catalog.allows_tool(active.as_deref(), *tool));
        let mut diagrams = Vec::new();
        if let Some(snapshot) = &snapshot {
            for diagram in &snapshot.section.diagrams {
                let diagram_id = diagram.id.clone();
                let owner = item.clone();
                let diagram_kind = diagram
                    .kind
                    .map(|kind| crate::pages::mde_page::diagram_kind_label(kind).to_owned())
                    .unwrap_or_else(|| diagram.type_label.clone());
                let display_name = diagram
                    .owner_label
                    .as_ref()
                    .map(|owner| format!("{} · {owner}", diagram.name))
                    .unwrap_or_else(|| diagram.name.clone());
                diagrams.push(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_2()
                        .when(active.as_deref() == Some(diagram_id.as_str()), |row| {
                            row.bg(palette.surface)
                        })
                        .child(crate::components::model_explorer::explorer_diagram_icon(
                            &diagram.uml_di_type,
                            &palette,
                        ))
                        .child(
                            div().flex_1().min_w_0().child(
                                TreeViewItem::new(diagram_id.clone(), display_name)
                                    .tooltip(ui::Tooltip::text(diagram_kind))
                                    .toggle_state(active.as_deref() == Some(diagram_id.as_str()))
                                    .on_click(move |_, window, cx| {
                                        if owner.read(cx).source().is_presentation_suspended() {
                                            return;
                                        }
                                        owner.update(cx, |workbench, cx| {
                                            workbench.select_diagram(diagram_id.clone(), cx)
                                        });
                                        window.focus(&owner.read(cx).focus_handle(cx), cx);
                                    }),
                            ),
                        ),
                );
            }
        }
        let create_item = item.clone();
        let diagram_kinds = catalog
            .diagrams
            .iter()
            .map(|entry| entry.kind)
            .collect::<Vec<_>>();
        let create_menu = ui::ContextMenu::build(window, cx, move |menu, _, _| {
            diagram_kinds.into_iter().fold(menu, |menu, kind| {
                let owner = create_item.clone();
                let label = crate::pages::mde_page::diagram_kind_label(kind).to_owned();
                menu.entry(label, None, move |window, cx| {
                    if owner.read(cx).source().is_read_only() {
                        return;
                    }
                    owner.update(cx, |workbench, cx| workbench.create_diagram(kind, cx));
                    window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
                })
            })
        });
        let tool_groups = catalog
            .groups
            .iter()
            .map(|group| {
                (
                    group.name.clone(),
                    group
                        .tools
                        .iter()
                        .copied()
                        .filter(|tool| catalog.allows_tool(active.as_deref(), *tool))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let query = self.search_input.read(cx).text(cx).trim().to_lowercase();
        let searching = !query.is_empty();
        let mut tool_buttons: Vec<gpui::AnyElement> = Vec::new();
        let selection_owner = item.clone();
        tool_buttons.push(
            div()
                .p_2()
                .child(
                    Button::new("mde-domain-select", "Select")
                        .toggle_state(chosen.is_none())
                        .tooltip(ui::Tooltip::text("Select and inspect existing objects"))
                        .on_click(move |_, _, cx| {
                            selection_owner.update(cx, |workbench, cx| workbench.cancel_tool(cx))
                        }),
                )
                .into_any_element(),
        );
        if native_tools {
            if !searching {
                tool_buttons.push(
                    div()
                        .px_2()
                        .pt_2()
                        .pb_1()
                        .text_xs()
                        .text_color(palette.muted)
                        .child("QUICK TOOLS")
                        .into_any_element(),
                );
                for (index, tool) in [
                    ModelingTool::Element(ModelingElementKind::Class),
                    ModelingTool::Element(ModelingElementKind::Interface),
                    ModelingTool::Relation(ModelingRelationKind::Association),
                    ModelingTool::Relation(ModelingRelationKind::Composition),
                    ModelingTool::Element(ModelingElementKind::Package),
                    ModelingTool::Element(ModelingElementKind::Comment),
                ]
                .into_iter()
                .enumerate()
                {
                    if !catalog.allows_tool(active.as_deref(), tool) {
                        continue;
                    }
                    tool_buttons.push(tool_entry(
                        format!("mde-quick-tool-{index}"),
                        tool,
                        catalog.label(tool).unwrap_or("Tool unavailable").to_owned(),
                        chosen,
                        &item,
                        cx,
                    ));
                }
            }
            tool_buttons.push(
                div()
                    .pt_2()
                    .child(
                        Button::new(
                            "mde-toggle-symbol-library",
                            if searching {
                                "Search results"
                            } else {
                                "Symbol library"
                            },
                        )
                        .disabled(searching)
                        .start_icon(ui::Icon::new(if self.library_open || searching {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        }))
                        .on_click(cx.listener(|panel, _, _, cx| {
                            panel.library_open = !panel.library_open;
                            cx.notify();
                        })),
                    )
                    .into_any_element(),
            );
            let mut matching_groups = 0;
            for (group_index, (group_name, tools)) in tool_groups.into_iter().enumerate() {
                if !self.library_open && !searching {
                    break;
                }
                let tools: Vec<_> = tools
                    .into_iter()
                    .filter(|tool| {
                        catalog
                            .label(*tool)
                            .is_some_and(|label| tool_matches_query(label, &query))
                    })
                    .collect();
                if tools.is_empty() {
                    continue;
                }
                matching_groups += 1;
                let expanded = searching || self.expanded_tool_groups.contains(&group_index);
                tool_buttons.push(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .pr_2()
                        .child(
                            Button::new(format!("mde-tool-group-{group_index}"), group_name)
                                .start_icon(ui::Icon::new(if expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                }))
                                .on_click(cx.listener(move |panel, _, _, cx| {
                                    if !panel.expanded_tool_groups.insert(group_index) {
                                        panel.expanded_tool_groups.remove(&group_index);
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child(tools.len().to_string()),
                        )
                        .into_any_element(),
                );
                if expanded {
                    for (index, tool) in tools.into_iter().enumerate() {
                        tool_buttons.push(tool_entry(
                            format!("mde-tool-{group_index}-{index}"),
                            tool,
                            catalog.label(tool).unwrap_or("Tool unavailable").to_owned(),
                            chosen,
                            &item,
                            cx,
                        ));
                    }
                }
            }
            if searching && matching_groups == 0 {
                tool_buttons.push(
                    div()
                        .p_2()
                        .text_xs()
                        .text_color(palette.muted)
                        .child("No symbols found.")
                        .into_any_element(),
                );
            }
        }
        if let Some(profile) = &profile {
            tool_buttons.push(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(palette.muted)
                    .child(profile.label.clone())
                    .into_any_element(),
            );
            for (index, tool) in profile.unavailable_tools.iter().enumerate() {
                tool_buttons.push(
                    div()
                        .px_2()
                        .py_1()
                        .child(
                            Button::new(
                                format!("mde-unavailable-domain-tool-{index}"),
                                format!("{} · unavailable", tool.label),
                            )
                            .disabled(true)
                            .tooltip(ui::Tooltip::text(tool.reason.clone())),
                        )
                        .into_any_element(),
                );
            }
            if !native_tools {
                tool_buttons.push(div().p_2().text_xs().text_color(palette.muted)
                    .child("Imported diagram: selection and inspection are available. Creation tools for this domain are not implemented yet.")
                    .into_any_element());
            }
        }
        if profile.is_none() && !creation_available {
            tool_buttons.push(div().p_2().text_xs().text_color(palette.muted)
                .child("Selection and inspection are available. Creation commands are not enabled for this document.")
                .into_any_element());
        }
        let project_name = snapshot.as_ref().map_or_else(
            || "Project unavailable".to_owned(),
            |snapshot| snapshot.name.clone(),
        );
        div()
            .id("mde-left-panel")
            .track_focus(&self.state.focus)
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(palette.panel)
            .when(read_only, |panel| {
                panel.child(Label::new("Read-only preview"))
            })
            .on_mouse_move(cx.listener(|panel, event: &MouseMoveEvent, _, cx| {
                if panel.state.move_split(event.position.y, event.dragging()) {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|panel, _: &MouseUpEvent, _, _| panel.state.finish_split()),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|panel, _: &MouseUpEvent, _, _| panel.state.finish_split()),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .h(px(if self.diagrams_open {
                        self.state.top_height
                    } else {
                        36.0
                    }))
                    .when(self.diagrams_open && !self.toolbox_open, |pane| {
                        pane.flex_1()
                    })
                    .child(
                        PaneHeader::new("Working diagrams")
                            .toggle(
                                self.diagrams_open,
                                cx.listener(|panel, _, _, cx| {
                                    panel.diagrams_open = !panel.diagrams_open;
                                    cx.notify();
                                }),
                            )
                            .when(creation_available && profile.is_none(), |header| {
                                header.action(ui::DropdownMenu::new(
                                    "mde-new-diagram",
                                    "",
                                    create_menu,
                                ).trigger_icon(IconName::Plus)
                                    .aria_label("New diagram")
                                    .trigger_tooltip(ui::Tooltip::text("New diagram")))
                            }),
                    )
                    .when(self.diagrams_open, |pane| {
                        pane.child(
                            div()
                                .id("bellatrix-mde-diagrams-scroll")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .child(
                                    div()
                                        .h_8()
                                        .px_3()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(crate::elements::uml::UmlVisualDefinition::for_project().render_icon(20.0, palette.panel))
                                        .child(
                                            div()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .child(Label::new(project_name)),
                                        ),
                                )
                                .children(diagrams)
                                .when(
                                    snapshot.as_ref().is_some_and(|snapshot| {
                                        snapshot.section.diagrams.is_empty()
                                    }),
                                    |pane| {
                                        pane.child(
                                            div()
                                                .p_3()
                                                .text_xs()
                                                .text_color(palette.muted)
                                                .child("Create a diagram to start modeling."),
                                        )
                                    },
                                ),
                        )
                    }),
            )
            .when(self.diagrams_open && self.toolbox_open, |panel| {
                panel.child(
                    div()
                        .id("mde-left-stack-resizer")
                        .h(px(4.0))
                        .w_full()
                        .flex_none()
                        .cursor_ns_resize()
                        .border_b_1()
                        .border_color(palette.border)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|panel, event: &MouseDownEvent, _, cx| {
                                panel.state.start_split(event.position.y);
                                cx.stop_propagation();
                            }),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .overflow_hidden()
                    .when(self.toolbox_open, |pane| pane.flex_1())
                    .when(!self.toolbox_open, |pane| pane.flex_none())
                    .child(
                        PaneHeader::new(if read_only {
                            "Toolbox · read-only"
                        } else {
                            "Toolbox"
                        })
                        .toggle(
                            self.toolbox_open,
                            cx.listener(|panel, _, window, cx| {
                                panel.toolbox_open = !panel.toolbox_open;
                                if !panel.toolbox_open {
                                    window.focus(&panel.state.focus, cx);
                                }
                                cx.notify();
                            }),
                        )
                        .when(!read_only && native_tools, |header| {
                            let draft_owner = item.clone();
                            header.action(div().flex().items_center().gap_1().child(ui::IconButton::new("mde-visual-draft-mode", IconName::Pencil)
                                .aria_label("Visual draft mode")
                                .toggle_state(visual_draft_mode)
                                .tooltip(ui::Tooltip::text("Visual draft mode: place freely without semantic validation"))
                                .on_click(move |_, _, cx| { draft_owner.update(cx, |workbench, cx| {
                                    workbench.presentation.toggle_visual_draft_mode();
                                    workbench.feedback = if workbench.presentation.visual_draft_mode() { "Visual drafts: no semantic validation or code generation" } else { "Semantic modeling mode" }.into();
                                    cx.notify();
                                }); }))
                            .child(
                                ui::IconButton::new("mde-search-tools", IconName::MagnifyingGlass)
                                    .aria_label("Search all symbols")
                                    .aria_expanded(self.search_open && self.toolbox_open)
                                    .toggle_state(self.search_open)
                                    .tooltip(ui::Tooltip::text("Search all symbols"))
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.search_open = !panel.search_open;
                                        panel.toolbox_open = true;
                                        if panel.search_open {
                                            window.focus(&panel.search_input.focus_handle(cx), cx);
                                        } else {
                                            let editor =
                                                panel.search_input.read(cx).editor().clone();
                                            editor.clear(window, cx);
                                            window.focus(&panel.state.focus, cx);
                                        }
                                        cx.notify();
                                    })),
                            ))
                        }),
                    )
                    .when(self.toolbox_open && (!read_only || profile.is_some()), |pane| {
                        pane.when(self.search_open && native_tools && !read_only, |pane| {
                            pane.child(div().p_2().child(self.search_input.clone()))
                        })
                        .child(
                            div()
                                .id("bellatrix-mde-toolbox-scroll")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .px_2()
                                .pb_2()
                                .flex()
                                .flex_col()
                                .children(tool_buttons),
                        )
                        .when_some(chosen, |pane, tool| {
                            let owner = item.clone();
                            let label = catalog.label(tool).unwrap_or("Tool unavailable").to_owned();
                            let instruction = match tool {
                                ModelingTool::Relation(kind) => catalog.relation(kind)
                                    .map(|metadata| format!("{}: {}", metadata.label, metadata.endpoint_instruction))
                                    .unwrap_or_else(|| "Tool metadata unavailable".into()),
                                _ => format!("{label}: drag to canvas or click empty space"),
                            };
                            pane.child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .border_t_1()
                                    .border_color(palette.border)
                                    .p_2()
                                    .child(div().flex_1().min_w_0().overflow_hidden().child(Label::new(label)))
                                    .child(
                                        ui::IconButton::new("mde-tool-instructions", IconName::Info)
                                            .aria_label(instruction.clone())
                                            .tooltip(ui::Tooltip::text(instruction)),
                                    )
                                    .child(
                                        ui::IconButton::new("mde-cancel-toolbox-tool", IconName::Close)
                                            .aria_label("Cancel tool and return to selection")
                                            .tooltip(ui::Tooltip::text("Cancel tool and return to selection"))
                                            .on_click(move |_, _, cx| {
                                                owner.update(cx, |workbench, cx| workbench.cancel_tool(cx));
                                            }),
                                    ),
                            )
                        })
                        .when(native_tools && !read_only, |pane| pane.child(
                            div().flex_none().px_2().py_1().child(
                                    ui::IconButton::new(
                                        "bellatrix-open-symbol-library",
                                        IconName::Book,
                                    )
                                    .aria_label("Browse symbol library")
                                    .tooltip(ui::Tooltip::text("Browse symbol library"))
                                    .on_click(
                                        move |_, window, cx| {
                                            window.dispatch_action(Box::new(OpenMdeSymbols), cx);
                                        },
                                    ),
                                ),
                        ))
                    }),
            )
            .into_any_element()
    }
}

pub(crate) fn tool_matches_query(label: &str, query: &str) -> bool {
    label.to_lowercase().contains(&query.trim().to_lowercase())
}

fn tool_entry(
    id: String,
    tool: ModelingTool,
    label: String,
    chosen: Option<ModelingTool>,
    item: &Entity<MdeWorkbenchItem>,
    cx: &App,
) -> gpui::AnyElement {
    let palette = WorkbenchPalette::current(cx);
    let owner = item.clone();
    let entry = div()
        .id(id)
        .w_full()
        .h(px(30.0))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .rounded_sm()
        .hover(|style| style.bg(palette.surface))
        .when(chosen == Some(tool), |entry| {
            entry
                .bg(palette.surface)
                .border_l_2()
                .border_color(palette.selection)
        })
        .on_click(move |_, _, cx| {
            owner.update(cx, |workbench, cx| workbench.select_tool(tool, cx));
        })
        .child(
            crate::elements::uml::UmlVisualDefinition::for_tool(tool)
                .render_icon(20.0, palette.panel),
        )
        .child(Label::new(label.clone()));
    if !matches!(tool, ModelingTool::Relation(_)) {
        entry
            .cursor_grab()
            .on_drag(
                MdeToolDrag {
                    project_id: item.read(cx).project_id().clone(),
                    kind: tool,
                    label,
                },
                |drag: &MdeToolDrag, _, _, cx| cx.new(|_| drag.clone()),
            )
            .into_any_element()
    } else {
        entry.cursor_pointer().into_any_element()
    }
}
