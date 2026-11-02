//! Attach the MDE document and tools to Zed Workspace events.

use crate::{MdeUiSource, open_in_workspace, panels};
use gpui::{App, Window};
use workspace::Workspace;

/// Must be invoked once after Zed initializes its services.
pub fn register_mde_workspace(cx: &mut App, source: MdeUiSource) {
    cx.observe_new(move |workspace: &mut Workspace, window: Option<&mut Window>, cx| {
        let Some(window) = window else { return };
        let item = open_in_workspace(workspace, source.clone(), window, cx);
        panels::attach_panels(workspace, item, window, cx);
    }).detach();
}
