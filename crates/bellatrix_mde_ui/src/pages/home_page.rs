use crate::pages::mde_page::{MdeUiSource, MdeWorkbenchItem, open_in_workspace, panels};
use gpui::{Context, Entity, Window};
use workspace::Workspace;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Mde,
}

pub(crate) fn navigate(
    page: Page,
    workspace: &mut Workspace,
    source: MdeUiSource,
    cached_mde: Option<Entity<MdeWorkbenchItem>>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Entity<MdeWorkbenchItem> {
    match page {
        Page::Mde => {
            source.set_preview_context(None);
            let item = if let Some(item) = workspace.item_of_type::<MdeWorkbenchItem>(cx) {
                panels::attach_panels(workspace, item.clone(), window, cx);
                workspace.activate_item(&item, true, true, window, cx);
                item
            } else if let Some(item) = cached_mde {
                panels::attach_panels(workspace, item.clone(), window, cx);
                workspace.add_item_to_active_pane(Box::new(item.clone()), None, true, window, cx);
                item
            } else {
                let item = open_in_workspace(workspace, source, window, cx);
                panels::attach_panels(workspace, item.clone(), window, cx);
                item
            };
            item
        }
    }
}
