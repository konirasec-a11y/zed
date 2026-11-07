//! Panels owned by the Zed Workspace Dock, consuming the single MDE domain.
use crate::components::inspector::render_inspector;
use crate::components::model_explorer::ExplorerSortOrder;
use crate::components::model_explorer::filter_explorer;
use crate::components::model_explorer::project_explorer;
use crate::components::pane_header::PaneHeader;
pub use crate::components::toolbox::MdeLeftPanel;
use crate::pages::mde_page::MdeWorkbenchItem;
use crate::pages::mde_page::WorkbenchPalette;
use crate::routes::workspace::OpenMdeWorkbench;
use gpui::Action;
use gpui::App;
use gpui::Context;
use gpui::Entity;
use gpui::EventEmitter;
use gpui::FocusHandle;
use gpui::Focusable;
use gpui::InteractiveElement;
use gpui::IntoElement;
use gpui::MouseButton;
use gpui::MouseDownEvent;
use gpui::MouseMoveEvent;
use gpui::MouseUpEvent;
use gpui::Pixels;
use gpui::Render;
use gpui::StatefulInteractiveElement;
use gpui::Subscription;
use gpui::Window;
use gpui::actions;
use gpui::div;
use gpui::prelude::*;
use gpui::px;
use std::collections::HashMap;
use std::collections::HashSet;
use ui::IconButton;
use ui::IconName;
use ui::Label;
use ui::TreeViewItem;
use ui::prelude::*;
use ui_input::ErasedEditorEvent;
use ui_input::InputField;
use workspace::Workspace;
use workspace::dock::DockPosition;
use workspace::dock::Panel;
use workspace::dock::PanelEvent;

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
pub(crate) const MDE_LEFT_PRIORITY: u32 = 10_000;
const MDE_RIGHT_PRIORITY: u32 = 10_001;
const MDE_BOTTOM_PRIORITY: u32 = 10_002;

fn explorer_move_menu(
    menu: ui::ContextMenu,
    owner: &Entity<MdeWorkbenchItem>,
    snapshot: &crate::pages::mde_page::MdeSnapshot,
    target: crate::components::model_explorer::ExplorerMoveTarget,
    _panel: gpui::WeakEntity<MdeRightPanel>,
) -> ui::ContextMenu {
    use crate::components::model_explorer::ExplorerMoveTarget;
    let current_parent = match &target {
        ExplorerMoveTarget::Element(id) => snapshot
            .section
            .relations
            .iter()
            .find(|relation| relation.kind == "owns" && relation.to == *id)
            .map(|relation| relation.from.clone()),
        ExplorerMoveTarget::Diagram(id) => snapshot
            .section
            .hierarchy
            .get(id)
            .and_then(|placement| placement.parent_id.clone()),
    };
    let project_parent = match &target {
        ExplorerMoveTarget::Element(_) => snapshot.section.root_model.clone(),
        ExplorerMoveTarget::Diagram(_) => None,
    };
    let mut destinations = vec![(
        project_parent.clone(),
        format!("Project: {}", snapshot.name),
    )];
    destinations.extend(
        snapshot
            .section
            .hierarchy_rows
            .clone()
            .into_iter()
            .filter(|row| {
                project_parent.as_deref() != Some(row.element_id.as_str())
                    && snapshot
                        .section
                        .elements
                        .iter()
                        .any(|element| element.id == row.element_id)
            })
            .map(|row| {
                (
                    Some(row.element_id),
                    format!(
                        "{}{} · {}",
                        "  ".repeat(row.depth.min(8)),
                        row.label,
                        row.kind
                    ),
                )
            }),
    );
    destinations.retain(|(parent, _)| parent != &current_parent);
    if destinations.is_empty() {
        return menu;
    }
    let owner = owner.clone();
    let project_id = snapshot.project_id.clone();
    let revision = snapshot.revision;
    let title = match target {
        ExplorerMoveTarget::Element(_) => "Move element into",
        ExplorerMoveTarget::Diagram(_) => "Move diagram into",
    };
    menu.submenu(title, move |mut menu, _, _cx| {
        // Let the kernel explain rejected moves instead of silently hiding them.
        for (parent, label) in &destinations {
            let owner = owner.clone();
            let project_id = project_id.clone();
            let target = target.clone();
            let parent = parent.clone();
            menu = menu.entry(label.clone(), None, move |_, cx| {
                owner.update(cx, |workbench, cx| {
                    workbench.move_explorer_item(
                        &project_id,
                        revision,
                        &target,
                        parent.clone(),
                        None,
                        cx,
                    )
                });
            });
        }
        menu
    })
}

