//! Panels owned by the Zed Workspace Dock, consuming the single MDE domain.
use crate::{
    MdeToolDrag, MdeWorkbenchItem, WorkbenchPalette,
    hierarchy::project_hierarchy,
    integration::{OpenMdeSymbols, OpenMdeWorkbench},
    property_editor::render_inspector,
};
use bellatrix_mde::commands::{ModelingElementKind, ModelingTool};
use bellatrix_mde::{
    ModelingRelationKind, PropertyEditor, RelationshipGraph, inspect_element,
    relationship_presentation,
};
use bellatrix_ui_panels::PaneHeader;
use bellatrix_ui_symbols::{ShapeKind, glyph};
use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Render,
    StatefulInteractiveElement, Subscription, Window, actions, div, prelude::*, px, svg,
};
use std::collections::{HashMap, HashSet};
use ui::{Button, IconName, Label, TreeViewItem, prelude::*};
use ui_input::{ErasedEditorEvent, InputField};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

actions!(
    bellatrix_mde,
    [
        ToggleMdeLeft,
        ToggleMdeRight,
        ToggleMdeBottom,
        ShowMdeDiagnostics
    ]
);

// Dock priorities must be distinct even when panels occupy different sides.
const MDE_LEFT_PRIORITY: u32 = 10_000;
const MDE_RIGHT_PRIORITY: u32 = 10_001;
const MDE_BOTTOM_PRIORITY: u32 = 10_002;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toolbox_search_matches_labels_case_insensitively() {
        assert!(tool_matches_query("Association Class", " class "));
        assert!(tool_matches_query("Interface", "INTER"));
        assert!(tool_matches_query("Package", ""));
        assert!(tool_matches_query("Package", "   "));
        assert!(!tool_matches_query("Package", "class"));
    }

    #[test]
    fn explorer_glyphs_preserve_semantic_types() {
        assert_eq!(element_glyph("rust.module"), ShapeKind::Package);
        assert_eq!(element_glyph("uml.interface"), ShapeKind::Interface);
        assert_eq!(element_glyph("rust.enum"), ShapeKind::Enum);
        assert_eq!(element_glyph("rust.struct"), ShapeKind::Class);
        assert_eq!(element_glyph("uml.constraint"), ShapeKind::Note);
        assert_eq!(element_glyph("uml.port"), ShapeKind::Connector);
        assert_eq!(element_glyph("uml.frame"), ShapeKind::Boundary);
        assert_eq!(element_glyph("custom.widget"), ShapeKind::Topic);
        assert_ne!(element_glyph("uml.reception"), ShapeKind::Class);
    }

    #[test]
    fn bellatrix_panel_priorities_do_not_collide_with_zed_or_each_other() {
        let priorities = [MDE_LEFT_PRIORITY, MDE_RIGHT_PRIORITY, MDE_BOTTOM_PRIORITY];
        assert!(priorities.iter().all(|priority| *priority > 7));
        assert_eq!(
            priorities
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            3
        );
    }
}

