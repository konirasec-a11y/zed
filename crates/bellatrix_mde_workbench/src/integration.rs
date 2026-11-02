//! Register MDE as a standard Zed Workspace action, not an automatic window.
//!
//! The Workspace owns tabs, docks, focus and session lifecycle. An MDE document
//! is only shown when the user explicitly activates it. This file does not
//! modify Zed core or create any competing app/project event infrastructure.

use crate::{MdeUiSource, MdeWorkbenchItem, open_in_workspace, panels};
use gpui::{App, Window, actions};
use workspace::Workspace;

actions!(bellatrix_mde, [OpenMdeWorkbench]);

/// Install an opt-in action on each native Zed Workspace.
///
/// Opening twice focuses the existing tab instead of creating another MDE
/// document or duplicating the Zed Dock panels.
pub fn register_mde_workspace(cx: &mut App, source: MdeUiSource) {
    cx.observe_new(move |workspace: &mut Workspace, _window: Option<&mut Window>, _cx| {
        let source = source.clone();
        workspace.register_action(move |workspace, _: &OpenMdeWorkbench, window, cx| {
            if let Some(item) = workspace.item_of_type::<MdeWorkbenchItem>(cx) {
                workspace.activate_item(&item, true, true, window, cx);
                return;
            }

            let item = open_in_workspace(workspace, source.clone(), window, cx);
            panels::attach_panels(workspace, item, window, cx);
        });
    })
    .detach();
}
