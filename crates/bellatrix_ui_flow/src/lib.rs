//! Presentation-only flowchart nodes.
use bellatrix_design_system::Palette;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowKind {
    Start,
    Process,
    Decision,
    Input,
    End,
}

#[derive(IntoElement)]
pub struct FlowNode {
    kind: FlowKind,
    title: SharedString,
}
impl FlowNode {
    pub fn new(kind: FlowKind, title: impl Into<SharedString>) -> Self {
        Self {
            kind,
            title: title.into(),
        }
    }
}
impl RenderOnce for FlowNode {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        let label = match self.kind {
            FlowKind::Start => "Start",
            FlowKind::Process => "Process",
            FlowKind::Decision => "Decision",
            FlowKind::Input => "Input",
            FlowKind::End => "End",
        };
        div()
            .min_w_32()
            .py_3()
            .px_4()
            .rounded_sm()
            .border_1()
            .border_color(gpui::rgb(p.border))
            .bg(gpui::rgb(p.panel))
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .child(div().text_xs().text_color(gpui::rgb(p.muted)).child(label))
            .child(
                div()
                    .text_sm()
                    .text_color(gpui::rgb(p.text))
                    .child(self.title),
            )
    }
}
