//! Panels owned by the Zed Workspace Dock.
//!
//! Zed's Dock shows one active panel per side. The two Bellatrix side Panels
//! compose the Working Diagrams/Toolbox and Explorer/Inspector stacks from
//! docs/design/ while Zed still owns placement, focus and persistence.

use crate::{MdeToolDrag, MdeWorkbenchItem, WorkbenchPalette, hierarchy::project_hierarchy, integration::{OpenMdeWorkbench, OpenMdeSymbols}, property_editor::render_inspector};
use bellatrix_mde_cordis::{ModelingElementKind, ModelingRelationKind, ModelingTool};
use bellatrix_mde_modeling::{inspect_element, PropertyEditor, RelationshipGraph, relationship_presentation};
use bellatrix_ui_symbols::{glyph, ShapeKind};
use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    InteractiveElement, StatefulInteractiveElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Render, Subscription, Window, actions, div, svg, prelude::*, px,
};
use ui::{Button, IconName, Label, TreeViewItem, prelude::*};
use ui_input::InputField;
use std::collections::{HashMap, HashSet};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

actions!(bellatrix_mde, [ToggleMdeLeft, ToggleMdeRight, ToggleMdeBottom]);

// Zed's built-in panels currently occupy priorities 0..=7. Dock::add_panel
// panics on duplicate priorities, even when the panels are on different sides.
// Reserve a stable, separate range for all Bellatrix MDE panels.
const MDE_LEFT_PRIORITY: u32 = 10_000;
const MDE_RIGHT_PRIORITY: u32 = 10_001;
const MDE_BOTTOM_PRIORITY: u32 = 10_002;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bellatrix_panel_priorities_do_not_collide_with_zed_or_each_other() {
        let priorities = [MDE_LEFT_PRIORITY, MDE_RIGHT_PRIORITY, MDE_BOTTOM_PRIORITY];
        assert!(priorities.iter().all(|priority| *priority > 7));
        assert_eq!(priorities.iter().copied().collect::<std::collections::HashSet<_>>().len(), 3);
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
        let Some((start, original)) = self.split_start else { return false };
        if !dragging { return false; }
        let next = (original + f32::from(y - start)).clamp(90.0, 650.0);
        if (self.top_height - next).abs() < 0.5 { return false; }
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
            ModelingElementKind::Class | ModelingElementKind::Template
                | ModelingElementKind::AssociationClass => ShapeKind::Class,
            ModelingElementKind::Interface => ShapeKind::Interface,
            ModelingElementKind::Enumeration => ShapeKind::Enum,
            ModelingElementKind::Package | ModelingElementKind::Model
                | ModelingElementKind::Subsystem => ShapeKind::Package,
            ModelingElementKind::Comment => ShapeKind::Note,
            ModelingElementKind::Frame => ShapeKind::Boundary,
            ModelingElementKind::NaryAssociation => ShapeKind::Aggregate,
            ModelingElementKind::Signal | ModelingElementKind::DataType
                | ModelingElementKind::PrimitiveType => ShapeKind::Class,
        },
        ModelingTool::Relation(kind) => match kind {
            ModelingRelationKind::Association | ModelingRelationKind::DirectedAssociation
                | ModelingRelationKind::Containment => ShapeKind::Association,
            ModelingRelationKind::Aggregation => ShapeKind::Aggregate,
            ModelingRelationKind::Composition => ShapeKind::Composition,
            ModelingRelationKind::Generalization
                | ModelingRelationKind::InterfaceRealization => ShapeKind::Inheritance,
            ModelingRelationKind::Dependency | ModelingRelationKind::Realization
                | ModelingRelationKind::TemplateBinding => ShapeKind::Dependency,
        },
    }
}

pub struct MdeLeftPanel {
    state: PanelState,
}

impl MdeLeftPanel {
    pub fn new(item: Entity<MdeWorkbenchItem>, cx: &mut Context<Self>) -> Self {
        Self { state: PanelState::new(item, DockPosition::Left, 216.0, cx) }
    }
}

