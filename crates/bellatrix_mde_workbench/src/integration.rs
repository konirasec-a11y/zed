//! Present Bellatrix MDE automatically as a native Zed Workspace Item.
//!
//! Zed owns tabs, docks and focus. The native action remains available
//! to focus or reopen the MDE tab without creating a parallel window.

use crate::{MdeUiSource, MdeWorkbenchItem, open_in_workspace, panels};
use gpui::{App, Window, actions};
use workspace::Workspace;

actions!(bellatrix_mde, [OpenMdeWorkbench]);

/// Register the native action and attach the initial MDE Item and Dock panels.
///
/// Opening the action again focuses the existing item without duplicates.
pub fn register_mde_workspace(cx: &mut App, source: MdeUiSource) {
    cx.observe_new(move |workspace: &mut Workspace, window: Option<&mut Window>, cx| {
        let action_source = source.clone();
        workspace.register_action(move |workspace, _: &OpenMdeWorkbench, window, cx| {
            if let Some(item) = workspace.item_of_type::<MdeWorkbenchItem>(cx) {
                workspace.activate_item(&item, true, true, window, cx);
                return;
            }

            let item = open_in_workspace(workspace, action_source.clone(), window, cx);
            panels::attach_panels(workspace, item, window, cx);
        });

        // Registering an action does not execute it. Open Bellatrix on every
        // native Workspace, including freshly created and restored workspaces.
        // Defer until GPUI finishes constructing this Workspace entity;
        // mutating panes or docks from the observe_new callback is too early.
        if let Some(window) = window {
            let startup_source = source.clone();
            cx.defer_in(window, move |workspace, window, cx| {
                if workspace.item_of_type::<MdeWorkbenchItem>(cx).is_none() {
                    let item = open_in_workspace(workspace, startup_source, window, cx);
                    panels::attach_panels(workspace, item, window, cx);
                }
            });
        }
    })
    .detach();
}
