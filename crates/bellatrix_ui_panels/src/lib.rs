//! Public panel components and a host-independent Dock view adapter.
//! Layout and DockArea ownership remain with the embedding Page.

// Zed Workspace owns Dock Panels; these are reusable panel contents.

use bellatrix_design_system::Palette;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

#[derive(IntoElement)]
pub struct PaneHeader {
    title: SharedString,
}
impl PaneHeader {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
        }
    }
}
impl RenderOnce for PaneHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        div()
            .h_9()
            .w_full()
            .flex()
            .items_center()
            .px_3()
            .border_b_1()
            .border_color(gpui::rgb(p.border))
            .text_xs()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(gpui::rgb(p.text))
            .child(self.title)
    }
}

#[derive(IntoElement)]
pub struct PropertyRow {
    label: SharedString,
    value: SharedString,
}
impl PropertyRow {
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
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
                    .text_xs()
                    .text_color(gpui::rgb(p.muted))
                    .child(self.label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(gpui::rgb(p.border))
                    .rounded_sm()
                    .text_xs()
                    .text_color(gpui::rgb(p.text))
                    .child(self.value),
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