impl EventEmitter<PanelEvent> for MdeLeftPanel {}
impl Focusable for MdeLeftPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.state.focus.clone() }
}
impl Panel for MdeLeftPanel {
    fn persistent_name() -> &'static str { "MDE Working Diagrams & Toolbox" }
    fn panel_key() -> &'static str { "bellatrix.mde.left.v1" }
    fn position(&self, _: &Window, _: &App) -> DockPosition { self.state.position }
    fn position_is_valid(&self, p: DockPosition) -> bool {
        matches!(p, DockPosition::Left | DockPosition::Right)
    }
    fn set_position(&mut self, p: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        if self.position_is_valid(p) { self.state.position = p; cx.notify(); }
    }
    fn default_size(&self, _: &Window, _: &App) -> Pixels { px(246.0) }
    fn min_size(&self, _: &Window, _: &App) -> Option<Pixels> { Some(px(190.0)) }
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> { Some(IconName::BellatrixMdeTools) }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> { Some("Bellatrix MDE - Working Diagrams & Toolbox (left dock)") }
    fn toggle_action(&self) -> Box<dyn Action> { Box::new(ToggleMdeLeft) }
    fn starts_open(&self, _: &Window, _: &App) -> bool { true }
    fn activation_priority(&self) -> u32 { MDE_LEFT_PRIORITY }
}

