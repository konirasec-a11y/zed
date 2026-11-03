use super::*;

fn tool_glyph(tool: ModelingTool) -> ShapeKind {
    match tool {
        ModelingTool::Element(kind) => match kind {
            ModelingElementKind::Port => ShapeKind::Port,
            ModelingElementKind::Part => ShapeKind::Class,
            ModelingElementKind::DeploymentSpecification => ShapeKind::Document,
            ModelingElementKind::CollaborationUse => ShapeKind::Collaboration,
            ModelingElementKind::DestructionOccurrenceSpecification => ShapeKind::Terminate,
            ModelingElementKind::Continuation => ShapeKind::State,

            ModelingElementKind::Choice => ShapeKind::Decision,
            ModelingElementKind::Junction => ShapeKind::Junction,
            ModelingElementKind::StateFork | ModelingElementKind::StateJoin => ShapeKind::Fork,
            ModelingElementKind::ShallowHistory => ShapeKind::History,
            ModelingElementKind::DeepHistory => ShapeKind::DeepHistory,
            ModelingElementKind::EntryPoint => ShapeKind::EntryPoint,
            ModelingElementKind::ExitPoint => ShapeKind::FlowFinal,
            ModelingElementKind::Terminate => ShapeKind::Terminate,
            ModelingElementKind::InputPin | ModelingElementKind::ValuePin => ShapeKind::InputPin,
            ModelingElementKind::OutputPin => ShapeKind::OutputPin,
            ModelingElementKind::InteractionOperand => ShapeKind::InteractionOperand,
            ModelingElementKind::Class
            | ModelingElementKind::Template
            | ModelingElementKind::AssociationClass => ShapeKind::Class,
            ModelingElementKind::Interface => ShapeKind::Interface,
            ModelingElementKind::Component => ShapeKind::Component,
            ModelingElementKind::Artifact => ShapeKind::Document,
            ModelingElementKind::Enumeration => ShapeKind::Enum,
            ModelingElementKind::Package
            | ModelingElementKind::Model
            | ModelingElementKind::Subsystem => ShapeKind::Package,
            ModelingElementKind::Node
            | ModelingElementKind::Device
            | ModelingElementKind::ExecutionEnvironment => ShapeKind::Deployment,
            ModelingElementKind::Collaboration => ShapeKind::Collaboration,
            ModelingElementKind::UseCase => ShapeKind::Usecase,
            ModelingElementKind::InstanceSpecification | ModelingElementKind::Stereotype => {
                ShapeKind::Class
            }
            ModelingElementKind::Profile => ShapeKind::Package,
            ModelingElementKind::Actor => ShapeKind::Actor,
            ModelingElementKind::Activity => ShapeKind::Activity,
            ModelingElementKind::StateMachine => ShapeKind::State,
            ModelingElementKind::Interaction | ModelingElementKind::InteractionUse => {
                ShapeKind::Boundary
            }
            ModelingElementKind::CombinedFragment => ShapeKind::CombinedFragment,
            ModelingElementKind::Region | ModelingElementKind::ActivityPartition => {
                ShapeKind::Partition
            }
            ModelingElementKind::OpaqueAction => ShapeKind::Activity,
            ModelingElementKind::SendSignalAction => ShapeKind::SendSignal,
            ModelingElementKind::AcceptEventAction => ShapeKind::AcceptEvent,
            ModelingElementKind::InitialNode | ModelingElementKind::Pseudostate => {
                ShapeKind::InitialNode
            }
            ModelingElementKind::ActivityFinalNode | ModelingElementKind::FinalState => {
                ShapeKind::FinalNode
            }
            ModelingElementKind::FlowFinalNode => ShapeKind::FlowFinal,
            ModelingElementKind::State | ModelingElementKind::StateInvariant => ShapeKind::State,
            ModelingElementKind::DecisionNode | ModelingElementKind::MergeNode => {
                ShapeKind::Decision
            }
            ModelingElementKind::ForkNode | ModelingElementKind::JoinNode => ShapeKind::Fork,
            ModelingElementKind::Lifeline => ShapeKind::Lifeline,
            ModelingElementKind::BehaviorExecutionSpecification => ShapeKind::Execution,
            ModelingElementKind::CentralBufferNode | ModelingElementKind::DataStoreNode => {
                ShapeKind::ObjectNode
            }
            ModelingElementKind::Comment => ShapeKind::Note,
            ModelingElementKind::Frame => ShapeKind::Boundary,
            ModelingElementKind::NaryAssociation => ShapeKind::Aggregate,
            ModelingElementKind::Signal
            | ModelingElementKind::DataType
            | ModelingElementKind::PrimitiveType => ShapeKind::Class,
        },
        ModelingTool::Relation(kind) => match kind {
            ModelingRelationKind::MessageSynchronous => ShapeKind::SynchronousMessage,
            ModelingRelationKind::MessageAsynchronous => ShapeKind::Message,
            ModelingRelationKind::MessageReply => ShapeKind::ReplyMessage,

            ModelingRelationKind::Extension => ShapeKind::Extension,
            ModelingRelationKind::Connector => ShapeKind::Association,
            ModelingRelationKind::NoteLink | ModelingRelationKind::ConstraintLink => {
                ShapeKind::DashedLink
            }
            ModelingRelationKind::ElementImport | ModelingRelationKind::ProfileApplication => {
                ShapeKind::Dependency
            }
            ModelingRelationKind::AssociationClassAttachment => {
                ShapeKind::AssociationClassAttachment
            }
            ModelingRelationKind::Usage
            | ModelingRelationKind::Abstraction
            | ModelingRelationKind::Include
            | ModelingRelationKind::Extend
            | ModelingRelationKind::PackageImport
            | ModelingRelationKind::PackageMerge
            | ModelingRelationKind::Deployment
            | ModelingRelationKind::Manifestation => ShapeKind::Dependency,
            ModelingRelationKind::ControlFlow
            | ModelingRelationKind::ObjectFlow
            | ModelingRelationKind::Transition => ShapeKind::Relation,
            ModelingRelationKind::Association
            | ModelingRelationKind::DirectedAssociation
            | ModelingRelationKind::Containment => ShapeKind::Association,
            ModelingRelationKind::Aggregation => ShapeKind::Aggregate,
            ModelingRelationKind::Composition => ShapeKind::Composition,
            ModelingRelationKind::Generalization | ModelingRelationKind::InterfaceRealization => {
                ShapeKind::Inheritance
            }
            ModelingRelationKind::Dependency
            | ModelingRelationKind::Realization
            | ModelingRelationKind::TemplateBinding => ShapeKind::Dependency,
        },
    }
}