struct PanelState {
    item: Entity<MdeWorkbenchItem>,
    focus: FocusHandle,
    position: DockPosition,
    top_height: f32,
    split_start: Option<(Pixels, f32)>,
    _listener: Subscription,
}
impl PanelState {
    fn new<T: 'static>(
        item: Entity<MdeWorkbenchItem>,
        position: DockPosition,
        top_height: f32,
        cx: &mut Context<T>,
    ) -> Self {
        let listener = cx.observe(&item, |_, _, cx| cx.notify());
        Self {
            item,
            focus: cx.focus_handle(),
            position,
            top_height,
            split_start: None,
            _listener: listener,
        }
    }
    fn start_split(&mut self, y: Pixels) {
        self.split_start = Some((y, self.top_height));
    }
    fn move_split(&mut self, y: Pixels, dragging: bool) -> bool {
        let Some((start, original)) = self.split_start else {
            return false;
        };
        if !dragging {
            return false;
        }
        let next = (original + f32::from(y - start)).clamp(90.0, 650.0);
        if (self.top_height - next).abs() < 0.5 {
            return false;
        }
        self.top_height = next;
        true
    }
    fn finish_split(&mut self) {
        self.split_start = None;
    }
}
fn tool_glyph(tool: ModelingTool) -> ShapeKind {
    match tool {
        ModelingTool::Element(kind) => match kind {
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
            ModelingElementKind::Comment => ShapeKind::Note,
            ModelingElementKind::Frame => ShapeKind::Boundary,
            ModelingElementKind::NaryAssociation => ShapeKind::Aggregate,
            ModelingElementKind::Signal
            | ModelingElementKind::DataType
            | ModelingElementKind::PrimitiveType => ShapeKind::Class,
        },
        ModelingTool::Relation(kind) => match kind {
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
    state: PanelState,
    diagrams_open: bool,
    toolbox_open: bool,
    search_open: bool,
    search_input: Entity<InputField>,
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
        let palette = WorkbenchPalette::current(cx);
        let item = self.state.item.clone();
        let state = item.read(cx);
        let active = state.active_diagram().map(str::to_owned);
        let chosen = state.selected_tool();
        let snapshot = state.source().snapshot().ok();
        let mut diagrams = Vec::new();
        if let Some(snapshot) = &snapshot {
            for diagram in &snapshot.section.diagrams {
                let diagram_id = diagram.id.clone();
                let owner = item.clone();
                let diagram_kind =
                    bellatrix_mde::StructuralDiagramKind::from_uml_di_type(&diagram.uml_di_type)
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
                                        owner.update(cx, |workbench, cx| {
                                            workbench.select_diagram(diagram_id.clone(), cx)
                                        });
                                        window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
                                    }),
                            ),
                        ),
                );
            }
        }
        let create_item = item.clone();
        let create_menu = ui::ContextMenu::build(window, cx, move |menu, _, _| {
            bellatrix_mde::StructuralDiagramKind::ALL
                .into_iter()
                .fold(menu, |menu, kind| {
                    let owner = create_item.clone();
                    let label = if kind == bellatrix_mde::StructuralDiagramKind::CompositeStructure
                    {
                        "Composite structure (selected classifier)".to_owned()
                    } else {
                        crate::diagram_kind_label(kind).to_owned()
                    };
                    menu.entry(label, None, move |window, cx| {
                        owner.update(cx, |workbench, cx| workbench.create_diagram(kind, cx));
                        window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
                    })
                })
        });
        let tool_groups: [(&str, Vec<ModelingTool>); 7] = [
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
                            .action(ui::DropdownMenu::new("mde-new-diagram", "New", create_menu)),
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
                        PaneHeader::new("Toolbox")
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
                            ),
                    )
                    .when(self.toolbox_open, |pane| {
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
    }
}