impl Render for MdeLeftPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let item = self.state.item.clone();
        let state = item.read(cx);
        let project = state.project_id().clone();
        let active = state.active_diagram().map(str::to_owned);
        let chosen = state.selected_tool();
        let snapshot = state.source().snapshot().ok();

        let mut diagrams = Vec::new();
        if let Some(snapshot) = &snapshot {
            for diagram in &snapshot.section.diagrams {
                let diagram_id = diagram.id.clone();
                let owner = item.clone();
                diagrams.push(
                    TreeViewItem::new(diagram_id.clone(), diagram.name.clone())
                        .toggle_state(active.as_deref() == Some(diagram_id.as_str()))
                        .on_click(move |_, window, cx| {
                            owner.update(cx, |workbench, cx| {
                                workbench.select_diagram(diagram_id.clone(), cx);
                            });
                            // The native Workspace owns the editor tab. Reopen it
                            // after a user closes the Canvas; never create a second Dock.
                            window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
                        }),
                );
            }
        }

        // StarUML Class Diagram toolbox: basic classifiers and relations,
        // advanced classifiers/connectors, packages and comment. A symbol
        // appears only if it has a typed backend command (not a fake button).
        let tool_groups: [(&str, Vec<ModelingTool>); 6] = [
            ("CLASSES (BASIC)", ModelingElementKind::BASIC.iter().copied()
                .map(ModelingTool::Element).collect()),
            ("RELATIONSHIPS (BASIC)", ModelingRelationKind::BASIC.iter().copied()
                .map(ModelingTool::Relation).collect()),
            ("CLASSES (ADVANCED)", ModelingElementKind::ADVANCED.iter().copied()
                .map(ModelingTool::Element).collect()),
            ("RELATIONSHIPS (ADVANCED)", ModelingRelationKind::ADVANCED.iter().copied()
                .map(ModelingTool::Relation).collect()),
            ("PACKAGES", ModelingElementKind::PACKAGES.iter().copied()
                .map(ModelingTool::Element).chain(
                    ModelingRelationKind::PACKAGES.iter().copied().map(ModelingTool::Relation)
                ).collect()),
            ("ANNOTATIONS", ModelingElementKind::ANNOTATIONS.iter().copied()
                .map(ModelingTool::Element).collect()),
        ];
        let mut tool_buttons: Vec<gpui::AnyElement> = Vec::new();
        for (group_index,(group_name,tools)) in tool_groups.into_iter().enumerate() {
            tool_buttons.push(
                div().id(format!("mde-tool-group-{group_index}"))
                    .pt_2().text_xs().text_color(palette.muted)
                    .child(group_name).into_any_element()
            );
            for (index,tool) in tools.into_iter().enumerate() {
                let click_item = item.clone();
                let glyph_bytes = glyph(tool_glyph(tool));
                // Relation tools are click-to-arm commands, not draggable
                // shapes. GPUI's drag recognizer on the same surface can
                // consume the click, leaving the Canvas in ordinary move mode.
                let entry = div()
                    .id(format!("mde-tool-{group_index}-{index}"))
                    .w_full().h(px(34.0)).flex().items_center().gap_2().px_2()
                    .border_1().border_color(palette.border).rounded_sm()
                    .when(chosen == Some(tool), |entry| entry
                        .border_color(palette.selection).bg(palette.surface))
                    .on_click(move |_, _, cx| {
                        click_item.update(cx, |workbench, cx| workbench.select_tool(tool, cx));
                    })
                    .child(svg().data(glyph_bytes).w(px(18.0)).h(px(18.0))
                        .text_color(palette.muted))
                    .child(Label::new(tool.label()));
                let entry: gpui::AnyElement = if matches!(tool, ModelingTool::Element(_)) {
                    entry.cursor_grab()
                        .on_drag(MdeToolDrag {
                            project_id: project.clone(), kind:tool,
                        }, |drag: &MdeToolDrag, _, _, cx| cx.new(|_| drag.clone()))
                        .into_any_element()
                } else {
                    entry.cursor_pointer().into_any_element()
                };
                tool_buttons.push(entry);
            }
        }

        div()
            .id("mde-left-panel")
            .track_focus(&self.state.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.panel)
            .on_mouse_move(cx.listener(|panel, event: &MouseMoveEvent, _, cx| {
                if panel.state.move_split(event.position.y, event.dragging()) { cx.notify(); }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|panel, _: &MouseUpEvent, _, _| {
                panel.state.finish_split();
            }))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|panel, _: &MouseUpEvent, _, _| {
                panel.state.finish_split();
            }))
            .child(
                div()
                    .h(px(self.state.top_height))
                    .flex_none()
                    .min_h(px(90.0))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div().h(px(36.0)).px_3().flex_none().flex().items_center()
                            .border_b_1().border_color(palette.border)
                            .child(Label::new("Working diagrams")),
                    )
                    .child(div().id("bellatrix-mde-diagrams-scroll").flex_1().overflow_y_scroll().children(diagrams)),
            )
            // This splitter belongs to the panel content, not to Zed Dock
            // placement. Each side independently retains its divider height.
            .child(
                div()
                    .id("mde-left-stack-resizer")
                    .h(px(6.0))
                    .w_full()
                    .flex_none()
                    .cursor_ns_resize()
                    .border_y_1()
                    .border_color(palette.border)
                    .on_mouse_down(MouseButton::Left, cx.listener(|panel, event: &MouseDownEvent, _, cx| {
                        panel.state.start_split(event.position.y);
                        cx.stop_propagation();
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(100.0))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div().h(px(36.0)).flex_none().px_3().flex().items_center()
                            .border_b_1().border_color(palette.border)
                            .child(Label::new("Toolbox")),
                    )
                    .child(
                        div().id("bellatrix-mde-toolbox-scroll")
                            .flex_1().overflow_y_scroll()
                            .p_2().flex().flex_col().gap_2()
                            .children(tool_buttons)
                            .child(Label::new(match chosen {
                                Some(ModelingTool::Element(kind)) =>
                                    format!("{}: drag to canvas or click empty space",kind.label()),
                                Some(ModelingTool::Relation(kind)) =>
                                    format!("{}: {}",kind.label(),kind.endpoint_instruction()),
                                None => "Choose a Relationship tool to connect two classes".into(),
                            }))
                            .when(matches!(chosen,Some(ModelingTool::Relation(_))), |toolbox| {
                                let owner=item.clone();
                                toolbox.child(
                                    Button::new("mde-cancel-toolbox-connection", "Cancel connection")
                                        .on_click(move |_,_,cx| {
                                            owner.update(cx,|workbench,cx|workbench.cancel_tool(cx));
                                        })
                                )
                            })
                            .child(
                                Button::new("bellatrix-open-symbol-library", "Browse symbol library")
                                    .on_click(move |_, window, cx| {
                                        window.dispatch_action(Box::new(OpenMdeSymbols), cx);
                                    }),
                            ),
                    ),
            )
    }
}

pub struct MdeRightPanel {
    state: PanelState,
    name_input: Entity<InputField>,
    save_input: Entity<InputField>,
    editing_id: Option<String>,
    property_inputs: HashMap<String, Entity<InputField>>,
    // These are presentation-only disclosure states; semantic ownership stays
    // in the MDE graph, while Zed's Project Panel owns filesystem disclosure.
    collapsed_elements: HashSet<String>,
}

