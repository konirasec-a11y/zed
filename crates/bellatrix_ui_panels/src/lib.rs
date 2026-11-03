//! Public panel components and a host-independent Dock view adapter.
//! Layout and DockArea ownership remain with the embedding Page.

// Zed Workspace owns Dock Panels; these are reusable panel contents.

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

#[derive(IntoElement)]
pub struct PropertyRow {
    label: SharedString,
    value: Option<SharedString>,
    control: Option<AnyElement>,
}
impl PropertyRow {
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: Some(value.into()),
            control: None,
        }
    }
}
impl PropertyRow {
    pub fn with_control(label: impl Into<SharedString>, control: impl IntoElement) -> Self {
        Self {
            label: label.into(),
            value: None,
            control: Some(control.into_any_element()),
        }
    }
}
impl RenderOnce for PropertyRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        div()
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .child(
                div()
                    .w_20()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(gpui::rgb(p.muted))
                    .child(self.label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_7()
                    .flex()
                    .items_center()
                    .when(self.control.is_none(), |this| {
                        this.px_2()
                            .border_1()
                            .border_color(gpui::rgb(p.border))
                            .rounded_sm()
                    })
                    .text_xs()
                    .text_color(gpui::rgb(p.text))
                    .children(self.value)
                    .children(self.control),
            )
    }
}

#[derive(IntoElement)]
pub struct EmptyPanel {
    message: SharedString,
}
impl EmptyPanel {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
impl RenderOnce for EmptyPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        div()
            .size_full()
            .p_4()
            .bg(gpui::rgb(p.panel))
            .text_xs()
            .text_color(gpui::rgb(p.muted))
            .child(self.message)
    }
}
