//! C4 system/container/component cards as reusable visual values.
use bellatrix_design_system::Palette;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum C4Kind {
    Person,
    System,
    Container,
    Component,
    Database,
}

#[derive(IntoElement)]
pub struct C4Node {
    kind: C4Kind,
    name: SharedString,
    technology: SharedString,
    description: SharedString,
}
impl C4Node {
    pub fn new(kind: C4Kind, name: impl Into<SharedString>) -> Self {
        Self {
            kind,
            name: name.into(),
            technology: "".into(),
            description: "".into(),
        }
    }
    pub fn technology(mut self, text: impl Into<SharedString>) -> Self {
        self.technology = text.into();
        self
    }
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = text.into();
        self
    }
}
impl RenderOnce for C4Node {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        let label = match self.kind {
            C4Kind::Person => "Person",
            C4Kind::System => "Software system",
            C4Kind::Container => "Container",
            C4Kind::Component => "Component",
            C4Kind::Database => "Database",
        };
        div()
            .w_48()
            .flex()
            .flex_col()
            .gap_1()
            .rounded_sm()
            .p_3()
            .border_1()
            .border_color(gpui::rgb(p.border))
            .bg(gpui::rgb(p.panel))
            .child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(p.muted))
                    .child(label),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(gpui::rgb(p.text))
                    .child(self.name),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(p.accent))
                    .child(self.technology),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(p.muted))
                    .child(self.description),
            )
    }
}