impl MdeRightPanel {
    /// Requires the normal Zed editor::init startup, which registers the
    /// input field editor factory. Never initialize in GPUI Kit.
    pub fn new(
        item: Entity<MdeWorkbenchItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name_input = cx.new(|cx| InputField::new(window, cx, "Element name"));
        let save_input = cx.new(|cx| InputField::new(window, cx, "Project file path"));
        Self {
            state: PanelState::new(item, DockPosition::Right, 230.0, cx),
            name_input,
            save_input,
            editing_id: None,
            property_inputs: HashMap::new(),
            collapsed_elements: HashSet::new(),
        }
    }
}

impl EventEmitter<PanelEvent> for MdeRightPanel {}
impl Focusable for MdeRightPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.state.focus.clone() }
}
impl Panel for MdeRightPanel {
    fn persistent_name() -> &'static str { "MDE Model Explorer & Inspector" }
    fn panel_key() -> &'static str { "bellatrix.mde.right.v1" }
    fn position(&self, _: &Window, _: &App) -> DockPosition { self.state.position }
    fn position_is_valid(&self, p: DockPosition) -> bool {
        matches!(p, DockPosition::Left | DockPosition::Right)
    }
    fn set_position(&mut self, p: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        if self.position_is_valid(p) { self.state.position = p; cx.notify(); }
    }
    fn default_size(&self, _: &Window, _: &App) -> Pixels { px(292.0) }
    fn min_size(&self, _: &Window, _: &App) -> Option<Pixels> { Some(px(230.0)) }
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> { Some(IconName::BellatrixMdeInspector) }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> { Some("Bellatrix MDE - Model Explorer & Inspector (right dock)") }
    fn toggle_action(&self) -> Box<dyn Action> { Box::new(ToggleMdeRight) }
    fn starts_open(&self, _: &Window, _: &App) -> bool { true }
    fn activation_priority(&self) -> u32 { MDE_RIGHT_PRIORITY }
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
        let inspection = snapshot.as_ref().and_then(|snapshot| selected_id.as_deref()
            .and_then(|id| inspect_element(&snapshot.section, id)));
        let mut rows = Vec::new();

        if let Some(snapshot) = &snapshot {
            // StarUML-style semantic ownership. Only MDE elements, never
            // files/directories, appear here. Expand/collapse is local UI state.
            let tree = project_hierarchy(&snapshot.section);
            let mut hidden_by_depth: Option<usize> = None;
            for (index, row) in tree.iter().enumerate() {
                if let Some(ancestor_depth) = hidden_by_depth {
                    if row.depth > ancestor_depth {
                        continue;
                    }
                    hidden_by_depth = None;
                }
                let has_children = tree.get(index + 1)
                    .is_some_and(|next| next.depth > row.depth);
                let expanded = !self.collapsed_elements.contains(&row.element_id);
                let id = row.element_id.clone();
                let owner = item.clone();
                let disclosure_id = row.element_id.clone();
                let disclosure_panel = cx.weak_entity();
                rows.push(
                    div()
                        .id(format!("mde-explorer-row-{id}"))
                        .pl(px(row.depth.min(15) as f32 * 14.0))
                        .child(
                            TreeViewItem::new(id.clone(), row.label.clone())
                                .root_item(has_children)
                                .expanded(expanded)
                                .toggle_state(selected_id.as_deref() == Some(id.as_str()))
                                .on_toggle(move |_, _, cx| {
                                    let _ = disclosure_panel.update(cx, |panel, cx| {
                                        if !panel.collapsed_elements.insert(disclosure_id.clone()) {
                                            panel.collapsed_elements.remove(&disclosure_id);
                                        }
                                        cx.notify();
                                    });
                                })
                                .on_click(move |_, _, cx| {
                                    owner.update(cx, |workbench, cx| {
                                        workbench.select_element(id.clone(), cx);
                                    });
                                }),
                        ),
                );
                if has_children && !expanded {
                    hidden_by_depth = Some(row.depth);
                }
            }

            // A relationship is a separate graph edge, never a fake child
            // in the physical/semantic containment hierarchy.
            // Legacy/project-specific relationships are first-class model
            // relations too. If they have a Path View, show them in Explorer;
            // filtering only "uml." hid the fixture's depends_on curve.
            let relations = active_diagram.as_deref()
                .or_else(|| snapshot.section.diagrams.first().map(|diagram| diagram.id.as_str()))
                .map(|id| RelationshipGraph::new(&snapshot.section, id).visible_relations())
                .unwrap_or_default();
            if !relations.is_empty() {
                rows.push(div().id("mde-relationships-heading").p_2()
                    .text_size(px(11.0)).text_color(palette.muted)
                    .child("Relationships"));
            }
            for relation in relations {
                let presentation = relationship_presentation(relation);
                let from=snapshot.section.elements.iter().find(|e|e.id==relation.from)
                    .map_or(relation.from.as_str(),|e|e.name.as_str());
                let to=snapshot.section.elements.iter().find(|e|e.id==relation.to)
                    .map_or(relation.to.as_str(),|e|e.name.as_str());
                let id=relation.id.clone();
                let owner=item.clone();
                rows.push(
                    div().id(format!("mde-explorer-relation-{id}")).pl(px(14.0))
                        .child(
                            TreeViewItem::new(id.clone(),presentation.describe(from, to))
                                .toggle_state(selected_id.as_deref()==Some(id.as_str()))
                                .on_click(move |_,_,cx| {
                                    owner.update(cx,|workbench,cx|workbench.select_element(id.clone(),cx));
                                }),
                        ),
                );
            }
        }

