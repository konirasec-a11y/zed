//! Panels owned by the Zed Workspace Dock, consuming the single MDE domain.
mod toolbox_panel;
pub(crate) use crate::hierarchy::element_glyph;
use crate::{
    MdeToolDrag, MdeWorkbenchItem, WorkbenchPalette,
    hierarchy::project_explorer,
    integration::{OpenMdeSymbols, OpenMdeWorkbench},
    property_editor::render_inspector,
};
use bellatrix_mde::commands::{ModelingElementKind, ModelingTool};
use bellatrix_mde::{
    ModelingRelationKind, inspect_element,
};
use bellatrix_ui_panels::PaneHeader;
use bellatrix_ui_symbols::{ShapeKind, glyph};
use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Render,
    StatefulInteractiveElement, Subscription, Window, actions, div, prelude::*, px, svg,
};
use std::collections::{HashMap, HashSet};
pub use toolbox_panel::MdeLeftPanel;
#[cfg(test)]
use toolbox_panel::tool_matches_query;
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
    item: Option<gpui::WeakEntity<MdeWorkbenchItem>>,
    focus: FocusHandle,
    position: DockPosition,
    top_height: f32,
    split_start: Option<(Pixels, f32)>,
    _listener: Option<Subscription>,
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
            item: Some(item.downgrade()),
            focus: cx.focus_handle(),
            position,
            top_height,
            split_start: None,
            _listener: Some(listener),
        }
    }
    fn bind<T: 'static>(&mut self, item: Option<&Entity<MdeWorkbenchItem>>, cx: &mut Context<T>) {
        self._listener = item.map(|item| cx.observe(item, |_, _, cx| cx.notify()));
        self.item = item.map(Entity::downgrade);
        self.split_start = None;
        cx.notify();
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
pub struct MdeRightPanel {
    state: PanelState,
    editing_id: Option<String>,
    pub(crate) property_drafts:
        HashMap<crate::inspector_drafts::DraftKey, crate::property_editor::InspectorDraft>,
    style_target_id: Option<crate::style_editor::StyleKey>,
    pub(crate) style_drafts:
        HashMap<crate::style_editor::StyleKey, crate::style_editor::StyleDraft>,
    pub(crate) inspector_styles_open: bool,
    collapsed_elements: HashSet<String>,
    collapsed_by_document: HashMap<gpui::EntityId, HashSet<String>>,
    explorer_open: bool,
    inspector_open: bool,
    pub(crate) inspector_documentation_open: bool,
    pub(crate) inspector_advanced_open: bool,
}
impl MdeRightPanel {
    pub(crate) fn is_bound_to(&self, item: &Entity<MdeWorkbenchItem>) -> bool {
        self.state
            .item
            .as_ref()
            .and_then(gpui::WeakEntity::upgrade)
            .is_some_and(|bound| bound.entity_id() == item.entity_id())
    }

