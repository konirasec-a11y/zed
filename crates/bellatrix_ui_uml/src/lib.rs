//! Native Zed GPUI UML class view; presentation independent of MDE storage.
use bellatrix_design_system::Palette;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px, rgb};

#[derive(IntoElement)]
pub struct UmlClassCard {
    title: SharedString,
    attributes: Vec<SharedString>,
    operations: Vec<SharedString>,
    selected: bool,
}

impl UmlClassCard {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), attributes: Vec::new(), operations: Vec::new(), selected: false }
    }

    pub fn attribute(mut self, value: impl Into<SharedString>) -> Self {
        self.attributes.push(value.into());
        self
    }

    pub fn operation(mut self, value: impl Into<SharedString>) -> Self {
        self.operations.push(value.into());
        self
    }

    pub fn selected(mut self, value: bool) -> Self {
        self.selected = value;
        self
    }
}

impl RenderOnce for UmlClassCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        let line = rgb(if self.selected { p.selection } else { p.border });
        div()
            .size_full()
            .flex()
            .flex_col()
            .border_1()
            .border_color(line)
            .rounded_sm()
            .bg(rgb(p.panel))
            .overflow_hidden()
            .child(
                div()
                    .h(px(34.0))
                    .w_full()
                    .flex_none()
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(p.panel_alt))
                    .border_b_1()
                    .border_color(line)
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(p.text))
                    .child(self.title),
            )
            .when(!self.attributes.is_empty(), |this| {
                this.child(
                    div()
                        .w_full()
                        .px_3()
                        .py_2()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(self.attributes.into_iter().map(|label| {
                            div().text_size(px(12.0)).text_color(rgb(p.muted)).child(label)
                        })),
                )
            })
            .when(!self.operations.is_empty(), |this| {
                this.child(
                    div()
                        .w_full()
                        .px_3()
                        .py_2()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .border_t_1()
                        .border_color(line)
                        .children(self.operations.into_iter().map(|label| {
                            div().text_size(px(12.0)).text_color(rgb(p.muted)).child(label)
                        })),
                )
            })
    }
}