        // Inputs belong to the selected object, not to a fixed Inspector form.
        // Rebuild on selection changes; avoid resetting in-progress typing
        // on every render. The domain's descriptors define these controls.
        if self.editing_id != selected_id {
            self.editing_id = selected_id.clone();
            self.property_inputs.clear();
            if let Some(inspection) = &inspection {
                for prop in &inspection.properties {
                    if !prop.editable || !matches!(prop.editor,
                        PropertyEditor::Text | PropertyEditor::Multiline | PropertyEditor::Number
                    ) {
                        continue;
                    }
                    let editor = if prop.path == "name" {
                        self.name_input.clone()
                    } else {
                        cx.new(|cx| InputField::new(window, cx, &prop.label))
                    };
                    let text = prop.value.as_str()
                        .map_or_else(|| prop.value.to_string(), str::to_owned);
                    editor.update(cx, |field, cx| field.set_text(&text, window, cx));
                    self.property_inputs.insert(prop.path.clone(), editor);
                }
            }
        }

        let inspector_rows = render_inspector(
            inspection.as_ref(), item.clone(), &self.property_inputs, window, cx
        );
        let save_item = item.clone();
        let save_input = self.save_input.clone();
        let delete_item = item.clone();

        div()
            .id("mde-right-panel")
            .track_focus(&self.state.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(panel_background)
            .on_mouse_move(cx.listener(|panel, event: &MouseMoveEvent, _, cx| {
                if panel.state.move_split(event.position.y, event.dragging()) { cx.notify(); }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|panel, _: &MouseUpEvent, _, _| {
                panel.state.finish_split();
            }))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|panel, _: &MouseUpEvent, _, _| {
                panel.state.finish_split();
            }))
            .child(
                div()
                    .h(px(self.state.top_height))
                    .flex_none()
                    .min_h(px(90.0))
                    .flex().flex_col().overflow_hidden()
                    .child(
                        div().h(px(36.0)).px_3().flex_none().flex().items_center()
                            .border_b_1().border_color(panel_border)
                            .child(Label::new("Model explorer")),
                    )
                    .child(
                        div().id("bellatrix-mde-explorer-scroll")
                            .flex_1().overflow_y_scroll()
                            .child(
                                div().px_3().py_2().text_xs()
                                    .text_color(palette.muted)
                                    .child(snapshot.as_ref().map_or_else(
                                        || "Project unavailable".to_string(),
                                        |s| format!("Model · {}", s.name),
                                    )),
                            )
                            .children(rows),
                    ),
            )
            .child(
                div()
                    .id("mde-right-stack-resizer")
                    .h(px(6.0)).w_full().flex_none().cursor_ns_resize()
                    .border_y_1().border_color(panel_border)
                    .on_mouse_down(MouseButton::Left, cx.listener(|panel, event: &MouseDownEvent, _, cx| {
                        panel.state.start_split(event.position.y);
                        cx.stop_propagation();
                    })),
            )
            .child(
                div()
                    .flex_1().min_h(px(110.0)).flex().flex_col().overflow_hidden()
                    .child(
                        div().h(px(36.0)).flex_none().px_3().flex().items_center()
                            .border_b_1().border_color(panel_border)
                            .child(Label::new("Property Editor · Inspector")),
                    )
                    .child(
                        div().id("bellatrix-mde-inspector-scroll")
                            .flex_1().overflow_y_scroll().p_2()
                            .flex().flex_col().gap_2()
                            .children(inspector_rows)
                            .child(
                                Button::new("mde-delete-element", "Delete selected element")
                                    .on_click(move |_, _, cx| {
                                        delete_item.update(cx, |workbench, cx| {
                                            workbench.delete_selected(cx);
                                        });
                                    }),
                            ),
                    )
                    // Project persistence is a separate platform concern,
                    // never a property of the currently selected Element.
                    .child(
                        div().w_full().flex_none().p_2()
                            .border_t_1().border_color(panel_border)
                            .flex().flex_col().gap_1()
                            .child(Label::new("Project persistence"))
                            .child(self.save_input.clone())
                            .child(Button::new("mde-save-project", "Save project").on_click(
                                move |_, _, cx| {
                                    let path = save_input.read(cx).text(cx);
                                    save_item.update(cx, |workbench, cx| {
                                        workbench.save_project(path, cx);
                                    });
                                },
                            )),
                    ),
            )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
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
        Self { state: PanelState::new(item, DockPosition::Bottom, 0.0, cx),
            active: BottomTab::Relationships }
    }
}