    pub fn new(
        item: Entity<MdeWorkbenchItem>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            state: PanelState::new(item, DockPosition::Right, 230.0, cx),
            editing_id: None,
            property_drafts: HashMap::new(),
            style_drafts: HashMap::new(),
            style_target_id: None,
            inspector_styles_open: true,
            collapsed_elements: HashSet::new(),
            collapsed_by_document: HashMap::new(),
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
        let Some(item) = self.state.item.as_ref().and_then(gpui::WeakEntity::upgrade) else {
            return div()
                .p_3()
                .child(Label::new("Select an MDE document to inspect its model."))
                .into_any_element();
        };
        let read_only = item.read(cx).source().is_read_only();
        let palette = WorkbenchPalette::current(cx);
        let panel_background = palette.panel;
        let panel_border = palette.border;
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
            let tree = project_explorer(&snapshot.section);
            let root_key = format!("project:{}", snapshot.project_id.as_str());
            let root_expanded = !self.collapsed_elements.contains(&root_key);
            let root_panel = cx.weak_entity();
            rows.push(div().id("mde-explorer-project").flex().items_center().gap_1()

                .child(svg().data(glyph(ShapeKind::Package)).size(px(14.0)).flex_none().text_color(palette.muted))
                .child(TreeViewItem::new(root_key.clone(), snapshot.name.clone())
                    .root_item(!tree.is_empty()).expanded(root_expanded)
                    .on_toggle(move |_, _, cx| {
                        let _ = root_panel.update(cx, |panel, cx| {
                            if !panel.collapsed_elements.insert(root_key.clone()) { panel.collapsed_elements.remove(&root_key); }
                            cx.notify();
                        });
                    })));
            let mut hidden_by_depth: Option<usize> = None;
            for (index, row) in tree.iter().enumerate() {
                if !root_expanded { break; }
                if let Some(ancestor_depth) = hidden_by_depth {
                    if row.depth > ancestor_depth {
                        continue;
                    }
                    hidden_by_depth = None;
                }
                let has_children = tree
                    .get(index + 1)
                    .is_some_and(|next| next.depth > row.depth);
                if row.diagram {
                    let id = row.element_id.clone();
                    let owner = item.clone();
                    let is_active = active_diagram.as_deref() == Some(id.as_str());
                    rows.push(
                        div()
                            .id(format!("mde-explorer-diagram-{id}"))
                            .pl(px((row.depth.min(15) + 2) as f32 * 14.0))
                            .flex().items_center().gap_1()
                            .when(is_active, |row| row.bg(palette.surface))
                            .child(svg().data(glyph(ShapeKind::Document)).size(px(14.0)).flex_none().text_color(if is_active { palette.selection } else { palette.muted }))
                            .child(div().flex_1().min_w_0().overflow_hidden().child(
                                TreeViewItem::new(id.clone(), row.label.clone())
                                    .toggle_state(is_active)
                                    .on_click(move |_, _, cx| {
                                        owner.update(cx, |workbench, cx| workbench.select_diagram(id.clone(), cx));
                                    })
                            ))
                            .child(Label::new(row.kind.clone()).size(LabelSize::XSmall).color(Color::Muted)),
                    );
                    continue;
                }
                let expanded = !self.collapsed_elements.contains(&row.element_id);
                let id = row.element_id.clone();
                let owner = item.clone();
                let disclosure_id = row.element_id.clone();
                let disclosure_panel = cx.weak_entity();
                let menu_owner = item.clone();
                let menu_subject = id.clone();
                let label = row.label.clone();
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

                        .pl(px((row.depth.min(15) + 2) as f32 * 14.0))
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
                                    let read_only = menu_owner.read(cx).source().is_read_only();
                                    ui::ContextMenu::build(window, cx, move |menu, _, _| {
                                        if read_only {
                                            return menu.header("Read-only preview");
                                        }
                                        menu.entry(
                                            "Show in active diagram",
                                            None,
                                            move |window, cx| {
                                                if show_owner.read(cx).source().is_read_only() {
                                                    return;
                                                }
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
                                                if delete_owner.read(cx).source().is_read_only() {
                                                    return;
                                                }
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

        }
        if !read_only && self.editing_id != selected_id {
            if self.property_drafts.values().any(|draft| {
                draft.input.as_ref().is_some_and(|input| {
                    input.read(cx).focus_handle(cx).contains_focused(window, cx)
                })
            }) {
                window.focus(&self.state.focus, cx);
            }
            self.editing_id = selected_id.clone();
        }
        let inspector_project = item.read(cx).project_id().clone();
        if !read_only {
            crate::property_editor::reconcile_inputs(
                &mut self.property_drafts,
                item.clone(),
                inspector_project.as_str(),
                inspection.as_ref(),
                snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
                window,
                cx,
            );
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
        let style_target_id = style_target.map(|(diagram_id, view, _)| {
            (
                inspector_project.as_str().to_owned(),
                diagram_id.to_owned(),
                view.id.clone(),
            )
        });
        if !read_only && self.style_target_id != style_target_id {
            if self
                .style_target_id
                .as_ref()
                .and_then(|key| self.style_drafts.get(key))
                .is_some_and(|draft| {
                    draft
                        .inputs
                        .values()
                        .any(|input| input.read(cx).focus_handle(cx).contains_focused(window, cx))
                })
            {
                window.focus(&self.state.focus, cx);
            }
            self.style_target_id = style_target_id.clone();
        }
        if !read_only {
            if let Some((diagram_id, view, _)) = style_target {
                crate::style_editor::reconcile_style_inputs(
                    &mut self.style_drafts,
                    &item,
                    diagram_id,
                    view,
                    snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
                    window,
                    cx,
                );
            }
        }
        let mut inspector_rows = crate::style_editor::render_styles(
            style_target,
            item.clone(),
            if read_only {
                None
            } else {
                style_target_id
                    .as_ref()
                    .and_then(|key| self.style_drafts.get(key))
            },
            self.inspector_styles_open,
            window,
            cx,
        );
        inspector_rows.extend(render_inspector(
            inspection.as_ref(),
            item.clone(),
            &self.property_drafts,
            snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
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
                            .when(!read_only, |header| {
                                header.action(Button::new("mde-save-project", "Save As…").on_click(
                                    move |_, window, cx| {
                                        window.dispatch_action(
                                            Box::new(crate::integration::SaveMdeProjectAs),
                                            cx,
                                        );
                                    },
                                ))
                            }),
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
                                        project_explorer(&snapshot.section).is_empty()
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
                                .when(selected_id.is_some() && !read_only, |pane| {
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
            .into_any_element()
    }
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
        let Some(item) = self.state.item.as_ref().and_then(gpui::WeakEntity::upgrade) else {
            return div()
                .p_3()
                .child(Label::new("Select an MDE document to inspect its model."))
                .into_any_element();
        };
        let read_only = item.read(cx).source().is_read_only();
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
            .when(read_only, |panel| {
                panel.child(Label::new("Read-only preview"))
            })
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
                        let feedback = item.read(cx).feedback();
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
            .into_any_element()
    }
}

pub(crate) fn bind_panels(
    workspace: &Workspace,
    item: Option<Entity<MdeWorkbenchItem>>,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(panel) = workspace.panel::<MdeLeftPanel>(cx) {
        panel.update(cx, |panel, cx| {
            if panel
                .search_input
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx)
            {
                window.focus(&panel.state.focus, cx);
            }
            panel.state.bind(item.as_ref(), cx);
        });
    }
    if let Some(panel) = workspace.panel::<MdeRightPanel>(cx) {
        panel.update(cx, |panel, cx| {
            let previous = panel
                .state
                .item
                .as_ref()
                .and_then(gpui::WeakEntity::upgrade)
                .map(|item| item.entity_id());
            let next = item.as_ref().map(Entity::entity_id);
            if previous != next {
                let hidden_input_focused = panel.property_drafts.values().any(|draft| {
                    draft.input.as_ref().is_some_and(|input| {
                        input.read(cx).focus_handle(cx).contains_focused(window, cx)
                    })
                }) || panel.style_drafts.values().any(|draft| {
                    draft
                        .inputs
                        .values()
                        .any(|input| input.read(cx).focus_handle(cx).contains_focused(window, cx))
                });
                if hidden_input_focused {
                    window.focus(&panel.state.focus, cx);
                }
                if let Some(previous) = previous {
                    panel
                        .collapsed_by_document
                        .insert(previous, std::mem::take(&mut panel.collapsed_elements));
                }
                panel.collapsed_elements = next
                    .and_then(|id| panel.collapsed_by_document.remove(&id))
                    .unwrap_or_default();
            }
            panel.state.bind(item.as_ref(), cx);
            if let Some(item) = item
                .as_ref()
                .filter(|item| !item.read(cx).source().is_read_only())
            {
                let item = item.downgrade();
                let owner = cx.weak_entity();
                cx.defer(move |cx| {
                    let Some(item) = item.upgrade() else {
                        return;
                    };
                    if let Err(error) = owner.update(cx, |panel, cx| {
                        if panel.is_bound_to(&item) && !item.read(cx).source().is_read_only() {
                            crate::property_editor::flush_property_edits(panel, &item, cx);
                            crate::style_editor::flush_style_edits(panel, &item, cx);
                        }
                    }) {
                        eprintln!("MDE document queue resume failed: {error}");
                    }
                });
            }
        });
    }
    if let Some(panel) = workspace.panel::<MdeBottomPanel>(cx) {
        panel.update(cx, |panel, cx| panel.state.bind(item.as_ref(), cx));
    }
}

pub fn attach_panels(
    workspace: &mut Workspace,
    item: Entity<MdeWorkbenchItem>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    if workspace.panel::<MdeLeftPanel>(cx).is_some() {
        bind_panels(workspace, Some(item), window, cx);
        return;
    }
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