fn tool_matches_query(label: &str, query: &str) -> bool {
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

pub struct MdeRightPanel {
    state: PanelState,
    editing_id: Option<String>,
    property_inputs: HashMap<String, Entity<InputField>>,
    property_drafts: HashMap<String, HashMap<String, Entity<InputField>>>,
    style_target_id: Option<(String, String)>,
    pub(crate) pending_theme_resets:
        HashMap<(String, String), crate::style_editor::PendingThemeReset>,
    pub(crate) style_listeners: HashMap<(String, String), Vec<Subscription>>,
    pub(crate) style_revisions: HashMap<(String, String), u64>,
    pub(crate) style_baselines: HashMap<(String, String), bellatrix_mde::Style>,
    pub(crate) style_inputs: HashMap<(String, String), HashMap<String, Entity<InputField>>>,
    pub(crate) inspector_styles_open: bool,
    collapsed_elements: HashSet<String>,
    explorer_open: bool,
    inspector_open: bool,
    pub(crate) inspector_documentation_open: bool,
    pub(crate) inspector_advanced_open: bool,
}
impl MdeRightPanel {
    pub fn new(
        item: Entity<MdeWorkbenchItem>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            state: PanelState::new(item, DockPosition::Right, 230.0, cx),
            editing_id: None,
            property_inputs: HashMap::new(),
            property_drafts: HashMap::new(),
            style_baselines: HashMap::new(),
            style_inputs: HashMap::new(),
            style_revisions: HashMap::new(),
            style_target_id: None,
            style_listeners: HashMap::new(),
            pending_theme_resets: HashMap::new(),
            inspector_styles_open: true,
            collapsed_elements: HashSet::new(),
            explorer_open: true,
            inspector_open: true,
            inspector_documentation_open: true,
            inspector_advanced_open: false,
        }
    }
}
impl EventEmitter<PanelEvent> for MdeRightPanel {}
impl Focusable for MdeRightPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.state.focus.clone()
    }
}
impl Panel for MdeRightPanel {
    fn persistent_name() -> &'static str {
        "MDE Model Explorer & Inspector"
    }
    fn panel_key() -> &'static str {
        "bellatrix.mde.right.v1"
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
        px(292.0)
    }
    fn min_size(&self, _: &Window, _: &App) -> Option<Pixels> {
        Some(px(230.0))
    }
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
        Some(IconName::BellatrixMdeInspector)
    }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Bellatrix MDE - Model Explorer & Inspector (right dock)")
    }
    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleMdeRight)
    }
    fn starts_open(&self, _: &Window, _: &App) -> bool {
        true
    }
    fn activation_priority(&self) -> u32 {
        MDE_RIGHT_PRIORITY
    }
}
impl Render for MdeRightPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let panel_background = palette.panel;
        let panel_border = palette.border;
        let item = self.state.item.clone();
        let selected_id = item.read(cx).selected_element().map(str::to_owned);
        let active_diagram = item.read(cx).active_diagram().map(str::to_owned);
        let snapshot = item.read(cx).source().snapshot().ok();
        let inspection = snapshot.as_ref().and_then(|snapshot| {
            selected_id
                .as_deref()
                .and_then(|id| inspect_element(&snapshot.section, id))
        });
        let mut rows = Vec::new();
        if let Some(snapshot) = &snapshot {
            let tree = project_hierarchy(&snapshot.section);
            let mut hidden_by_depth: Option<usize> = None;
            for (index, row) in tree.iter().enumerate() {
                if let Some(ancestor_depth) = hidden_by_depth {
                    if row.depth > ancestor_depth {
                        continue;
                    }
                    hidden_by_depth = None;
                }
                let has_children = tree
                    .get(index + 1)
                    .is_some_and(|next| next.depth > row.depth);
                let expanded = !self.collapsed_elements.contains(&row.element_id);
                let id = row.element_id.clone();
                let owner = item.clone();
                let disclosure_id = row.element_id.clone();
                let disclosure_panel = cx.weak_entity();
                let menu_owner = item.clone();
                let menu_subject = id.clone();
                let label = snapshot
                    .section
                    .elements
                    .iter()
                    .find(|element| element.id == row.element_id)
                    .map(|element| {
                        bellatrix_mde::template_parametered_element(&snapshot.section, element)
                            .map(bellatrix_mde::element_display_label)
                            .unwrap_or_else(|| bellatrix_mde::element_display_label(element))
                    })
                    .unwrap_or_else(|| row.label.clone());
                let tree_item = TreeViewItem::new(id.clone(), label)
                    .root_item(has_children)
                    .expanded(expanded)
                    .toggle_state(selected_id.as_deref() == Some(id.as_str()))
                    .on_toggle(move |_, _, cx| {
                        if let Err(error) = disclosure_panel.update(cx, |panel, cx| {
                            if !panel.collapsed_elements.insert(disclosure_id.clone()) {
                                panel.collapsed_elements.remove(&disclosure_id);
                            }
                            cx.notify();
                        }) {
                            eprintln!("MDE Explorer expansion failed: {error}");
                        }
                    })
                    .on_click(move |_, _, cx| {
                        owner.update(cx, |workbench, cx| workbench.select_element(id.clone(), cx));
                    });
                rows.push(
                    div()
                        .id(format!("mde-explorer-row-{}", row.element_id))
                        .pl(px((row.depth.min(15) + 1) as f32 * 14.0))
                        .flex()
                        .items_center()
                        .gap_1()
                        .when(
                            selected_id.as_deref() == Some(row.element_id.as_str()),
                            |row| row.bg(palette.surface),
                        )
                        .child(
                            svg()
                                .data(glyph(element_glyph(&row.kind)))
                                .size(px(14.0))
                                .flex_none()
                                .text_color(
                                    if selected_id.as_deref() == Some(row.element_id.as_str()) {
                                        palette.selection
                                    } else {
                                        palette.muted
                                    },
                                ),
                        )
                        .child(
                            div().flex_1().min_w_0().overflow_hidden().child(
                                ui::right_click_menu(format!(
                                    "mde-explorer-menu-{}",
                                    row.element_id
                                ))
                                .trigger(move |_, _, _| tree_item)
                                .menu(move |window, cx| {
                                    let show_owner = menu_owner.clone();
                                    let show_subject = menu_subject.clone();
                                    let delete_owner = menu_owner.clone();
                                    let delete_subject = menu_subject.clone();
                                    ui::ContextMenu::build(window, cx, move |menu, _, _| {
                                        menu.entry(
                                            "Show in active diagram",
                                            None,
                                            move |window, cx| {
                                                show_owner.update(cx, |item, cx| {
                                                    item.show_element_in_active_diagram(
                                                        show_subject.clone(),
                                                        window,
                                                        cx,
                                                    )
                                                });
                                                window.dispatch_action(
                                                    Box::new(OpenMdeWorkbench),
                                                    cx,
                                                );
                                            },
                                        )
                                        .entry(
                                            "Delete from model",
                                            None,
                                            move |window, cx| {
                                                delete_owner.update(cx, |item, cx| {
                                                    item.select_element(delete_subject.clone(), cx);
                                                    item.delete_selected_from_model(cx);
                                                });
                                                window.dispatch_action(
                                                    Box::new(OpenMdeWorkbench),
                                                    cx,
                                                );
                                            },
                                        )
                                    })
                                }),
                            ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .px_1()
                                .rounded_sm()
                                .bg(palette.surface)
                                .child(
                                    Label::new(
                                        bellatrix_mde::semantic_identity(&row.kind)
                                            .type_label
                                            .to_owned(),
                                    )
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                                ),
                        ),
                );
                if has_children && !expanded {
                    hidden_by_depth = Some(row.depth);
                }
            }
            let relations = active_diagram
                .as_deref()
                .or_else(|| {
                    snapshot
                        .section
                        .diagrams
                        .first()
                        .map(|diagram| diagram.id.as_str())
                })
                .map(|id| RelationshipGraph::new(&snapshot.section, id).visible_relations())
                .unwrap_or_default();
            if !relations.is_empty() {
                rows.push(
                    div()
                        .id("mde-relationships-heading")
                        .p_2()
                        .text_size(px(11.0))
                        .text_color(palette.muted)
                        .child("Relationships"),
                );
            }
            for relation in relations {
                let presentation = relationship_presentation(relation);
                let from = snapshot
                    .section
                    .elements
                    .iter()
                    .find(|element| element.id == relation.from)
                    .map_or(relation.from.as_str(), |element| element.name.as_str());
                let to = snapshot
                    .section
                    .elements
                    .iter()
                    .find(|element| element.id == relation.to)
                    .map_or(relation.to.as_str(), |element| element.name.as_str());
                let id = relation.id.clone();
                let owner = item.clone();
                rows.push(
                    div()
                        .id(format!("mde-explorer-relation-{id}"))
                        .pl(px(14.0))
                        .child(
                            TreeViewItem::new(id.clone(), presentation.describe(from, to))
                                .toggle_state(selected_id.as_deref() == Some(id.as_str()))
                                .on_click(move |_, _, cx| {
                                    owner.update(cx, |workbench, cx| {
                                        workbench.select_element(id.clone(), cx)
                                    });
                                }),
                        ),
                );
            }
        }
        // Preserve in-progress editing when a model revision triggers a render.
        if self.editing_id != selected_id {
            if self
                .property_inputs
                .values()
                .any(|input| input.read(cx).focus_handle(cx).contains_focused(window, cx))
            {
                window.focus(&self.state.focus, cx);
            }
            if let Some(previous) = self.editing_id.take() {
                self.property_drafts
                    .insert(previous, std::mem::take(&mut self.property_inputs));
            }
            self.editing_id = selected_id.clone();
            self.property_inputs = selected_id
                .as_ref()
                .and_then(|id| self.property_drafts.remove(id))
                .unwrap_or_default();
            if let Some(inspection) = &inspection {
                for property in &inspection.properties {
                    if self.property_inputs.contains_key(&property.path)
                        || !property.editable
                        || !matches!(
                            property.editor,
                            PropertyEditor::Text
                                | PropertyEditor::Multiline
                                | PropertyEditor::Number
                        )
                    {
                        continue;
                    }
                    let editor = cx.new(|cx| {
                        InputField::new(window, cx, &property.label).label_min_width(px(0.0))
                    });
                    let text = property
                        .value
                        .as_str()
                        .map_or_else(|| property.value.to_string(), str::to_owned);
                    editor.update(cx, |field, cx| {
                        if property.editor == PropertyEditor::Multiline {
                            field.editor().set_multiline(Some(12), window, cx);
                        }
                        field.set_text(&text, window, cx);
                    });
                    self.property_inputs.insert(property.path.clone(), editor);
                }
            }
        }
        let selected_view = item.read(cx).selected_view.clone();
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
            let is_edge = snapshot
                .section
                .relations
                .iter()
                .any(|relation| view.subject.as_deref() == Some(relation.id.as_str()));
            Some((diagram.id.as_str(), view, is_edge))
        });
        let style_target_id =
            style_target.map(|(diagram_id, view, _)| (diagram_id.to_owned(), view.id.clone()));
        if self.style_target_id != style_target_id {
            if self
                .style_target_id
                .as_ref()
                .and_then(|key| self.style_inputs.get(key))
                .is_some_and(|inputs| {
                    inputs
                        .values()
                        .any(|input| input.read(cx).focus_handle(cx).contains_focused(window, cx))
                })
            {
                window.focus(&self.state.focus, cx);
            }
            self.style_target_id = style_target_id;
        }
        let mut inspector_rows = if let Some((diagram_id, view, _)) = style_target {
            let key = (diagram_id.to_owned(), view.id.clone());
            if !self.style_inputs.contains_key(&key) {
                let inputs = crate::style_editor::inputs_for_style(&view.style, window, cx);
                let listeners = inputs
                    .values()
                    .map(|input| {
                        let owner = cx.weak_entity();
                        input.read(cx).editor().clone().subscribe(
                            Box::new(move |event, _, cx| {
                                if event == ErasedEditorEvent::BufferEdited {
                                    let owner = owner.clone();
                                    cx.defer(move |cx| {
                                        if let Err(error) = owner.update(cx, |_, cx| cx.notify()) {
                                            eprintln!("MDE style input refresh failed: {error}");
                                        }
                                    });
                                }
                            }),
                            window,
                            cx,
                        )
                    })
                    .collect();
                self.style_listeners.insert(key.clone(), listeners);
                self.style_inputs.insert(key.clone(), inputs);
            }
            if let (Some(pending), Some(inputs), Some(snapshot)) = (
                self.pending_theme_resets.get(&key),
                self.style_inputs.get(&key),
                snapshot.as_ref(),
            ) {
                let acknowledgment = crate::style_editor::acknowledge_theme_reset(
                    pending,
                    inputs,
                    snapshot.revision,
                    &view.style,
                    window,
                    cx,
                );
                if acknowledgment == crate::style_editor::ResetAcknowledgment::Applied {
                    self.style_baselines.insert(key.clone(), view.style.clone());
                    self.style_revisions.insert(key.clone(), snapshot.revision);
                }
                if acknowledgment != crate::style_editor::ResetAcknowledgment::Pending {
                    self.pending_theme_resets.remove(&key);
                }
            }
            let dirty = self.style_baselines.get(&key).is_some_and(|previous| {
                self.style_inputs.get(&key).is_some_and(|inputs| {
                    crate::style_editor::style_inputs_dirty(inputs, previous, cx)
                        && crate::style_editor::style_inputs_dirty(inputs, &view.style, cx)
                })
            });
            if !dirty && !self.pending_theme_resets.contains_key(&key) {
                if let (Some(previous), Some(inputs)) =
                    (self.style_baselines.get(&key), self.style_inputs.get(&key))
                {
                    crate::style_editor::sync_style_inputs(
                        inputs,
                        previous,
                        &view.style,
                        window,
                        cx,
                    );
                }
                self.style_baselines.insert(key.clone(), view.style.clone());
                self.style_revisions.insert(
                    key.clone(),
                    snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
                );
            }
            let revision = self.style_revisions.get(&key).copied().unwrap_or(0);
            let conflict = snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.revision != revision);
            crate::style_editor::render_styles(
                style_target,
                item.clone(),
                self.style_inputs.get(&key).unwrap_or(&HashMap::new()),
                revision,
                conflict,
                self.pending_theme_resets.contains_key(&key),
                self.inspector_styles_open,
                window,
                cx,
            )
        } else {
            crate::style_editor::render_styles(
                None,
                item.clone(),
                &HashMap::new(),
                0,
                false,
                false,
                self.inspector_styles_open,
                window,
                cx,
            )
        };
        inspector_rows.extend(render_inspector(
            inspection.as_ref(),
            item.clone(),
            &self.property_inputs,
            self.inspector_documentation_open,
            self.inspector_advanced_open,
            window,
            cx,
        ));
        let delete_item = item.clone();
        let project_name = snapshot.as_ref().map_or_else(
            || "Project unavailable".to_owned(),
            |snapshot| snapshot.name.clone(),
        );
        div()
            .id("mde-right-panel")
            .track_focus(&self.state.focus)
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(panel_background)
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
                    .h(px(if self.explorer_open {
                        self.state.top_height
                    } else {
                        36.0
                    }))
                    .when(self.explorer_open && !self.inspector_open, |pane| {
                        pane.flex_1()
                    })
                    .child(
                        PaneHeader::new("Model explorer")
                            .toggle(
                                self.explorer_open,
                                cx.listener(|panel, _, _, cx| {
                                    panel.explorer_open = !panel.explorer_open;
                                    cx.notify();
                                }),
                            )
                            .action(Button::new("mde-save-project", "Save As…").on_click(
                                move |_, window, cx| {
                                    window.dispatch_action(
                                        Box::new(crate::integration::SaveMdeProjectAs),
                                        cx,
                                    );
                                },
                            )),
                    )
                    .when(self.explorer_open, |pane| {
                        pane.child(
                            div()
                                .id("bellatrix-mde-explorer-scroll")
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
                                .children(rows)
                                .when(
                                    snapshot.as_ref().is_some_and(|snapshot| {
                                        project_hierarchy(&snapshot.section).is_empty()
                                    }),
                                    |pane| {
                                        pane.child(
                                            div()
                                                .p_3()
                                                .text_xs()
                                                .text_color(palette.muted)
                                                .child("No model elements yet."),
                                        )
                                    },
                                ),
                        )
                    }),
            )
            .when(self.explorer_open && self.inspector_open, |panel| {
                panel.child(
                    div()
                        .id("mde-right-stack-resizer")
                        .h(px(4.0))
                        .w_full()
                        .flex_none()
                        .cursor_ns_resize()
                        .border_b_1()
                        .border_color(panel_border)
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
                    .when(self.inspector_open, |pane| pane.flex_1())
                    .when(!self.inspector_open, |pane| pane.flex_none())
                    .child(PaneHeader::new("Properties").toggle(
                        self.inspector_open,
                        cx.listener(|panel, _, window, cx| {
                            panel.inspector_open = !panel.inspector_open;
                            if !panel.inspector_open {
                                window.focus(&panel.state.focus, cx);
                            }
                            cx.notify();
                        }),
                    ))
                    .when(self.inspector_open, |pane| {
                        pane.child(
                            div()
                                .id("bellatrix-mde-inspector-scroll")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .p_2()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .children(inspector_rows)
                                .when(selected_id.is_some(), |pane| {
                                    pane.child(
                                        div()
                                            .mt_2()
                                            .pt_2()
                                            .border_t_1()
                                            .border_color(panel_border)
                                            .child(
                                                Button::new(
                                                    "mde-delete-element",
                                                    "Delete from model",
                                                )
                                                .on_click(move |_, _, cx| {
                                                    delete_item.update(cx, |workbench, cx| {
                                                        workbench.delete_selected_from_model(cx)
                                                    });
                                                }),
                                            ),
                                    )
                                }),
                        )
                    }),
            )
    }
}

