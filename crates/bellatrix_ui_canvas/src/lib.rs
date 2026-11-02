//! Generic canvas presentation; editing, hit-testing and persistence are host responsibilities.
use bellatrix_design_system::Palette;
use gpui::{AnyElement, App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

#[derive(IntoElement)]
pub struct CanvasSurface {
    breadcrumb: SharedString,
    trailing: SharedString,
    body: Option<AnyElement>,
}
impl CanvasSurface {
    pub fn new(breadcrumb: impl Into<SharedString>) -> Self {
        Self {
            breadcrumb: breadcrumb.into(),
            trailing: "100%".into(),
            body: None,
        }
    }
    pub fn trailing(mut self, value: impl Into<SharedString>) -> Self {
        self.trailing = value.into();
        self
    }
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.body = Some(content.into_any_element());
        self
    }
}
impl RenderOnce for CanvasSurface {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(gpui::rgb(p.canvas))
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .h_9()
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(gpui::rgb(p.panel))
                    .border_b_1()
                    .border_color(gpui::rgb(p.border))
                    .text_xs()
                    .text_color(gpui::rgb(p.muted))
                    .child(self.breadcrumb)
                    .child(self.trailing),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .children(self.body),
            )
    }
}