fn explorer_menu(
    owner: &Entity<MdeWorkbenchItem>,
    subject: &str,
    occurrence: Option<(String, String)>,
    panel: gpui::WeakEntity<MdeRightPanel>,
    row_key: String,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ui::ContextMenu> {
    let read_only = owner.read(cx).source().is_read_only()
        || owner.read(cx).source().is_presentation_suspended();
    let mut occurrences = Vec::new();
    let snapshot = owner.read(cx).source().snapshot().ok();
    if let Some(snapshot) = &snapshot {
        for diagram in &snapshot.section.diagrams {
            for (index, view) in diagram.all_views().enumerate() {
                if view.subject.as_deref() == Some(subject) {
                    occurrences.push((
                        diagram.id.clone(),
                        view.id.clone(),
                        format!("Select in {} · view {}", diagram.name, index + 1),
                    ));
                }
            }
        }
    }
    let owner = owner.clone();
    let subject = subject.to_owned();
    ui::ContextMenu::build(window, cx, move |mut menu, _, cx| {
        if !occurrences.is_empty() {
            menu = menu.header("Diagram occurrences");
        }
        for (diagram, view, label) in occurrences {
            let owner = owner.clone();
            menu = menu.entry(label, None, move |_, cx| {
                owner.update(cx, |workbench, cx| {
                    workbench.select_diagram_view(diagram.clone(), view.clone(), cx)
                });
            });
        }
        if read_only {
            return menu.header("Read-only preview");
        }
        if let Some((diagram_id, view_id)) = occurrence {
            let remove_owner = owner.clone();
            menu = menu.entry("Remove from diagram", None, move |_, cx| {
                remove_owner.update(cx, |workbench, cx| {
                    workbench.remove_view(diagram_id.clone(), view_id.clone(), cx)
                });
            });
        }
        if let Some(snapshot) = &snapshot {
            menu = explorer_creation_menus(
                menu,
                &owner,
                snapshot,
                Some(&subject),
                panel.clone(),
                &row_key,
                cx,
            );
            menu = explorer_move_menu(
                menu,
                &owner,
                snapshot,
                crate::components::model_explorer::ExplorerMoveTarget::Element(subject.clone()),
                panel.clone(),
            );
        }
        if let Some(snapshot) = &snapshot {
            if let Some(element) = snapshot
                .section
                .elements
                .iter()
                .find(|element| element.id == subject)
            {
                let rename_owner = owner.clone();
                let rename_panel = panel.clone();
                let project_id = snapshot.project_id.clone();
                let element_id = subject.clone();
                let revision = snapshot.revision;
                let name = element
                    .template_parametered_id
                    .as_ref()
                    .and_then(|id| {
                        snapshot
                            .section
                            .elements
                            .iter()
                            .find(|element| &element.id == id)
                    })
                    .unwrap_or(element)
                    .name
                    .clone();
                menu = menu.entry("Rename element", None, move |window, cx| {
                    if let Err(error) = rename_panel.update(cx, |panel, cx| {
                        if !panel.is_bound_to(&rename_owner)
                            || rename_owner.read(cx).source().is_read_only()
                        {
                            return;
                        }
                        let input = cx.new(|cx| {
                            let input = InputField::new(window, cx, "Element name")
                                .label_min_width(px(0.0));
                            input.set_text(&name, window, cx);
                            input
                        });
                        window.focus(&input.read(cx).focus_handle(cx), cx);
                        panel.explorer_action = Some(ExplorerActionDraft {
                            item_id: rename_owner.entity_id(),
                            project_id: project_id.clone(),
                            target: ExplorerActionTarget::Element(element_id.clone()),
                            revision,
                            name: name.clone(),
                            rename: Some(input),
                        });
                        panel.explorer_open = true;
                        cx.notify();
                    }) {
                        eprintln!("MDE element rename failed: {error}");
                    }
                });
            }
        }
        let show_owner = owner.clone();
        let show_subject = subject.clone();
        menu.entry("Show in active diagram", None, move |window, cx| {
            if show_owner.read(cx).source().is_read_only() {
                return;
            }
            show_owner.update(cx, |item, cx| {
                item.show_element_in_active_diagram(show_subject.clone(), window, cx)
            });
            window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
        })
        .entry("Delete from model", None, move |window, cx| {
            if owner.read(cx).source().is_read_only() {
                return;
            }
            owner.update(cx, |item, cx| {
                item.select_element(subject.clone(), cx);
                item.delete_selected_from_model(cx);
            });
            window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
        })
    })
}

enum ExplorerActionTarget {
    Diagram(String),
    Element(String),
}

struct ExplorerActionDraft {
    item_id: gpui::EntityId,
    project_id: bellatrix_mde_cordis::project::ProjectId,
    target: ExplorerActionTarget,
    revision: u64,
    name: String,
    rename: Option<Entity<InputField>>,
}

fn explorer_creation_menus(
    mut menu: ui::ContextMenu,
    owner: &Entity<MdeWorkbenchItem>,
    snapshot: &crate::pages::mde_page::MdeSnapshot,
    subject: Option<&str>,
    panel: gpui::WeakEntity<MdeRightPanel>,
    row_key: &str,
    cx: &App,
) -> ui::ContextMenu {
    use crate::view_model::explorer;
    if !owner.read(cx).source().allows_creation() {
        return menu.header("Creation is not available for this document");
    }
    let diagram_kinds = subject
        .map(|id| explorer::contextual_diagram_kinds(&snapshot.section, id))
        .unwrap_or_else(|| explorer::project_diagram_kinds(&snapshot.section));
    if !diagram_kinds.is_empty() {
        let owner = owner.clone();
        let snapshot = snapshot.clone();
        let parent_occurrence = explorer::parent_occurrence(&snapshot.section, subject, row_key);
        let subject = subject.map(str::to_owned);
        let panel = panel.clone();
        let row_key = row_key.to_owned();
        menu = menu.submenu("Add Diagram", move |mut menu, _, _| {
            for kind in diagram_kinds.iter().copied() {
                let owner = owner.clone();
                let project_id = snapshot.project_id.clone();
                let revision = snapshot.revision;
                let parent_id = subject.clone();
                let parent_occurrence = parent_occurrence.clone();
                let label = crate::pages::mde_page::diagram_kind_label(kind).to_owned();
                let panel = panel.clone();
                let row_key = row_key.clone();
                menu = menu.entry(label, None, move |_, cx| {
                    let sent = owner.update(cx, |workbench, cx| {
                        workbench.submit_explorer_operation(
                            &project_id,
                            revision,
                            |source| {
                                source.create_explorer_diagram(
                                    kind,
                                    parent_id.as_deref(),
                                    parent_occurrence.clone(),
                                    revision,
                                )
                            },
                            cx,
                        )
                    });
                    if sent {
                        reveal_explorer_owner(&panel, &owner, &row_key, cx);
                    }
                });
            }
            menu
        });
    }
    let element_kinds = explorer::model_creation_kinds(&snapshot.section, subject);
    let member_kinds = subject
        .map(|id| explorer::member_creation_kinds(&snapshot.section, id))
        .unwrap_or_default();
    if !element_kinds.is_empty() || !member_kinds.is_empty() {
        let owner = owner.clone();
        let snapshot = snapshot.clone();
        let subject = subject.map(str::to_owned);
        let row_key = row_key.to_owned();
        menu = menu.submenu("Add", move |mut menu, _, _| {
            for kind in element_kinds.iter().copied() {
                let owner = owner.clone();
                let project_id = snapshot.project_id.clone();
                let revision = snapshot.revision;
                let owner_id = subject.clone();
                let panel = panel.clone();
                let row_key = row_key.clone();
                menu = menu.entry(
                    snapshot
                        .section
                        .tools
                        .element(kind)
                        .map(|metadata| metadata.label.clone())
                        .unwrap_or_else(|| "Element".into()),
                    None,
                    move |_, cx| {
                        let sent = owner.update(cx, |workbench, cx| {
                            workbench.submit_explorer_operation(
                                &project_id,
                                revision,
                                |source| {
                                    source.create_model_element(kind, owner_id.as_deref(), revision)
                                },
                                cx,
                            )
                        });
                        if sent {
                            reveal_explorer_owner(&panel, &owner, &row_key, cx);
                        }
                    },
                );
            }
            if let Some(subject) = &subject {
                for (kind, label) in member_kinds.iter().copied() {
                    let owner = owner.clone();
                    let project_id = snapshot.project_id.clone();
                    let revision = snapshot.revision;
                    let owner_id = subject.clone();
                    let panel = panel.clone();
                    let row_key = row_key.clone();
                    menu = menu.entry(label, None, move |_, cx| {
                        let sent = owner.update(cx, |workbench, cx| {
                            workbench.submit_explorer_operation(
                                &project_id,
                                revision,
                                |source| source.add_member(&owner_id, kind, revision),
                                cx,
                            )
                        });
                        if sent {
                            reveal_explorer_owner(&panel, &owner, &row_key, cx);
                        }
                    });
                }
            }
            menu
        });
    }
    menu
}

fn reveal_explorer_owner(
    panel: &gpui::WeakEntity<MdeRightPanel>,
    owner: &Entity<MdeWorkbenchItem>,
    row_key: &str,
    cx: &mut App,
) {
    if let Err(error) = panel.update(cx, |panel, cx| {
        if panel.is_bound_to(owner) {
            panel.collapsed_elements.remove(row_key);
            cx.notify();
        }
    }) {
        eprintln!("MDE element owner reveal failed: {error}");
    }
}

fn explorer_project_menu(
    owner: &Entity<MdeWorkbenchItem>,
    panel: gpui::WeakEntity<MdeRightPanel>,
    row_key: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ui::ContextMenu> {
    let snapshot = owner.read(cx).source().snapshot();
    let editable = !owner.read(cx).source().is_read_only()
        && !owner.read(cx).source().is_presentation_suspended();
    let owner = owner.clone();
    let row_key = row_key.to_owned();
    ui::ContextMenu::build(window, cx, move |menu, _, cx| {
        if !editable {
            return menu.header("Read-only preview");
        }
        let Ok(snapshot) = snapshot else {
            return menu.header("Project snapshot unavailable");
        };
        explorer_creation_menus(menu, &owner, &snapshot, None, panel, &row_key, cx)
    })
}

/// Navigation only: unbound visual objects are not model-tree children.
fn explorer_unbound_views_menu(
    owner: &Entity<MdeWorkbenchItem>,
    diagram_id: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ui::ContextMenu> {
    let snapshot = owner.read(cx).source().snapshot().ok();
    let diagram = snapshot.as_ref().and_then(|snapshot| {
        snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
    });
    let mut entries = Vec::new();
    let mut nested_count = 0usize;
    if let Some(diagram) = diagram {
        for view in diagram.all_views().filter(|view| view.subject.is_none()) {
            if !diagram
                .views
                .iter()
                .any(|top_level| top_level.id == view.id)
            {
                nested_count += 1;
                continue;
            }
            let kind = view
                .draft
                .as_ref()
                .map(|draft| draft.notation_kind.as_str())
                .or(view.presentation.kind.as_deref())
                .unwrap_or("Visual View");
            let label = view
                .label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
                .unwrap_or(kind);
            entries.push((
                view.id.clone(),
                format!(
                    "{} · {} · {}",
                    entries.len() + 1,
                    if view.draft.is_some() {
                        "Draft"
                    } else {
                        "Unbound View"
                    },
                    label
                ),
            ));
        }
    }
    let owner = owner.clone();
    let diagram_id = diagram_id.to_owned();
    ui::ContextMenu::build(window, cx, move |mut menu, _, _| {
        menu = menu.header("Visual objects without a model subject");
        if entries.is_empty() {
            menu = menu.header("No selectable top-level views");
        }
        for (view_id, label) in entries {
            let owner = owner.clone();
            let diagram_id = diagram_id.clone();
            menu = menu.entry(label, None, move |_, cx| {
                owner.update(cx, |workbench, cx| {
                    workbench.select_diagram_view(diagram_id.clone(), view_id.clone(), cx)
                });
            });
        }
        if nested_count > 0 {
            menu = menu.header(format!("{nested_count} nested views: inspect through their parent View; visual editing unavailable"));
        }
        menu
    })
}

fn explorer_diagram_menu(
    owner: &Entity<MdeWorkbenchItem>,
    panel: gpui::WeakEntity<MdeRightPanel>,
    diagram_id: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ui::ContextMenu> {
    owner.update(cx, |workbench, cx| {
        workbench.select_diagram(diagram_id.to_owned(), cx);
        let change = workbench.presentation.clear_selection();
        workbench.apply_selection_change(change, cx);
    });
    let editable = !owner.read(cx).source().is_read_only()
        && !owner.read(cx).source().is_presentation_suspended();
    let snapshot = owner.read(cx).source().snapshot().ok();
    let diagram = snapshot
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
        })
        .cloned();
    let owner = owner.clone();
    let diagram_id = diagram_id.to_owned();
    ui::ContextMenu::build(window, cx, move |menu, _, _| {
        let open_owner = owner.clone();
        let open_id = diagram_id.clone();
        let menu = menu.entry("Open diagram", None, move |_, cx| {
            open_owner.update(cx, |workbench, cx| {
                workbench.select_diagram(open_id.clone(), cx);
                let change = workbench.presentation.clear_selection();
                workbench.apply_selection_change(change, cx);
            });
        });
        if !editable {
            return menu.header("Read-only preview");
        }
        let (Some(snapshot), Some(diagram)) = (snapshot, diagram) else {
            return menu.header("Diagram unavailable");
        };
        let menu = explorer_move_menu(
            menu,
            &owner,
            &snapshot,
            crate::components::model_explorer::ExplorerMoveTarget::Diagram(diagram_id.clone()),
            panel.clone(),
        );
        [true, false].into_iter().fold(menu, |menu, rename| {
            let panel = panel.clone();
            let owner = owner.clone();
            let project_id = snapshot.project_id.clone();
            let diagram_id = diagram_id.clone();
            let name = diagram.name.clone();
            let revision = snapshot.revision;
            menu.entry(
                if rename {
                    "Rename diagram"
                } else {
                    "Delete diagram…"
                },
                None,
                move |window, cx| {
                    if let Err(error) = panel.update(cx, |panel, cx| {
                        if !panel.is_bound_to(&owner) {
                            return;
                        }
                        let input = rename.then(|| {
                            cx.new(|cx| {
                                let input = InputField::new(window, cx, "Diagram name")
                                    .label_min_width(px(0.0));
                                input.set_text(&name, window, cx);
                                input
                            })
                        });
                        if let Some(input) = &input {
                            window.focus(&input.read(cx).focus_handle(cx), cx);
                        }
                        panel.explorer_action = Some(ExplorerActionDraft {
                            item_id: owner.entity_id(),
                            project_id: project_id.clone(),
                            target: ExplorerActionTarget::Diagram(diagram_id.clone()),
                            revision,
                            name: name.clone(),
                            rename: input,
                        });
                        panel.explorer_open = true;
                        cx.notify();
                    }) {
                        eprintln!("MDE diagram action failed: {error}");
                    }
                },
            )
        })
    })
}

pub(crate) struct PanelState {
    pub(crate) item: Option<gpui::WeakEntity<MdeWorkbenchItem>>,
    pub(crate) focus: FocusHandle,
    pub(crate) position: DockPosition,
    pub(crate) top_height: f32,
    split_start: Option<(Pixels, f32)>,
    _listener: Option<Subscription>,
}
impl PanelState {
    pub(crate) fn new<T: 'static>(
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
    pub(crate) fn bind<T: 'static>(
        &mut self,
        item: Option<&Entity<MdeWorkbenchItem>>,
        cx: &mut Context<T>,
    ) {
        self._listener = item.map(|item| cx.observe(item, |_, _, cx| cx.notify()));
        self.item = item.map(Entity::downgrade);
        self.split_start = None;
        cx.notify();
    }

    pub(crate) fn start_split(&mut self, y: Pixels) {
        self.split_start = Some((y, self.top_height));
    }
    pub(crate) fn move_split(&mut self, y: Pixels, dragging: bool) -> bool {
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
    pub(crate) fn finish_split(&mut self) {
        self.split_start = None;
    }
}
pub struct MdeRightPanel {
    pub(crate) state: PanelState,
    pub(crate) editing_id: Option<String>,
    explorer_action: Option<ExplorerActionDraft>,
    pub(crate) property_drafts: HashMap<
        crate::view_model::inspector_drafts::DraftKey,
        crate::components::inspector::InspectorDraft,
    >,
    pub(crate) style_target_id: Option<crate::components::style_editor::StyleKey>,
    pub(crate) style_drafts: HashMap<
        crate::components::style_editor::StyleKey,
        crate::components::style_editor::StyleDraft,
    >,
    pub(crate) inspector_styles_open: bool,
    pub(crate) timing_drafts:
        HashMap<(String, String, String), crate::components::timing_editor::TimingDraft>,
    pub(crate) collapsed_elements: HashSet<String>,
    pub(crate) last_revealed_row: Option<(gpui::EntityId, Vec<String>)>,
    pub(crate) explorer_scroll: gpui::ScrollHandle,
    pub(crate) collapsed_by_document: HashMap<gpui::EntityId, HashSet<String>>,
    pub(crate) explorer_open: bool,
    pub(crate) explorer_search_open: bool,
    pub(crate) explorer_search: Entity<InputField>,
    pub(crate) explorer_sort: ExplorerSortOrder,
    pub(crate) _explorer_search_listener: Subscription,
    pub(crate) inspector_open: bool,
    pub(crate) inspector_documentation_open: bool,
    pub(crate) inspector_advanced_open: bool,
}
impl MdeRightPanel {
    fn reveal_explorer_selection(
        &mut self,
        tree: &[bellatrix_mde_cordis::ExplorerRow],
        selected_index: Option<usize>,
        root_key: &str,
        item_id: gpui::EntityId,
        searching: bool,
    ) -> bool {
        let selected = selected_index.and_then(|index| tree.get(index).map(|row| (index, row)));
        let reveal = selected.map(|(index, row)| {
            let mut path = vec![row.key.clone()];
            let mut depth = row.depth;
            for ancestor in tree[..index].iter().rev() {
                if ancestor.depth < depth {
                    path.push(ancestor.key.clone());
                    depth = ancestor.depth;
                }
                if depth == 0 {
                    break;
                }
            }
            path.push(root_key.to_owned());
            (item_id, path)
        });
        if searching || reveal == self.last_revealed_row {
            return false;
        }
        if let Some((_, path)) = &reveal {
            for key in path.iter().skip(1) {
                self.collapsed_elements.remove(key);
            }
            if let Some((_, row)) = selected.filter(|(_, row)| row.diagram) {
                self.collapsed_elements.remove(&row.key);
            }
        }
        self.last_revealed_row = reveal;
        selected.is_some()
    }

    pub(crate) fn bound_item(&self) -> Option<Entity<MdeWorkbenchItem>> {
        self.state.item.as_ref().and_then(gpui::WeakEntity::upgrade)
    }

    pub(crate) fn is_bound_to(&self, item: &Entity<MdeWorkbenchItem>) -> bool {
        self.bound_item()
            .is_some_and(|bound| bound.entity_id() == item.entity_id())
    }

    pub(crate) fn commit_explorer_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.explorer_action.as_ref() else {
            return;
        };
        let Some(owner) = self.state.item.as_ref().and_then(gpui::WeakEntity::upgrade) else {
            return;
        };
        if owner.entity_id() != draft.item_id {
            self.explorer_action = None;
            return;
        }
        let name = if let Some(input) = &draft.rename {
            let name = input.read(cx).text(cx).trim().to_owned();
            if name.is_empty() {
                owner.update(cx, |workbench, cx| {
                    workbench.feedback = "Name cannot be blank".into();
                    cx.notify();
                });
                return;
            }
            Some(name)
        } else if matches!(&draft.target, ExplorerActionTarget::Diagram(_)) {
            None
        } else {
            return;
        };
        if owner.update(cx, |workbench, cx| {
            workbench.submit_explorer_operation(
                &draft.project_id,
                draft.revision,
                |source| match (&draft.target, name) {
                    (ExplorerActionTarget::Diagram(id), Some(name)) => {
                        source.rename_diagram(id, name, draft.revision)
                    }
                    (ExplorerActionTarget::Element(id), Some(name)) => {
                        source.rename_element(id, name, draft.revision)
                    }
                    (ExplorerActionTarget::Diagram(id), None) => {
                        source.delete_diagram(id, draft.revision)
                    }
                    _ => Err("Unsupported Explorer action".into()),
                },
                cx,
            )
        }) {
            self.explorer_action = None;
            window.focus(&self.state.focus, cx);
        }
        cx.notify();
    }

    pub fn new(
        item: Entity<MdeWorkbenchItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let explorer_search = cx.new(|cx| {
            InputField::new(window, cx, "Search model elements…")
                .start_icon(IconName::MagnifyingGlass)
                .label_min_width(px(0.0))
        });
        let owner = cx.weak_entity();
        let editor = explorer_search.read(cx).editor().clone();
        let explorer_search_listener = editor.subscribe(
            Box::new(move |event, _, cx| {
                if event == ErasedEditorEvent::BufferEdited {
                    if let Err(error) = owner.update(cx, |panel, cx| {
                        panel.explorer_scroll.scroll_to_item(0);
                        cx.notify();
                    }) {
                        eprintln!("MDE Explorer search failed: {error}");
                    }
                }
            }),
            window,
            cx,
        );
        Self {
            state: PanelState::new(item, DockPosition::Right, 230.0, cx),
            editing_id: None,
            explorer_action: None,
            property_drafts: HashMap::new(),
            style_drafts: HashMap::new(),
            style_target_id: None,
            inspector_styles_open: true,
            timing_drafts: HashMap::new(),
            collapsed_elements: HashSet::new(),
            last_revealed_row: None,
            explorer_scroll: gpui::ScrollHandle::new(),
            collapsed_by_document: HashMap::new(),
            explorer_open: true,
            explorer_search_open: false,
            explorer_search,
            explorer_sort: ExplorerSortOrder::Added,
            _explorer_search_listener: explorer_search_listener,
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
        if self
            .explorer_action
            .as_ref()
            .is_some_and(|draft| draft.item_id != item.entity_id())
        {
            self.explorer_action = None;
        }
        let read_only = item.read(cx).source().is_read_only();
        let palette = WorkbenchPalette::current(cx);
        let panel_background = palette.panel;
        let panel_border = palette.border;
        let selected_id = item.read(cx).selected_element().map(str::to_owned);
        let active_diagram = item.read(cx).active_diagram().map(str::to_owned);
        let selected_view = item
            .read(cx)
            .presentation
            .selected_view()
            .map(str::to_owned);
        let selected_explorer_key = item
            .read(cx)
            .presentation
            .selected_explorer_key()
            .map(str::to_owned);
        let snapshot = item.read(cx).source().presentation_snapshot().ok();
        let inspection = snapshot.as_ref().and_then(|snapshot| {
            selected_id
                .as_deref()
                .and_then(|id| snapshot.section.inspect_element(id))
        });
        let explorer_query = self.explorer_search.read(cx).text(cx).trim().to_lowercase();
        let searching = !explorer_query.is_empty();
        let mut explorer_has_rows = false;
        let mut rows = Vec::new();
        let mut explorer_branch_keys = Vec::new();
        let mut can_reveal_selection = false;
        if let Some(snapshot) = &snapshot {
            let full_tree = project_explorer(&snapshot.section, self.explorer_sort);
            let tree = filter_explorer(full_tree, &explorer_query);
            explorer_has_rows = !tree.is_empty();
            let root_key = format!("project:{}", snapshot.project_id.as_str());
            explorer_branch_keys.push(root_key.clone());
            explorer_branch_keys.extend(
                tree.windows(2)
                    .filter(|rows| rows[1].depth > rows[0].depth)
                    .map(|rows| rows[0].key.clone()),
            );
            let selected_row = selected_explorer_key
                .as_ref()
                .and_then(|key| tree.iter().find(|row| &row.key == key))
                .or_else(|| {
                    selected_view.as_ref().and_then(|id| {
                        tree.iter().find(|row| {
                            row.view.as_ref().is_some_and(|view| {
                                &view.view_id == id
                                    && active_diagram.as_deref() == Some(view.diagram_id.as_str())
                            })
                        })
                    })
                })
                .or_else(|| {
                    selected_id.as_ref().and_then(|id| {
                        tree.iter()
                            .find(|row| {
                                row.view.is_none()
                                    && !row.diagram
                                    && &row.element_id == id
                                    && (row.diagram_context.is_none()
                                        || row.diagram_context.as_deref()
                                            == active_diagram.as_deref())
                            })
                            .or_else(|| {
                                tree.iter().find(|row| {
                                    row.view.as_ref().is_some_and(|view| {
                                        view.subject_id.as_ref() == Some(id)
                                            && active_diagram.as_deref()
                                                == Some(view.diagram_id.as_str())
                                    })
                                })
                            })
                            .or_else(|| {
                                tree.iter().find(|row| {
                                    row.view.is_none() && !row.diagram && &row.element_id == id
                                })
                            })
                            .or_else(|| {
                                tree.iter().find(|row| {
                                    row.view
                                        .as_ref()
                                        .is_some_and(|view| view.subject_id.as_ref() == Some(id))
                                })
                            })
                    })
                })
                .or_else(|| {
                    active_diagram
                        .as_ref()
                        .and_then(|id| tree.iter().find(|row| row.diagram && &row.element_id == id))
                });
            can_reveal_selection = selected_id.is_some() || active_diagram.is_some();
            let selected_index = selected_row
                .and_then(|selected| tree.iter().position(|row| row.key == selected.key));
            let reveal_selected = self.reveal_explorer_selection(
                &tree,
                selected_index,
                &root_key,
                item.entity_id(),
                searching,
            );
            let root_expanded = searching || !self.collapsed_elements.contains(&root_key);
            let root_panel = cx.weak_entity();
            let root_disclosure_key = root_key.clone();
            let project_menu_owner = item.clone();
            let project_menu_panel = cx.weak_entity();
            let project_menu_key = root_key.clone();
            let project_row_key = root_key.clone();
            let project_row_name = snapshot.name.clone();
            let project_has_children = !tree.is_empty();
            let root_filter_owner = item.clone();
            let root_element_owner = item.clone();
            let root_diagram_owner = item.clone();
            let root_element_parent = snapshot.section.root_model.clone();
            rows.push(div().id("mde-explorer-project").flex().items_center().gap_1()
                .can_drop(move |drag, _, cx| {
                    let project_id = if let Some(drag) = drag.downcast_ref::<crate::components::model_explorer::ExplorerElementDrag>() {
                        &drag.project_id
                    } else if let Some(drag) = drag.downcast_ref::<crate::components::model_explorer::ExplorerDiagramDrag>() {
                        &drag.project_id
                    } else { return false; };
                    let workbench = root_filter_owner.read(cx);
                    workbench.project_id() == project_id
                        && !workbench.source.is_read_only()
                        && !workbench.source.is_presentation_suspended()
                })
                .on_drop(move |drag: &crate::components::model_explorer::ExplorerElementDrag, _, cx| {
                    root_element_owner.update(cx, |workbench, cx| workbench.move_explorer_item(
                        &drag.project_id, drag.revision,
                        &crate::components::model_explorer::ExplorerMoveTarget::Element(drag.element_id.clone()),
                        root_element_parent.clone(), None, cx));
                })
                .on_drop(move |drag: &crate::components::model_explorer::ExplorerDiagramDrag, _, cx| {
                    root_diagram_owner.update(cx, |workbench, cx| workbench.move_explorer_item(
                        &drag.project_id, drag.revision,
                        &crate::components::model_explorer::ExplorerMoveTarget::Diagram(drag.diagram_id.clone()),
                        None, None, cx));
                })
                .child(crate::components::model_explorer::explorer_project_icon(&palette))
                .child(ui::right_click_menu("mde-explorer-project-menu")
                    .trigger(move |_, _, _| TreeViewItem::new(project_row_key, project_row_name)
                    .root_item(project_has_children).expanded(root_expanded)
                    .on_toggle(move |_, _, cx| {
                        if searching { cx.stop_propagation(); return; }
                        if let Err(error) = root_panel.update(cx, |panel, cx| {
                            if !panel.collapsed_elements.insert(root_disclosure_key.clone()) { panel.collapsed_elements.remove(&root_disclosure_key); }
                            cx.notify();
                        }) { eprintln!("MDE project expansion failed: {error}"); }
                    }))
                    .menu(move |window, cx| explorer_project_menu(&project_menu_owner, project_menu_panel.clone(), &project_menu_key, window, cx))));
            let mut hidden_by_depth: Option<usize> = None;
            for (index, row) in tree.iter().enumerate() {
                if !root_expanded {
                    break;
                }
                if let Some(ancestor_depth) = hidden_by_depth {
                    if row.depth > ancestor_depth {
                        continue;
                    }
                    hidden_by_depth = None;
                }
                let has_children = tree
                    .get(index + 1)
                    .is_some_and(|next| next.depth > row.depth);
                let type_label = crate::components::model_explorer::explorer_type_label(
                    &snapshot.section,
                    &row.kind,
                    snapshot.section.relations.iter().any(|relation| {
                        row.view
                            .as_ref()
                            .and_then(|view| view.subject_id.as_deref())
                            .unwrap_or(&row.element_id)
                            == relation.id
                    }),
                );
                let drag_subject = row
                    .view
                    .as_ref()
                    .and_then(|view| view.subject_id.as_deref())
                    .or_else(|| {
                        (!row.diagram && row.view.is_none()).then_some(row.element_id.as_str())
                    });
                let element_drag = if read_only {
                    None
                } else {
                    drag_subject
                        .filter(|id| {
                            snapshot
                                .section
                                .elements
                                .iter()
                                .any(|element| element.id == *id)
                        })
                        .map(
                            |element_id| crate::components::model_explorer::ExplorerElementDrag {
                                project_id: snapshot.project_id.clone(),
                                element_id: element_id.to_owned(),
                                target_diagram_id: active_diagram.clone(),
                                source_view_id: row
                                    .view
                                    .as_ref()
                                    .filter(|view| {
                                        active_diagram.as_deref() == Some(view.diagram_id.as_str())
                                    })
                                    .map(|view| view.view_id.clone()),
                                revision: snapshot.revision,
                                label: row.label.clone(),
                            },
                        )
                };
                let ownership_target = if read_only {
                    None
                } else {
                    drag_subject
                        .filter(|id| {
                            snapshot
                                .section
                                .elements
                                .iter()
                                .any(|element| element.id == *id)
                        })
                        .map(str::to_owned)
                };
                let ownership_occurrence = crate::view_model::explorer::parent_occurrence_from_rows(
                    &snapshot.section,
                    ownership_target.as_deref(),
                    &row.key,
                    &tree,
                );
                let ownership_filter_owner = item.clone();
                let ownership_drop_owner = item.clone();
                if let Some(view) = &row.view {
                    let diagram_id = view.diagram_id.clone();
                    let view_id = view.view_id.clone();
                    let owner = item.clone();
                    let selected = selected_row.is_some_and(|selected| selected.key == row.key);
                    if selected && reveal_selected {
                        self.explorer_scroll.scroll_to_item(rows.len());
                    }
                    let expanded = searching || !self.collapsed_elements.contains(&row.key);
                    let disclosure_id = row.key.clone();
                    let disclosure_panel = cx.weak_entity();
                    let menu_owner = item.clone();
                    let menu_subject = view.subject_id.clone();
                    let member_menu_panel = cx.weak_entity();
                    let member_menu_key = row.key.clone();
                    let menu_diagram = view.diagram_id.clone();
                    let menu_view = view.view_id.clone();
                    let view_row_key = row.key.clone();
                    let view_row_label = row.label.clone();
                    let view_accessible_label = format!("{}: {}", type_label, row.label);
                    rows.push(div().id(format!("mde-explorer-view-{}", row.key))
                        .when_some(element_drag, |row, drag| row.on_drag(drag, |drag: &crate::components::model_explorer::ExplorerElementDrag, _, _, cx| cx.new(|_| drag.clone())))
                        .when_some(ownership_target, |row, target| {
                            let diagram_target = target.clone();
                            let diagram_occurrence = ownership_occurrence.clone();
                            let diagram_hover_occurrence = diagram_occurrence.clone();
                            let diagram_hover_target = target.clone();
                            let diagram_hover_owner = ownership_filter_owner.clone();
                            let diagram_drop_owner = ownership_drop_owner.clone();
                            let hover_target = target.clone();
                            let hover_owner = ownership_filter_owner.clone();
                            let highlight = palette.selection;
                            row.drag_over::<crate::components::model_explorer::ExplorerElementDrag>(move |style, drag, _, cx| {
                                if hover_owner.read(cx).validate_explorer_reparent(drag, &hover_target).is_ok() {
                                    style.border_1().border_color(highlight)
                                } else {
                                    style
                                }
                            }).drag_over::<crate::components::model_explorer::ExplorerDiagramDrag>(move |style, drag, _, cx| {
                                if diagram_hover_owner.read(cx).validate_explorer_diagram_reparent(drag, Some(&diagram_hover_target), diagram_hover_occurrence.as_ref()).is_ok() {
                                    style.border_1().border_color(highlight)
                                } else { style }
                            }).can_drop(move |drag, _, cx| {
                                if let Some(drag) = drag.downcast_ref::<crate::components::model_explorer::ExplorerDiagramDrag>() {
                                    let workbench = ownership_filter_owner.read(cx);
                                    return workbench.project_id() == &drag.project_id
                                        && !workbench.source.is_read_only()
                                        && !workbench.source.is_presentation_suspended();
                                }
                                // GPUI checks every row here, including unhovered rows.
                                // Hover previews use the kernel; Cordis validates the submitted drop.
                                drag.downcast_ref::<crate::components::model_explorer::ExplorerElementDrag>()
                                    .is_some_and(|drag| {
                                        let workbench = ownership_filter_owner.read(cx);
                                        workbench.project_id() == &drag.project_id
                                            && !workbench.source.is_read_only()
                                            && !workbench.source.is_presentation_suspended()
                                    })
                            }).on_drop(move |drag: &crate::components::model_explorer::ExplorerElementDrag, _, cx| {
                                ownership_drop_owner.update(cx, |workbench, cx| {
                                    workbench.reparent_explorer_element(drag, target.clone(), cx)
                                });
                            }).on_drop(move |drag: &crate::components::model_explorer::ExplorerDiagramDrag, _, cx| {
                                diagram_drop_owner.update(cx, |workbench, cx| workbench.move_explorer_item(
                                    &drag.project_id, drag.revision,
                                    &crate::components::model_explorer::ExplorerMoveTarget::Diagram(drag.diagram_id.clone()),
                                    Some(diagram_target.clone()), diagram_occurrence.clone(), cx));
                            })
                        })
                        .tooltip(ui::Tooltip::text(format!("Visual component · {type_label}. Drag onto an owner to move its semantic element; the view stays in this diagram.")))
                        .pl(px((row.depth.min(15) + 2) as f32 * 14.0)).flex().items_center().gap_1()
                        .when(selected, |row| row.bg(palette.surface))
                        .child(crate::components::model_explorer::explorer_view_icon(&row.key, &row.kind, &snapshot.section.identity(&row.kind), selected, &palette))
                        .child(div().flex_1().min_w_0().overflow_hidden().child(
                            ui::right_click_menu(format!("mde-explorer-view-menu-{}", row.key))
                            .trigger(move |_, _, _| TreeViewItem::new(view_row_key, view_row_label)
                                .aria_label(view_accessible_label)
                                .root_item(has_children).expanded(expanded).toggle_state(selected)
                                .on_toggle(move |_, _, cx| {
                                    if searching { cx.stop_propagation(); return; }
                                    cx.stop_propagation();
                                    if let Err(error) = disclosure_panel.update(cx, |panel, cx| {
                                        if !panel.collapsed_elements.insert(disclosure_id.clone()) { panel.collapsed_elements.remove(&disclosure_id); }
                                        cx.notify();
                                    }) { eprintln!("MDE Explorer expansion failed: {error}"); }
                                })
                                .on_click(move |_, _, cx| {
                                    owner.update(cx, |workbench, cx| workbench.select_diagram_view(diagram_id.clone(), view_id.clone(), cx));
                                }))
                            .menu(move |window, cx| {
                                menu_owner.update(cx, |workbench, cx| {
                                    workbench.select_diagram_view(menu_diagram.clone(), menu_view.clone(), cx);
                                });
                                if let Some(subject) = &menu_subject {
                                    explorer_menu(&menu_owner, subject, Some((menu_diagram.clone(), menu_view.clone())), member_menu_panel.clone(), member_menu_key.clone(), window, cx)
                                } else {
                                    let remove_owner = menu_owner.clone();
                                    let remove_diagram = menu_diagram.clone();
                                    let remove_view = menu_view.clone();
                                    let read_only = remove_owner.read(cx).source().is_read_only()
                                        || remove_owner.read(cx).source().is_presentation_suspended();
                                    ui::ContextMenu::build(window, cx, move |menu, _, _| {
                                        let menu = menu.header("Visual component");
                                        if read_only {
                                            return menu.header("Read-only preview");
                                        }
                                        menu.entry("Remove from diagram", None, move |_, cx| {
                                            remove_owner.update(cx, |workbench, cx| {
                                                workbench.remove_view(remove_diagram.clone(), remove_view.clone(), cx)
                                            });
                                        })
                                    })
                                }
                            })
                        )));
                    if has_children && !expanded {
                        hidden_by_depth = Some(row.depth);
                    }
                    continue;
                }
                if row.diagram {
                    let diagram_drag = (!read_only).then(|| {
                        crate::components::model_explorer::ExplorerDiagramDrag {
                            project_id: snapshot.project_id.clone(),
                            diagram_id: row.element_id.clone(),
                            revision: snapshot.revision,
                            label: row.label.clone(),
                        }
                    });
                    let id = row.element_id.clone();
                    let owner = item.clone();
                    let selected = selected_row.is_some_and(|selected| selected.key == row.key);
                    let expanded = searching || !self.collapsed_elements.contains(&row.key);
                    let disclosure_id = row.key.clone();
                    let disclosure_panel = cx.weak_entity();
                    let selection_panel = cx.weak_entity();
                    let selection_key = row.key.clone();
                    let selection_root = root_key.clone();
                    let secondary_owner = item.clone();
                    let secondary_diagram = id.clone();
                    let menu_owner = item.clone();
                    let menu_diagram = id.clone();
                    let menu_panel = cx.weak_entity();
                    let diagram_row_key = row.key.clone();
                    let diagram_row_label = row.label.clone();
                    let diagram_element_drop_owner = item.clone();
                    let diagram_element_drop_target = id.clone();
                    let visual_menu_owner = item.clone();
                    let visual_menu_diagram = id.clone();
                    let visual_menu_id = format!("mde-unbound-views-{id}");
                    let has_unbound_views = snapshot
                        .section
                        .diagrams
                        .iter()
                        .find(|diagram| diagram.id == id)
                        .is_some_and(|diagram| {
                            diagram.all_views().any(|view| view.subject.is_none())
                        });
                    if reveal_selected
                        && selected_row.is_some_and(|selected| selected.key == row.key)
                    {
                        self.explorer_scroll.scroll_to_item(rows.len());
                    }
                    rows.push(
                        div()
                            .id(format!("mde-explorer-diagram-{id}"))
                            .when_some(diagram_drag, |row, drag| row.on_drag(drag, |drag: &crate::components::model_explorer::ExplorerDiagramDrag, _, _, cx| cx.new(|_| drag.clone())))
                            .on_drop(move |drag: &crate::components::model_explorer::ExplorerElementDrag, _, cx| {
                                diagram_element_drop_owner.update(cx, |workbench, cx| {
                                    workbench.drop_explorer_element_on_diagram(drag, &diagram_element_drop_target, cx);
                                });
                            })
                            .tooltip(ui::Tooltip::text("Diagram. Drop a model element here to show a view; its owner and other views are preserved."))
                            .pl(px((row.depth.min(15) + 2) as f32 * 14.0))
                            .flex().items_center().gap_1()
                            .when(selected, |row| row.bg(palette.surface))
                            .child(crate::components::model_explorer::explorer_diagram_icon(&row.kind, &palette))
                            .child(div().flex_1().min_w_0().overflow_hidden().child(
                                ui::right_click_menu(format!("mde-explorer-diagram-menu-{id}"))
                                .trigger(move |_, _, _| TreeViewItem::new(diagram_row_key, diagram_row_label.clone())
                                    .aria_label(format!("Diagram: {}", diagram_row_label))
                                    .root_item(has_children).expanded(expanded)
                                    .on_toggle(move |_, _, cx| {
                                        if searching { cx.stop_propagation(); return; }
                                        cx.stop_propagation();
                                        if let Err(error) = disclosure_panel.update(cx, |panel, cx| {
                                            if !panel.collapsed_elements.insert(disclosure_id.clone()) { panel.collapsed_elements.remove(&disclosure_id); }
                                            cx.notify();
                                        }) {
                                            eprintln!("MDE diagram expansion failed: {error}");
                                        }
                                    })
                                    .toggle_state(selected)
                                    .on_secondary_mouse_down(move |_, _, cx| {
                                        secondary_owner.update(cx, |workbench, cx| {
                                            workbench.select_diagram(secondary_diagram.clone(), cx);
                                            let change = workbench.presentation.clear_selection();
                                            workbench.apply_selection_change(change, cx);
                                        });
                                    })
                                    .on_click(move |_, _, cx| {
                                        if let Err(error) = selection_panel.update(cx, |panel, cx| {
                                            panel.collapsed_elements.remove(&selection_root);
                                            panel.collapsed_elements.remove(&selection_key);
                                            cx.notify();
                                        }) {
                                            eprintln!("MDE diagram reveal failed: {error}");
                                        }
                                        owner.update(cx, |workbench, cx| {
                                            workbench.select_diagram(id.clone(), cx);
                                            let change = workbench.presentation.clear_selection();
                                            workbench.apply_selection_change(change, cx);
                                        });
                                    }))
                                .menu(move |window, cx| explorer_diagram_menu(&menu_owner, menu_panel.clone(), &menu_diagram, window, cx))
                            ))
                            .when(has_unbound_views, |row| row.child(
                                ui::PopoverMenu::new(visual_menu_id.clone())
                                    .trigger_with_tooltip(
                                        IconButton::new(format!("{visual_menu_id}-trigger"), IconName::Crosshair)
                                            .aria_label("Navigate visual drafts and unbound views"),
                                        ui::Tooltip::text("Navigate visual drafts and unbound views; separate from model hierarchy"),
                                    )
                                    .menu(move |window, cx| Some(explorer_unbound_views_menu(
                                        &visual_menu_owner, &visual_menu_diagram, window, cx,
                                    )))
                            )),
                    );
                    if has_children && !expanded {
                        hidden_by_depth = Some(row.depth);
                    }
                    continue;
                }
                let expanded = searching || !self.collapsed_elements.contains(&row.key);
                let id = row.element_id.clone();
                let owner = item.clone();
                let diagram_context = row.diagram_context.clone();
                let selection_key = row.key.clone();
                let selection_occurrence = ownership_occurrence.clone();
                let menu_selection_key = row.key.clone();
                let menu_selection_occurrence = ownership_occurrence.clone();
                let disclosure_id = row.key.clone();
                let disclosure_panel = cx.weak_entity();
                let menu_owner = item.clone();
                let menu_subject = id.clone();
                let member_menu_panel = cx.weak_entity();
                let member_menu_key = row.key.clone();
                let menu_diagram_context = row.diagram_context.clone();
                let selected = selected_row.is_some_and(|selected| selected.key == row.key);
                if selected && reveal_selected {
                    self.explorer_scroll.scroll_to_item(rows.len());
                }
                let tree_item = TreeViewItem::new(row.key.clone(), row.label.clone())
                    .aria_label(format!("{}: {}", type_label, row.label))
                    .root_item(has_children)
                    .expanded(expanded)
                    .toggle_state(selected)
                    .on_toggle(move |_, _, cx| {
                        if searching {
                            cx.stop_propagation();
                            return;
                        }
                        cx.stop_propagation();
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
                        owner.update(cx, |workbench, cx| {
                            if let Some(diagram) = &diagram_context {
                                workbench.select_diagram(diagram.clone(), cx);
                            }
                            workbench.select_explorer_element(
                                id.clone(),
                                selection_key.clone(),
                                selection_occurrence.clone(),
                                cx,
                            );
                        });
                    });
                rows.push(
                    div()
                        .id(format!("mde-explorer-row-{}", row.key))
                        .when_some(element_drag, |row, drag| row.on_drag(drag, |drag: &crate::components::model_explorer::ExplorerElementDrag, _, _, cx| cx.new(|_| drag.clone())))
                        .when_some(ownership_target, |row, target| {
                            let diagram_target = target.clone();
                            let diagram_occurrence = ownership_occurrence.clone();
                            let diagram_hover_occurrence = diagram_occurrence.clone();
                            let diagram_hover_target = target.clone();
                            let diagram_hover_owner = ownership_filter_owner.clone();
                            let diagram_drop_owner = ownership_drop_owner.clone();
                            let hover_target = target.clone();
                            let hover_owner = ownership_filter_owner.clone();
                            let highlight = palette.selection;
                            row.drag_over::<crate::components::model_explorer::ExplorerElementDrag>(move |style, drag, _, cx| {
                                if hover_owner.read(cx).validate_explorer_reparent(drag, &hover_target).is_ok() {
                                    style.border_1().border_color(highlight)
                                } else {
                                    style
                                }
                            }).drag_over::<crate::components::model_explorer::ExplorerDiagramDrag>(move |style, drag, _, cx| {
                                if diagram_hover_owner.read(cx).validate_explorer_diagram_reparent(drag, Some(&diagram_hover_target), diagram_hover_occurrence.as_ref()).is_ok() {
                                    style.border_1().border_color(highlight)
                                } else { style }
                            }).can_drop(move |drag, _, cx| {
                                if let Some(drag) = drag.downcast_ref::<crate::components::model_explorer::ExplorerDiagramDrag>() {
                                    let workbench = ownership_filter_owner.read(cx);
                                    return workbench.project_id() == &drag.project_id
                                        && !workbench.source.is_read_only()
                                        && !workbench.source.is_presentation_suspended();
                                }
                                // GPUI checks every row here, including unhovered rows.
                                // Hover previews use the kernel; Cordis validates the submitted drop.
                                drag.downcast_ref::<crate::components::model_explorer::ExplorerElementDrag>()
                                    .is_some_and(|drag| {
                                        let workbench = ownership_filter_owner.read(cx);
                                        workbench.project_id() == &drag.project_id
                                            && !workbench.source.is_read_only()
                                            && !workbench.source.is_presentation_suspended()
                                    })
                            }).on_drop(move |drag: &crate::components::model_explorer::ExplorerElementDrag, _, cx| {
                                ownership_drop_owner.update(cx, |workbench, cx| {
                                    workbench.reparent_explorer_element(drag, target.clone(), cx)
                                });
                            }).on_drop(move |drag: &crate::components::model_explorer::ExplorerDiagramDrag, _, cx| {
                                diagram_drop_owner.update(cx, |workbench, cx| workbench.move_explorer_item(
                                    &drag.project_id, drag.revision,
                                    &crate::components::model_explorer::ExplorerMoveTarget::Diagram(drag.diagram_id.clone()),
                                    Some(diagram_target.clone()), diagram_occurrence.clone(), cx));
                            })
                        })
                        .tooltip(ui::Tooltip::text(format!("Model element · {type_label}. Moving this element changes semantic ownership and preserves its diagram views.")))
                        .pl(px((row.depth.min(15) + 2) as f32 * 14.0))
                        .flex()
                        .items_center()
                        .gap_1()
                        .when(selected, |row| row.bg(palette.surface))
                        .child(
                            crate::components::model_explorer::explorer_element_icon(&row.kind, &snapshot.section.identity(&row.kind), &palette),
                        )
                        .child(
                            div().flex_1().min_w_0().overflow_hidden().child(
                                ui::right_click_menu(format!(
                                    "mde-explorer-menu-{}",
                                    row.key
                                ))
                                .trigger(move |_, _, _| tree_item)
                                .menu(move |window, cx| {
                                    menu_owner.update(cx, |workbench, cx| {
                                        if let Some(diagram) = &menu_diagram_context {
                                            workbench.select_diagram(diagram.clone(), cx);
                                        }
                                        workbench.select_explorer_element(menu_subject.clone(), menu_selection_key.clone(), menu_selection_occurrence.clone(), cx);
                                    });
                                    explorer_menu(&menu_owner, &menu_subject, None, member_menu_panel.clone(), member_menu_key.clone(), window, cx)
                                }),
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
            crate::components::inspector::reconcile_inputs(
                &mut self.property_drafts,
                item.clone(),
                inspector_project.as_str(),
                inspection.as_ref(),
                snapshot.as_ref().map_or(0, |snapshot| snapshot.revision),
                window,
                cx,
            );
        }
        let mut inspector_rows = self.render_view_presentation(
            item.clone(),
            &snapshot,
            &active_diagram,
            read_only,
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
                            .action(div().flex().items_center().gap_1()
                                .child(IconButton::new("mde-explorer-search", IconName::MagnifyingGlass)
                                    .aria_label("Search Explorer")
                                    .aria_expanded(self.explorer_search_open && self.explorer_open)
                                    .toggle_state(self.explorer_search_open)
                                    .tooltip(ui::Tooltip::text("Search Explorer"))
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.explorer_search_open = !panel.explorer_search_open;
                                        panel.explorer_open = true;
                                        if panel.explorer_search_open {
                                            window.focus(&panel.explorer_search.focus_handle(cx), cx);
                                        } else {
                                            let editor = panel.explorer_search.read(cx).editor().clone();
                                            editor.clear(window, cx);
                                            window.focus(&panel.state.focus, cx);
                                        }
                                        cx.notify();
                                    })))
                                .child(IconButton::new("mde-explorer-sort", IconName::ArrowDown10)
                                    .aria_label(if self.explorer_sort == ExplorerSortOrder::Added { "Sort Explorer by name" } else { "Sort Explorer by added order" })
                                    .tooltip(ui::Tooltip::text(if self.explorer_sort == ExplorerSortOrder::Added { "Sort by name (currently added order)" } else { "Sort by added order (currently name)" }))
                                    .toggle_state(self.explorer_sort == ExplorerSortOrder::Name)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.explorer_sort = if panel.explorer_sort == ExplorerSortOrder::Added { ExplorerSortOrder::Name } else { ExplorerSortOrder::Added };
                                        panel.explorer_scroll.scroll_to_item(0);
                                        cx.notify();
                                    })))
                                .child(IconButton::new("mde-explorer-reveal", IconName::Crosshair)
                                    .aria_label("Reveal selection in Explorer")
                                    .tooltip(ui::Tooltip::text("Reveal selection in Explorer"))
                                    .disabled(!can_reveal_selection)
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        let editor = panel.explorer_search.read(cx).editor().clone();
                                        editor.clear(window, cx);
                                        panel.explorer_open = true;
                                        panel.last_revealed_row = None;
                                        cx.notify();
                                    })))
                                .child(IconButton::new("mde-explorer-expand-all", IconName::ExpandVertical)
                                    .aria_label("Expand all Explorer nodes")
                                    .disabled(searching)
                                    .tooltip(ui::Tooltip::text("Expand all Explorer nodes"))
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.explorer_open = true;
                                        panel.collapsed_elements.clear();
                                        cx.notify();
                                    })))
                                .child(IconButton::new("mde-explorer-collapse-all", IconName::ListCollapse)
                                    .aria_label("Collapse all Explorer nodes")
                                    .disabled(searching)
                                    .tooltip(ui::Tooltip::text("Collapse all Explorer nodes"))
                                    .on_click(cx.listener(move |panel, _, _, cx| {
                                        panel.collapsed_elements.extend(explorer_branch_keys.iter().cloned());
                                        cx.notify();
                                    })))
                                .when(!read_only, |actions| actions.child(
                                    IconButton::new("mde-save-project", IconName::Download)
                                        .aria_label("Save project as")
                                        .tooltip(ui::Tooltip::text("Save project as…"))
                                        .on_click(move |_, window, cx| {
                                            window.dispatch_action(
                                                Box::new(crate::routes::workspace::SaveMdeProjectAs), cx,
                                            );
                                        })
                                ))),
                    )
                    .when(self.explorer_open && !item.read(cx).feedback().is_empty(), |pane| {
                        let feedback = item.read(cx).feedback().to_owned();
                        pane.child(div().id("mde-explorer-feedback").px_2().py_1()
                            .flex_none().max_h(px(72.0)).overflow_y_scroll()
                            .whitespace_normal().child(Label::new(feedback).size(ui::LabelSize::Small)))
                    })
                    .when(self.explorer_open && self.explorer_search_open, |pane| {
                        pane.child(div().p_2().flex().items_center().gap_1()
                            .child(div().flex_1().min_w_0().child(self.explorer_search.clone()))
                            .child(IconButton::new("mde-explorer-clear-search", IconName::Close)
                                .aria_label("Clear Explorer search")
                                .tooltip(ui::Tooltip::text("Clear Explorer search"))
                                .disabled(!searching)
                                .on_click(cx.listener(|panel, _, window, cx| {
                                    let editor = panel.explorer_search.read(cx).editor().clone();
                                    editor.clear(window, cx);
                                    window.focus(&panel.explorer_search.focus_handle(cx), cx);
                                    cx.notify();
                                }))))
                    })
                    .when(self.explorer_open, |pane| {
                        pane.when_some(self.explorer_action.as_ref(), |pane, draft| {
                            let rename = draft.rename.is_some();
                            pane.child(div().p_2().flex().items_center().gap_1()
                                .capture_key_down(cx.listener(|panel, event: &gpui::KeyDownEvent, window, cx| {
                                    let input_focused = panel.explorer_action.as_ref().and_then(|draft| draft.rename.as_ref())
                                        .is_some_and(|input| input.read(cx).focus_handle(cx).contains_focused(window, cx));
                                    if !input_focused || event.keystroke.modifiers.modified() { return; }
                                    if event.keystroke.key.eq_ignore_ascii_case("enter") {
                                        panel.commit_explorer_action(window, cx);
                                        cx.stop_propagation();
                                        window.prevent_default();
                                    } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
                                        panel.explorer_action = None;
                                        window.focus(&panel.state.focus, cx);
                                        cx.notify();
                                        cx.stop_propagation();
                                        window.prevent_default();
                                    }
                                }))
                                .child(div().flex_1().min_w_0()
                                    .when_some(draft.rename.clone(), |row, input| row.child(input))
                                    .when(!rename, |row| row.child(Label::new(format!("Delete {} and its views? Model elements remain.", draft.name)).size(ui::LabelSize::Small))))
                                .child(IconButton::new("mde-diagram-action-confirm", if rename { IconName::Check } else { IconName::Trash })
                                    .aria_label(if rename { "Save name" } else { "Confirm diagram deletion" })
                                    .tooltip(ui::Tooltip::text(if rename { "Save name (Enter); cancel (Escape)" } else { "Delete diagram and its views; keep model elements" }))
                                    .disabled(read_only || item.read(cx).source().is_presentation_suspended())
                                    .on_click(cx.listener(|panel, _, window, cx| panel.commit_explorer_action(window, cx))))
                                .child(IconButton::new("mde-diagram-action-cancel", IconName::Close)
                                    .aria_label("Cancel Explorer action").tooltip(ui::Tooltip::text("Cancel"))
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.explorer_action = None;
                                        window.focus(&panel.state.focus, cx);
                                        cx.notify();
                                    }))))
                        }).child(
                            div()
                                .id("bellatrix-mde-explorer-scroll")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .track_scroll(&self.explorer_scroll)
                                .children(rows)
                                .when(
                                    snapshot.is_some() && !explorer_has_rows,
                                    |pane| {
                                        pane.child(
                                            div()
                                                .p_3()
                                                .text_xs()
                                                .text_color(palette.muted)
                                                .child(if searching { "No matching model elements." } else { "No model elements yet." }),
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
                                                IconButton::new("mde-delete-element", IconName::Trash)
                                                .aria_label("Delete from model")
                                                .tooltip(ui::Tooltip::text("Delete from model"))
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
    execution: Entity<crate::components::execution_dashboard::MdeExecutionDashboard>,
}
impl MdeBottomPanel {
    pub fn new(
        item: Entity<MdeWorkbenchItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let execution = cx.new(|cx| {
            crate::components::execution_dashboard::MdeExecutionDashboard::new(
                item.clone(),
                window,
                cx,
            )
        });
        Self {
            state: PanelState::new(item, DockPosition::Bottom, 0.0, cx),
            active: BottomTab::Diagnostics,
            execution,
        }
    }

    fn bind(&mut self, item: Option<&Entity<MdeWorkbenchItem>>, cx: &mut Context<Self>) {
        self.state.bind(item, cx);
        self.execution
            .update(cx, |execution, cx| execution.bind(item, cx));
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
        Some("Bellatrix MDE - Logs, Diagnostics, Relations, Markdown & Thumbnails (bottom dock)")
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
        for (index, (kind, label, icon)) in [
            (
                BottomTab::Diagnostics,
                "Logs & diagnostics",
                IconName::ListTree,
            ),
            (BottomTab::Relationships, "Relationships", IconName::Link),
            (BottomTab::Documentation, "Markdown", IconName::FileMarkdown),
            (BottomTab::Thumbnails, "Thumbnails", IconName::Image),
        ]
        .into_iter()
        .enumerate()
        {
            let owner = handle.clone();
            tabs.push(
                IconButton::new(format!("mde-bottom-tab-{index}"), icon)
                    .aria_label(label)
                    .tooltip(ui::Tooltip::text(label))
                    .toggle_state(active == kind)
                    .on_click(move |_, window, cx| {
                        if let Err(error) = owner.update(cx, |panel, cx| {
                            if panel.active != kind {
                                window.focus(&panel.state.focus, cx);
                            }
                            panel.active = kind;
                            cx.notify();
                        }) {
                            eprintln!("MDE details tab selection failed: {error}");
                        }
                    }),
            );
        }
        let description = match active {
            BottomTab::Diagnostics => "Logs & diagnostics",
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
            .child(
                div()
                    .h(px(36.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children(tabs)
                    .child(div().flex_1())
                    .child(
                        IconButton::new("mde-bottom-close", IconName::Close)
                            .aria_label("Hide MDE logs and details")
                            .tooltip(ui::Tooltip::text(
                                "Hide panel; execution events continue to be recorded",
                            ))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Close))),
                    ),
            )
            .when(active == BottomTab::Diagnostics, |panel| {
                panel.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .child(self.execution.clone()),
                )
            })
            .when(active != BottomTab::Diagnostics, |panel| {
                panel.child(
                    div()
                        .id("mde-bottom-output")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p_3()
                        .child(div().whitespace_normal().child(description)),
                )
            })
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
                }) || panel
                    .timing_drafts
                    .values()
                    .any(|draft| draft.has_focused_input(window, cx))
                    || panel
                        .explorer_action
                        .as_ref()
                        .and_then(|draft| draft.rename.as_ref())
                        .is_some_and(|input| {
                            input.read(cx).focus_handle(cx).contains_focused(window, cx)
                        });
                if hidden_input_focused {
                    window.focus(&panel.state.focus, cx);
                }
                panel.explorer_action = None;
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
                            crate::components::inspector::flush_property_edits(panel, &item, cx);
                            crate::components::style_editor::flush_style_edits(panel, &item, cx);
                        }
                    }) {
                        eprintln!("MDE document queue resume failed: {error}");
                    }
                });
            }
        });
    }
    if let Some(panel) = workspace.panel::<MdeBottomPanel>(cx) {
        panel.update(cx, |panel, cx| panel.bind(item.as_ref(), cx));
    }
}

fn show_logs_and_diagnostics(
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    if let Some(panel) = workspace.panel::<MdeBottomPanel>(cx) {
        panel.update(cx, |panel, cx| {
            panel.active = BottomTab::Diagnostics;
            cx.notify();
        });
    }
    workspace.reveal_panel::<MdeBottomPanel>(window, cx);
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
    workspace.register_action(
        |workspace,
         _: &crate::components::execution_dashboard::ToggleMdeExecutionDashboard,
         window,
         cx| {
            show_logs_and_diagnostics(workspace, window, cx);
        },
    );
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
        show_logs_and_diagnostics(workspace, window, cx);
    });
    let left = cx.new(|cx| MdeLeftPanel::new(item.clone(), window, cx));
    let right = cx.new(|cx| MdeRightPanel::new(item.clone(), window, cx));
    let bottom = cx.new(|cx| MdeBottomPanel::new(item, window, cx));
    workspace.add_panel(left, window, cx);
    workspace.add_panel(right, window, cx);
    workspace.add_panel(bottom, window, cx);
}
