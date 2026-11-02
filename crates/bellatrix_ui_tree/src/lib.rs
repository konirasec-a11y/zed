//! A tree-like presentation from owner-provided rows; not a bellatrix-mde-modeling tree schema.
use bellatrix_design_system::Palette;
use gpui::InteractiveElement;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px};

#[derive(Clone)]
pub struct TreeRow {
    pub id: String,
    pub title: SharedString,
    pub depth: u8,
    pub selected: bool,
}
impl TreeRow {
    pub fn new(id: impl Into<String>, title: impl Into<SharedString>, depth: u8) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            depth,
            selected: false,
        }
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

#[derive(IntoElement)]
pub struct TreePane {
    rows: Vec<TreeRow>,
}
impl TreePane {
    pub fn new(rows: Vec<TreeRow>) -> Self {
        Self { rows }
    }
}
impl RenderOnce for TreePane {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_1()
            .overflow_y_scroll()
            .bg(gpui::rgb(p.panel))
            .p_2()
            .children(self.rows.into_iter().map(|row| {
                div()
                    .id(format!("tree-{}", row.id))
                    .w_full()
                    .h_7()
                    .rounded_sm()
                    .ml(px(f32::from(row.depth) * 14.))
                    .px_2()
                    .flex()
                    .items_center()
                    .bg(gpui::rgb(if row.selected {
                        p.selected
                    } else {
                        p.panel
                    }))
                    .text_xs()
                    .text_color(gpui::rgb(if row.selected { p.text } else { p.muted }))
                    .child(row.title)
            }))
    }
}