pub(crate) fn element_glyph(kind: &str) -> ShapeKind {
    let identity = bellatrix_mde::semantic_identity(kind);
    if identity.type_label == "Frame" {
        return ShapeKind::Boundary;
    }
    let label = if kind == "rust.module" {
        "Package"
    } else {
        identity.type_label
    };
    bellatrix_ui_symbols::groups(bellatrix_ui_symbols::Family::Uml)
        .iter()
        .flat_map(|group| group.items)
        .find(|symbol| symbol.label.eq_ignore_ascii_case(label))
        .map(|symbol| symbol.kind)
        .unwrap_or(ShapeKind::Topic)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Diagnostics,
    Relationships,
    Documentation,
    Thumbnails,
}
pub struct MdeBottomPanel {
    state: PanelState,
    active: BottomTab,
}
impl MdeBottomPanel {
    pub fn new(item: Entity<MdeWorkbenchItem>, cx: &mut Context<Self>) -> Self {
        Self {
            state: PanelState::new(item, DockPosition::Bottom, 0.0, cx),
            active: BottomTab::Relationships,
        }
    }
}
impl EventEmitter<PanelEvent> for MdeBottomPanel {}
impl Focusable for MdeBottomPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.state.focus.clone()
    }
}
impl Panel for MdeBottomPanel {
    fn persistent_name() -> &'static str {
        "MDE Auxiliary Dock"
    }
    fn panel_key() -> &'static str {
        "bellatrix.mde.bottom.v1"
    }
    fn position(&self, _: &Window, _: &App) -> DockPosition {
        self.state.position
    }
    fn position_is_valid(&self, position: DockPosition) -> bool {
        position == DockPosition::Bottom
    }
    fn set_position(&mut self, position: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        if self.position_is_valid(position) {
            self.state.position = position;
            cx.notify();
        }
    }
    fn default_size(&self, _: &Window, _: &App) -> Pixels {
        px(190.0)
    }
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
        Some(IconName::BellatrixMdeDetails)
    }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Bellatrix MDE - Details: Diagnostics, Relations, Markdown & Thumbnails (bottom dock)")
    }
    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleMdeBottom)
    }
    fn starts_open(&self, _: &Window, _: &App) -> bool {
        false
    }
    fn activation_priority(&self) -> u32 {
        MDE_BOTTOM_PRIORITY
    }
}
impl Render for MdeBottomPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let active = self.active;
        let handle = cx.weak_entity();
        let mut tabs = Vec::new();
        for (index, (kind, label)) in [
            (BottomTab::Diagnostics, "Diagnostics"),
            (BottomTab::Relationships, "Relationships"),
            (BottomTab::Documentation, "Markdown"),
            (BottomTab::Thumbnails, "Thumbnails"),
        ]
        .into_iter()
        .enumerate()
        {
            let owner = handle.clone();
            tabs.push(
                Button::new(format!("mde-bottom-tab-{index}"), label).on_click(move |_, _, cx| {
                    let _ = owner.update(cx, |panel, cx| {
                        panel.active = kind;
                        cx.notify();
                    });
                }),
            );
        }
        let description = match active {
            BottomTab::Diagnostics => {
                "Read-only implemented UML checks using the stored diagram type. This is not full conformance certification."
            }
            BottomTab::Relationships => "Select a relation to inspect its endpoints",
            BottomTab::Documentation => "Documentation is part of the MDE model",
            BottomTab::Thumbnails => "Diagrams are listed in Working diagrams",
        };
        div()
            .id("mde-bottom-panel")
            .track_focus(&self.state.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.panel)
            .child(div().h(px(36.0)).flex().gap_2().children(tabs))
            .child(
                div()
                    .id("mde-bottom-output")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_3()
                    .child(div().whitespace_normal().child(description))
                    .when(active == BottomTab::Diagnostics, |output| {
                        let feedback = self.state.item.read(cx).feedback();
                        output.children(
                            feedback
                                .lines()
                                .map(|line| {
                                    div().w_full().whitespace_normal().child(line.to_owned())
                                })
                                .collect::<Vec<_>>(),
                        )
                    }),
            )
    }
}

