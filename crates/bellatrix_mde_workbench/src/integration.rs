//! Open Bellatrix MDE on request as a native Zed Workspace Item.
//!
//! Zed owns tabs, docks and focus. The native action remains available
//! to focus or reopen the MDE tab without creating a parallel window.

use crate::status_entry::{MdeStatusEntry, status_button_enabled};
use crate::{MdeSymbolLibraryItem, MdeUiSource, MdeWorkbenchItem, open_in_workspace, panels};
use bellatrix_mde_cordis::MdeUiCommand;
use bellatrix_project::ProjectSaveRequested;
use gpui::{App, AppContext, Entity, Window, actions};
use project::DirectoryLister;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use workspace::Workspace;

actions!(
    bellatrix_mde,
    [
        OpenMdeWorkbench,
        OpenMdeSymbols,
        SaveMdeProjectAs,
        RemoveFromDiagram,
        DeleteFromModel
    ]
);

struct SavePathPromptGuard(Rc<Cell<bool>>);

impl SavePathPromptGuard {
    fn acquire(pending: &Rc<Cell<bool>>) -> Option<Self> {
        if pending.replace(true) {
            None
        } else {
            Some(Self(Rc::clone(pending)))
        }
    }
}

impl Drop for SavePathPromptGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// Keep MDE discoverable without opening it in unconfigured Workspaces.
///
/// Opening the action again focuses the existing item without duplicates.
pub fn register_mde_workspace(cx: &mut App, source: MdeUiSource) {
    let auto_open_configured_source = std::env::var_os("BELLATRIX_PROJECT_FILE").is_some()
        || std::env::var_os("BELLATRIX_MODEL_FILE").is_some();
    let show_status_button =
        status_button_enabled(std::env::var("BELLATRIX_MDE_STATUS_BUTTON").ok().as_deref());
    cx.bind_keys([
        gpui::KeyBinding::new("delete", RemoveFromDiagram, Some("BellatrixMde && !Editor")),
        gpui::KeyBinding::new(
            "backspace",
            RemoveFromDiagram,
            Some("BellatrixMde && !Editor"),
        ),
        gpui::KeyBinding::new(
            "ctrl-delete",
            DeleteFromModel,
            Some("BellatrixMde && !Editor"),
        ),
    ]);

    cx.observe_new(
        move |workspace: &mut Workspace, window: Option<&mut Window>, cx| {
            // The full Bellatrix catalog is a native Zed Item. The Workbench
            // remains focused on supported Class and Interface editing gestures.
            workspace.register_action(|workspace, _: &OpenMdeSymbols, window, cx| {
                if let Some(item) = workspace.item_of_type::<MdeSymbolLibraryItem>(cx) {
                    workspace.activate_item(&item, true, true, window, cx);
                } else {
                    let item = cx.new(MdeSymbolLibraryItem::new);
                    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
                }
            });

            // Panels retain the MDE Item when its center tab closes. Reuse that
            // Entity on reopening so selection survives and Dock panels are not
            // registered twice under the same panel keys.
            let cached_item = Rc::new(RefCell::new(None::<Entity<MdeWorkbenchItem>>));
            let save_item = Rc::clone(&cached_item);
            let save_prompt_pending = Rc::new(Cell::new(false));
            workspace.register_action(move |workspace, _: &SaveMdeProjectAs, window, cx| {
                if !workspace.project().read(cx).is_local() {
                    workspace.show_error(
                        "Bellatrix project saving currently requires a local workspace",
                        cx,
                    );
                    return;
                }
                let Some(item) = workspace
                    .item_of_type::<MdeWorkbenchItem>(cx)
                    .or_else(|| save_item.borrow().clone())
                else {
                    workspace.show_error("Open the MDE Workbench before saving its project", cx);
                    return;
                };
                let Some(prompt_guard) = SavePathPromptGuard::acquire(&save_prompt_pending) else {
                    return;
                };
                let suggested_name = "bellatrix-project.json".to_owned();
                let paths = workspace.prompt_for_new_path(
                    DirectoryLister::Local(
                        workspace.project().clone(),
                        workspace.app_state().fs.clone(),
                    ),
                    Some(suggested_name),
                    window,
                    cx,
                );
                cx.spawn_in(window, async move |workspace, cx| {
                    // Releasing the guard also handles cancellation or a dropped task.
                    let _prompt_guard = prompt_guard;
                    let paths = paths.await;
                    if let Err(error) = workspace.update(cx, |workspace, cx| {
                        let paths = match paths {
                            Ok(Some(paths)) => paths,
                            Ok(None) => return,
                            Err(_) => {
                                workspace.show_error(
                                    "The Save Project As dialog closed without a result",
                                    cx,
                                );
                                return;
                            }
                        };
                        let Some(path) = paths.into_iter().next() else {
                            return;
                        };
                        if !workspace.project().read(cx).is_local() {
                            workspace.show_error(
                                "Bellatrix project saving currently requires a local workspace",
                                cx,
                            );
                            return;
                        }
                        item.update(cx, |item, cx| {
                            // Keep the native PathBuf intact, including non-UTF-8 names.
                            // The existing bridge reports the actual ProjectSave outcome.
                            item.submit(
                                MdeUiCommand::SaveProject(ProjectSaveRequested {
                                    id: item.project_id().clone(),
                                    path,
                                }),
                                cx,
                            );
                        });
                    }) {
                        eprintln!("Bellatrix save dialog workspace unavailable: {error}");
                    }
                })
                .detach();
            });
            let action_item = Rc::clone(&cached_item);
            let action_source = source.clone();
            workspace.register_action(move |workspace, _: &OpenMdeWorkbench, window, cx| {
                if let Some(item) = workspace.item_of_type::<MdeWorkbenchItem>(cx) {
                    workspace.activate_item(&item, true, true, window, cx);
                    return;
                }
                if let Some(item) = action_item.borrow().clone() {
                    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
                    return;
                }

                let item = open_in_workspace(workspace, action_source.clone(), window, cx);
                panels::attach_panels(workspace, item.clone(), window, cx);
                *action_item.borrow_mut() = Some(item);
            });

            if let Some(window) = window {
                let startup_source = source.clone();
                let startup_item = Rc::clone(&cached_item);
                cx.defer_in(window, move |workspace, window, cx| {
                    if show_status_button {
                        workspace.status_bar().update(cx, |status_bar, cx| {
                            if status_bar.item_of_type::<MdeStatusEntry>().is_none() {
                                let button = cx.new(|_| MdeStatusEntry);
                                status_bar.add_left_item(button, window, cx);
                            }
                        });
                    }
                    // Only an explicitly configured startup source authorizes
                    // auto-opening; an empty/restoring Workspace is not intent.
                    if auto_open_configured_source {
                        let cached = startup_item.borrow().clone();
                        if let Some(item) = workspace.item_of_type::<MdeWorkbenchItem>(cx) {
                            *startup_item.borrow_mut() = Some(item);
                        } else if let Some(item) = cached {
                            workspace.add_item_to_active_pane(
                                Box::new(item),
                                None,
                                true,
                                window,
                                cx,
                            );
                        } else {
                            let item = open_in_workspace(workspace, startup_source, window, cx);
                            panels::attach_panels(workspace, item.clone(), window, cx);
                            *startup_item.borrow_mut() = Some(item);
                        }
                    }
                });
            }
        },
    )
    .detach();
}

#[cfg(test)]
mod tests {
    use super::SavePathPromptGuard;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn save_dialog_guard_blocks_duplicates_and_resets_on_drop() {
        let pending = Rc::new(Cell::new(false));
        let first = SavePathPromptGuard::acquire(&pending).expect("first prompt");
        assert!(SavePathPromptGuard::acquire(&pending).is_none());
        drop(first);
        assert!(!pending.get());
        let next = SavePathPromptGuard::acquire(&pending).expect("prompt after cancellation");
        drop(next);
        assert!(!pending.get());
    }
}
