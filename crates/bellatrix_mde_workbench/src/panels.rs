//! Panels owned by the Zed Workspace Dock.
//!
//! Zed's Dock shows one active panel per side. The two Bellatrix side Panels
//! compose the Working Diagrams/Toolbox and Explorer/Inspector stacks from
//! docs/design/ while Zed still owns placement, focus and persistence.

use crate::{MdeToolDrag, MdeWorkbenchItem, WorkbenchPalette};
use bellatrix_mde_cordis::ModelingElementKind;
use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    InteractiveElement, StatefulInteractiveElement, Pixels, Render, Subscription, Window, actions, div, prelude::*, px,
};
use ui::{Button, IconName, Label, TreeViewItem, prelude::*};
use ui_input::InputField;
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
    _listener: Subscription,
}

impl PanelState {
    fn new<T: 'static>(item: Entity<MdeWorkbenchItem>, position: DockPosition, cx: &mut Context<T>) -> Self {
        let listener = cx.observe(&item, |_, _, cx| cx.notify());
        Self {
            item, focus: cx.focus_handle(), position, _listener: listener,
        }
    }
}

pub struct MdeLeftPanel {
    state: PanelState,
}

impl MdeLeftPanel {
    pub fn new(item: Entity<MdeWorkbenchItem>, cx: &mut Context<Self>) -> Self {
        Self { state: PanelState::new(item, DockPosition::Left, cx) }
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
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> { Some(IconName::FileTree) }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> { Some("MDE Tools") }
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
                        .on_click(move |_, _, cx| {
                            owner.update(cx, |workbench, cx| {
                                workbench.select_diagram(diagram_id.clone(), cx);
                            });
                        }),
                );
            }
        }

        let tools = [
            ("class", "Class", ModelingElementKind::Class),
            ("interface", "Interface", ModelingElementKind::Interface),
        ];
        let mut tool_buttons = Vec::new();
        for (id, name, kind) in tools {
            let click_item = item.clone();
            tool_buttons.push(
                div()
                    .id(format!("mde-tool-drag-{id}"))
                    .on_drag(MdeToolDrag {
                        project_id: project.clone(), kind,
                    }, |drag: &MdeToolDrag, _, _, cx| {
                        cx.new(|_| drag.clone())
                    })
                    .child(
                        Button::new(format!("mde-tool-{id}"), name)
                            .on_click(move |_, _, cx| {
                                click_item.update(cx, |workbench, cx| {
                                    workbench.select_tool(kind, cx);
                                });
                            }),
                    ),
            );
        }

        div()
            .id("mde-left-panel")
            .track_focus(&self.state.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.panel)
            .child(
                div().h(px(36.0)).px_3().flex().items_center()
                    .border_b_1().border_color(palette.border)
                    .child(Label::new("Working diagrams")),
            )
            .child(div().id("bellatrix-mde-diagrams-scroll").flex_1().overflow_y_scroll().children(diagrams))
            .child(
                div().h(px(36.0)).px_3().flex().items_center()
                    .border_y_1().border_color(palette.border)
                    .child(Label::new("Toolbox")),
            )
            .child(
                div().p_2().flex().flex_col().gap_2()
                    .children(tool_buttons)
                    .child(Label::new(match chosen {
                        Some(ModelingElementKind::Class) => "Class: click canvas to place",
                        Some(ModelingElementKind::Interface) => "Interface: click canvas to place",
                        None => "Drag a tool onto the canvas",
                    })),
            )
    }
}

pub struct MdeRightPanel {
    state: PanelState,
    name_input: Entity<InputField>,
    save_input: Entity<InputField>,
    editing_id: Option<String>,
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
            state: PanelState::new(item, DockPosition::Right, cx),
            name_input,
            save_input,
            editing_id: None,
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
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> { Some(IconName::ListTree) }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> { Some("MDE Inspector") }
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
        let snapshot = item.read(cx).source().snapshot().ok();
        let mut rows = Vec::new();
        let mut selected_name = None;

        if let Some(snapshot) = &snapshot {
            for element in &snapshot.section.elements {
                if Some(element.id.as_str()) == selected_id.as_deref() {
                    selected_name = Some(element.name.clone());
                }
                let id = element.id.clone();
                let owner = item.clone();
                rows.push(
                    TreeViewItem::new(id.clone(), element.name.clone())
                        .toggle_state(selected_id.as_deref() == Some(id.as_str()))
                        .on_click(move |_, _, cx| {
                            owner.update(cx, |workbench, cx| {
                                workbench.select_element(id.clone(), cx);
                            });
                        }),
                );
            }
        }

        // Do not overwrite the user's in-progress input during render.
        // Only initialize the name field when the selected item actually changes.
        if self.editing_id != selected_id {
            self.editing_id = selected_id.clone();
            if let Some(name) = selected_name {
                self.name_input.update(cx, |input, cx| input.set_text(&name, window, cx));
            }
        }

        let rename_item = item.clone();
        let name_input = self.name_input.clone();
        let save_item = item.clone();
        let save_input = self.save_input.clone();

        div()
            .id("mde-right-panel")
            .track_focus(&self.state.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(panel_background)
            .child(
                div().h(px(36.0)).px_3().flex().items_center()
                    .border_b_1().border_color(panel_border)
                    .child(Label::new("Model explorer")),
            )
            .child(div().id("bellatrix-mde-explorer-scroll").flex_1().overflow_y_scroll().children(rows))
            .child(
                div().h(px(36.0)).px_3().flex().items_center()
                    .border_y_1().border_color(panel_border)
                    .child(Label::new("Inspector")),
            )
            .child(
                div()
                    .p_2().flex().flex_col().gap_2()
                    .child(self.name_input.clone())
                    .child(Button::new("mde-inspector-rename", "Rename selected").on_click(
                        move |_, _, cx| {
                            let name = name_input.read(cx).text(cx);
                            rename_item.update(cx, |workbench, cx| {
                                workbench.rename_selected(name, cx);
                            });
                        },
                    ))
                    .child(self.save_input.clone())
                    .child(Button::new("mde-save-project", "Save project").on_click(
                        move |_, _, cx| {
                            let path = save_input.read(cx).text(cx);
                            save_item.update(cx, |workbench, cx| {
                                workbench.save_project(path, cx);
                            });
                        },
                    )),
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
        Self { state: PanelState::new(item, DockPosition::Bottom, cx),
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
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> { Some(IconName::ListTodo) }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> { Some("MDE Details") }
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
