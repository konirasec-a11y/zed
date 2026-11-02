//! Present Bellatrix MDE automatically as a native Zed Workspace Item.
//!
//! Zed owns tabs, docks and focus. The native action remains available
//! to focus or reopen the MDE tab without creating a parallel window.

use crate::{MdeSymbolLibraryItem, MdeUiSource, MdeWorkbenchItem, open_in_workspace, panels};
use gpui::{App, AppContext, Entity, Window, actions};
use std::{cell::RefCell, rc::Rc};
use workspace::Workspace;

actions!(
    bellatrix_mde,
    [
        OpenMdeWorkbench,
        OpenMdeSymbols,
        RemoveFromDiagram,
        DeleteFromModel
    ]
);

/// Register the native action and attach the initial MDE Item and Dock panels.
///
/// Opening the action again focuses the existing item without duplicates.
pub fn register_mde_workspace(cx: &mut App, source: MdeUiSource) {
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

            // Registering an action does not execute it. Open Bellatrix on every
            // native Workspace, including freshly created and restored workspaces.
            // Defer until GPUI finishes constructing this Workspace entity;
            // mutating panes or docks from the observe_new callback is too early.
            if let Some(window) = window {
                let startup_source = source.clone();
                let startup_item = Rc::clone(&cached_item);
                cx.defer_in(window, move |workspace, window, cx| {
                    if let Some(item) = workspace.item_of_type::<MdeWorkbenchItem>(cx) {
                        *startup_item.borrow_mut() = Some(item);
                    } else {
                        let item = open_in_workspace(workspace, startup_source, window, cx);
                        panels::attach_panels(workspace, item.clone(), window, cx);
                        *startup_item.borrow_mut() = Some(item);
                    }
                });
            }
        },
    )
    .detach();
}