impl EventEmitter<PanelEvent> for MdeBottomPanel {}
impl Focusable for MdeBottomPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.state.focus.clone() }
}
impl Panel for MdeBottomPanel {
    fn persistent_name() -> &'static str { "MDE Auxiliary Dock" }
    fn panel_key() -> &'static str { "bellatrix.mde.bottom.v1" }
    fn position(&self, _: &Window, _: &App) -> DockPosition { self.state.position }
    fn position_is_valid(&self, p: DockPosition) -> bool { p == DockPosition::Bottom }
    fn set_position(&mut self, p: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        if self.position_is_valid(p) { self.state.position = p; cx.notify(); }
    }
    fn default_size(&self, _: &Window, _: &App) -> Pixels { px(190.0) }
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> { Some(IconName::BellatrixMdeDetails) }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> { Some("Bellatrix MDE - Details: Relations, Markdown & Thumbnails (bottom dock)") }
    fn toggle_action(&self) -> Box<dyn Action> { Box::new(ToggleMdeBottom) }
    fn starts_open(&self, _: &Window, _: &App) -> bool { false }
    fn activation_priority(&self) -> u32 { MDE_BOTTOM_PRIORITY }
}

impl Render for MdeBottomPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let active = self.active;
        let handle = cx.weak_entity();
        let mut tabs = Vec::new();
        for (idx, (kind, label)) in [
            (BottomTab::Relationships, "Relationships"),
            (BottomTab::Documentation, "Markdown"),
            (BottomTab::Thumbnails, "Thumbnails"),
        ].into_iter().enumerate() {
            let owner = handle.clone();
            tabs.push(
                Button::new(format!("mde-bottom-tab-{idx}"), label).on_click(
                    move |_, _, cx| {
                        let _ = owner.update(cx, |panel, cx| {
                            panel.active = kind;
                            cx.notify();
                        });
                    },
                ),
            );
        }
        let description = match active {
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
            .child(div().p_3().child(Label::new(description)))
    }
}

/// Called from a normal, fully initialized Zed Workspace context. This
/// installs Zed Panels without building a second DockArea or App runtime.
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

    let left = cx.new(|cx| MdeLeftPanel::new(item.clone(), cx));
    let right = cx.new(|cx| MdeRightPanel::new(item.clone(), window, cx));
    let bottom = cx.new(|cx| MdeBottomPanel::new(item, cx));
    workspace.add_panel(left, window, cx);
    workspace.add_panel(right, window, cx);
    workspace.add_panel(bottom, window, cx);
}
