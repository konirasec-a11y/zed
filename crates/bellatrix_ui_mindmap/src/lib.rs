//! Presentation-only mindmap topics; the host owns the graph.
use bellatrix_design_system::Palette;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopicLevel {
    Central,
    Main,
    Subtopic,
}

#[derive(IntoElement)]
pub struct MindTopic {
    level: TopicLevel,
    title: SharedString,
}
impl MindTopic {
    pub fn new(level: TopicLevel, title: impl Into<SharedString>) -> Self {
        Self {
            level,
            title: title.into(),
        }
    }
}
impl RenderOnce for MindTopic {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        let selected = matches!(self.level, TopicLevel::Central);
        div()
            .min_w_24()
            .px_4()
            .py_2()
            .rounded_lg()
            .border_1()
            .border_color(gpui::rgb(if selected { p.accent } else { p.border }))
            .bg(gpui::rgb(if selected { p.selected } else { p.panel }))
            .text_sm()
            .text_color(gpui::rgb(p.text))
            .child(self.title)
    }
}
