//! Public symbol gallery on the same GPUI runtime as Zed Workspace.
use crate::{Family, count, glyph, groups};
use bellatrix_design_system::Palette;
use gpui::{
    App, IntoElement, RenderOnce, StatefulInteractiveElement, Window, div, prelude::*, px, rgb, svg,
};

#[derive(IntoElement)]
pub struct SymbolGallery {
    family: Family,
}

impl SymbolGallery {
    pub fn new(family: Family) -> Self {
        Self { family }
    }
}

impl RenderOnce for SymbolGallery {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        div()
            .id("bellatrix-symbol-gallery")
            .size_full()
            .flex()
            .flex_col()
            .gap_6()
            .p_5()
            .overflow_y_scroll()
            .bg(rgb(p.canvas))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(p.muted))
                    .child(format!("{} symbols · visual reference", count(self.family))),
            )
            .children(groups(self.family).iter().map(|group| {
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .border_b_1()
                            .border_color(rgb(p.border))
                            .pb_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(group.title),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(p.muted))
                                    .child(format!("{} items", group.items.len())),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .children(group.items.iter().map(|symbol| {
                                div()
                                    .w(px(96.0))
                                    .h(px(80.0))
                                    .p_3()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(p.border))
                                    .bg(rgb(p.panel))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .child(
                                        svg()
                                            .data(glyph(symbol.kind))
                                            .w(px(24.0))
                                            .h(px(24.0))
                                            .text_color(rgb(p.muted)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_center()
                                            .text_color(rgb(p.text))
                                            .child(symbol.label),
                                    )
                            })),
                    )
            }))
    }
}