pub fn attach_panels(
    workspace: &mut Workspace,
    item: Entity<MdeWorkbenchItem>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    workspace.register_action(|workspace, _: &ToggleMdeLeft, window, cx| {
        workspace.toggle_panel_focus::<MdeLeftPanel>(window, cx);
    });
    workspace.register_action(|workspace, _: &ToggleMdeRight, window, cx| {
        workspace.toggle_panel_focus::<MdeRightPanel>(window, cx);
    });
    workspace.register_action(|workspace, _: &ToggleMdeBottom, window, cx| {
        workspace.toggle_panel_focus::<MdeBottomPanel>(window, cx);
    });
    workspace.register_action(|workspace, _: &ShowMdeDiagnostics, window, cx| {
        if let Some(panel) = workspace.panel::<MdeBottomPanel>(cx) {
            panel.update(cx, |panel, cx| {
                panel.active = BottomTab::Diagnostics;
                cx.notify();
            });
        }
        workspace.reveal_panel::<MdeBottomPanel>(window, cx);
    });
    let left = cx.new(|cx| MdeLeftPanel::new(item.clone(), window, cx));
    let right = cx.new(|cx| MdeRightPanel::new(item.clone(), window, cx));
    let bottom = cx.new(|cx| MdeBottomPanel::new(item, cx));
    workspace.add_panel(left, window, cx);
    workspace.add_panel(right, window, cx);
    workspace.add_panel(bottom, window, cx);
}
