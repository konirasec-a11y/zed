use crate::ui::integration::OpenMdeWorkbench;
use crate::ui::native_document::OpenMdeProjectPreview;
use gpui::App;
use ui::{ContextMenu, DropdownMenu, DropdownStyle, Tooltip, prelude::*};
use workspace::{HideStatusItem, ItemHandle, StatusItemView};

pub(crate) struct MdeStatusEntry;

pub(crate) fn status_button_enabled(value: Option<&str>) -> bool {
    !value.is_some_and(|value| value.trim().eq_ignore_ascii_case("false"))
}

impl Render for MdeStatusEntry {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = ContextMenu::build(window, cx, |menu, _, _| {
            menu.action(
                "Open MDE project (read-only preview)",
                Box::new(OpenMdeProjectPreview),
            )
        });
        h_flex()
            .child(
                Button::new("bellatrix-mde-status-entry", "MDE")
                    .label_size(LabelSize::Small)
                    .tab_index(0isize)
                    .aria_label("Open MDE Workbench")
                    .tooltip(|_window, cx| {
                        Tooltip::for_action("Open MDE Workbench", &OpenMdeWorkbench, cx)
                    })
                    .on_click(cx.listener(|_this, _, window, cx| {
                        window.dispatch_action(Box::new(OpenMdeWorkbench), cx);
                    })),
            )
            .child(
                DropdownMenu::new("bellatrix-mde-open-project", "Open project", menu)
                    .trigger_size(ButtonSize::Compact)
                    .style(DropdownStyle::Subtle),
            )
    }
}

impl StatusItemView for MdeStatusEntry {
    fn set_active_pane_item(
        &mut self,
        _active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn hide_setting(&self, _: &App) -> Option<HideStatusItem> {
        // Visibility is controlled by BELLATRIX_MDE_STATUS_BUTTON=false at startup,
        // rather than changing an unrelated native editor preference.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::status_button_enabled;

    #[test]
    fn status_entry_is_visible_unless_explicitly_disabled() {
        assert!(status_button_enabled(None));
        assert!(status_button_enabled(Some("true")));
        assert!(status_button_enabled(Some("")));
        assert!(!status_button_enabled(Some("false")));
        assert!(!status_button_enabled(Some(" FALSE ")));
    }
}
