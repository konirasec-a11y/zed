//! Reusable panel headers; the embedding workspace owns dock layout and panel contents.

use bellatrix_design_system::Palette;
use gpui::{
    AnyElement, App, ClickEvent, IntoElement, RenderOnce, SharedString, Window, div, prelude::*,
};

#[derive(IntoElement)]
pub struct PaneHeader {
    title: SharedString,
    disclosure: Option<AnyElement>,
    action: Option<AnyElement>,
}
impl PaneHeader {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            disclosure: None,
            action: None,
        }
    }
}
impl PaneHeader {
    pub fn toggle(
        mut self,
        open: bool,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.disclosure = Some(
            ui::Disclosure::new(format!("{}-disclosure", self.title), open)
                .on_click(handler)
                .into_any_element(),
        );
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}
impl RenderOnce for PaneHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        div()
            .h_9()
            .flex_none()
            .w_full()
            .flex()
            .items_center()
            .px_3()
            .border_b_1()
            .border_color(gpui::rgb(p.border))
            .text_xs()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(gpui::rgb(p.text))
            .gap_1()
            .children(self.disclosure)
            .child(div().flex_1().min_w_0().overflow_hidden().child(self.title))
            .children(self.action)
    }
}