pub struct MdeLeftPanel {
    pub(super) state: PanelState,
    diagrams_open: bool,
    toolbox_open: bool,
    search_open: bool,
    pub(super) search_input: Entity<InputField>,
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
                state.source().snapshot().ok(),
                state.visual_draft_mode,
            )
        };
        let mut diagrams = Vec::new();
        if let Some(snapshot) = &snapshot {
            for diagram in &snapshot.section.diagrams {
                let diagram_id = diagram.id.clone();
                let owner = item.clone();
                let diagram_kind = bellatrix_mde::StructuralDiagramKind::from_diagram(diagram)
                    .map(crate::diagram_kind_label)
                    .unwrap_or("Unsupported diagram");
                diagrams.push(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_2()
                        .when(active.as_deref() == Some(diagram_id.as_str()), |row| {
                            row.bg(palette.surface)
                        })
                        .child(
                            svg()
                                .data(glyph(ShapeKind::Class))
                                .size(px(14.0))
                                .text_color(palette.muted),
                        )
                        .child(
                            div().flex_1().min_w_0().child(
                                TreeViewItem::new(diagram_id.clone(), diagram.name.clone())
                                    .tooltip(ui::Tooltip::text(diagram_kind))
                                    .toggle_state(active.as_deref() == Some(diagram_id.as_str()))
                                    .on_click(move |_, window, cx| {
                                        if owner.read(cx).source().is_presentation_suspended() {
                                            return;
                                        }
                                        owner.update(cx, |workbench, cx| {
                                            workbench.select_diagram(diagram_id.clone(), cx)
                                        });
                                        if !owner.read(cx).source().is_read_only() {
                                            window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
                                        }
                                    }),
                            ),
                        ),
                );
            }
        }
        let create_item = item.clone();
        let create_menu = ui::ContextMenu::build(window, cx, move |menu, _, _| {
            bellatrix_mde::StructuralDiagramKind::ALL_DIAGRAMS
                .into_iter()
                .fold(menu, |menu, kind| {
                    let owner = create_item.clone();
                    let label = crate::diagram_kind_label(kind).to_owned();
                    menu.entry(label, None, move |window, cx| {
                        if owner.read(cx).source().is_read_only() {
                            return;
                        }
                        owner.update(cx, |workbench, cx| workbench.create_diagram(kind, cx));
                        window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
                    })
                })
        });
        let tool_groups: [(&str, Vec<ModelingTool>); 17] = [
            (
                "Classes · basic",
                ModelingElementKind::BASIC
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Relationships · basic",
                ModelingRelationKind::BASIC
                    .iter()
                    .copied()
                    .map(ModelingTool::Relation)
                    .collect(),
            ),
            (
                "Classes · advanced",
                ModelingElementKind::ADVANCED
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Relationships · advanced",
                ModelingRelationKind::ADVANCED
                    .iter()
                    .copied()
                    .map(ModelingTool::Relation)
                    .collect(),
            ),
            (
                "Packages",
                ModelingElementKind::PACKAGES
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .chain(
                        ModelingRelationKind::PACKAGES
                            .iter()
                            .copied()
                            .map(ModelingTool::Relation),
                    )
                    .collect(),
            ),
            (
                "Components & deployment",
                ModelingElementKind::STRUCTURAL
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Profiles",
                ModelingElementKind::PROFILES
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Behavior roots · notation",
                ModelingElementKind::BEHAVIOR
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Activity nodes",
                ModelingElementKind::ACTIVITY_NODES
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "States",
                ModelingElementKind::STATE_NODES
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Interactions",
                ModelingElementKind::INTERACTION_NODES
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Structural relationships",
                ModelingRelationKind::STRUCTURAL
                    .iter()
                    .copied()
                    .map(ModelingTool::Relation)
                    .collect(),
            ),
            (
                "Behavior relationships",
                ModelingRelationKind::BEHAVIOR
                    .iter()
                    .copied()
                    .map(ModelingTool::Relation)
                    .collect(),
            ),
            (
                "Ports, roles & nested elements",
                ModelingElementKind::DETAILS
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "State pseudostates",
                ModelingElementKind::PSEUDOSTATES
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
            (
                "Messages · visual",
                ModelingRelationKind::MESSAGES
                    .iter()
                    .copied()
                    .map(ModelingTool::Relation)
                    .collect(),
            ),
            (
                "Annotations",
                ModelingElementKind::ANNOTATIONS
                    .iter()
                    .copied()
                    .map(ModelingTool::Element)
                    .collect(),
            ),
        ];
        let query = self.search_input.read(cx).text(cx).trim().to_lowercase();
        let searching = !query.is_empty();
        let mut tool_buttons: Vec<gpui::AnyElement> = Vec::new();
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
                tool_buttons.push(tool_entry(
                    format!("mde-quick-tool-{index}"),
                    tool,
                    chosen,
                    &item,
                    cx,
                ));
            }
        }
        tool_buttons.push(
            div()
                .px_2()
                .pt_3()
                .pb_1()
                .text_xs()
                .text_color(palette.muted)
                .child(if searching {
                    "RESULTS"
                } else {
                    "SYMBOL LIBRARY"
                })
                .into_any_element(),
        );
        let mut matching_groups = 0;
        for (group_index, (group_name, tools)) in tool_groups.into_iter().enumerate() {
            let tools: Vec<_> = tools
                .into_iter()
                .filter(|tool| tool_matches_query(tool.label(), &query))
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
                        chosen,
                        &item,
                        cx,
                    ));
                }
            }
        }
        if matching_groups == 0 {
            tool_buttons.push(
                div()
                    .p_2()
                    .text_xs()
                    .text_color(palette.muted)
                    .child("No symbols found.")
                    .into_any_element(),
            );
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
                            .when(!read_only, |header| {
                                header.action(ui::DropdownMenu::new(
                                    "mde-new-diagram",
                                    "New",
                                    create_menu,
                                ))
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
                                        .child(ui::Icon::new(IconName::Folder))
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
                        .when(!read_only, |header| {
                            let draft_owner = item.clone();
                            header.action(ui::IconButton::new("mde-visual-draft-mode", IconName::Pencil)
                                .toggle_state(visual_draft_mode)
                                .tooltip(ui::Tooltip::text("Visual draft mode: place freely without semantic validation"))
                                .on_click(move |_, _, cx| { draft_owner.update(cx, |workbench, cx| {
                                    workbench.visual_draft_mode = !workbench.visual_draft_mode;
                                    workbench.pending_relation_source = None;
                                    workbench.feedback = if workbench.visual_draft_mode { "Visual drafts: no semantic validation or code generation" } else { "Semantic modeling mode" }.into();
                                    cx.notify();
                                }); }))
                            .action(
                                ui::IconButton::new("mde-search-tools", IconName::MagnifyingGlass)
                                    .toggle_state(self.search_open)
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
                            )
                        }),
                    )
                    .when(self.toolbox_open && !read_only, |pane| {
                        pane.when(self.search_open, |pane| {
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
                                .children(tool_buttons)
                                .when_some(chosen, |toolbox, tool| {
                                    toolbox.child(
                                        div()
                                            .p_2()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .whitespace_normal()
                                            .child(match tool {
                                                ModelingTool::Element(kind) => format!(
                                                    "{}: drag to canvas or click empty space",
                                                    kind.label()
                                                ),
                                                ModelingTool::Relation(kind) => format!(
                                                    "{}: {}",
                                                    kind.label(),
                                                    kind.endpoint_instruction()
                                                ),
                                            }),
                                    )
                                })
                                .when(chosen.is_some(), |toolbox| {
                                    let owner = item.clone();
                                    toolbox.child(
                                        Button::new("mde-cancel-toolbox-tool", "Selection tool")
                                            .on_click(move |_, _, cx| {
                                                owner.update(cx, |workbench, cx| {
                                                    workbench.cancel_tool(cx)
                                                });
                                            }),
                                    )
                                })
                                .child(
                                    Button::new(
                                        "bellatrix-open-symbol-library",
                                        "Browse symbol library",
                                    )
                                    .on_click(
                                        move |_, window, cx| {
                                            window.dispatch_action(Box::new(OpenMdeSymbols), cx);
                                        },
                                    ),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }
}

pub(super) fn tool_matches_query(label: &str, query: &str) -> bool {
    label.to_lowercase().contains(&query.trim().to_lowercase())
}

fn tool_entry(
    id: String,
    tool: ModelingTool,
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
            svg()
                .data(glyph(tool_glyph(tool)))
                .size(px(20.0))
                .text_color(if chosen == Some(tool) {
                    palette.selection
                } else {
                    palette.muted
                }),
        )
        .child(Label::new(tool.label()));
    if matches!(tool, ModelingTool::Element(_)) {
        entry
            .cursor_grab()
            .on_drag(
                MdeToolDrag {
                    project_id: item.read(cx).project_id().clone(),
                    kind: tool,
                },
                |drag: &MdeToolDrag, _, _, cx| cx.new(|_| drag.clone()),
            )
            .into_any_element()
    } else {
        entry.cursor_pointer().into_any_element()
    }
}
