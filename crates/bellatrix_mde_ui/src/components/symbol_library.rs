//! Bellatrix symbol catalog as a normal Zed Workspace Item.
//! The catalog lives in crate::assets::symbols and SVGs in crate::assets::icons; this is a
//! preview, not a second Dock and not a substitute for domain commands.
use crate::assets::symbols::Family;
use crate::components::symbol_gallery::SymbolGallery;
use gpui::App;
use gpui::Context;
use gpui::EventEmitter;
use gpui::FocusHandle;
use gpui::Focusable;
use gpui::IntoElement;
use gpui::Render;
use gpui::SharedString;
use gpui::Window;
use gpui::div;
use gpui::prelude::*;
use gpui::px;
use ui::Button;
use ui::Clickable;
use workspace::item::Item;

pub struct MdeSymbolLibraryItem {
    focus: FocusHandle,
    family: Family,
}
impl MdeSymbolLibraryItem {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            family: Family::Uml,
        }
    }
}
impl EventEmitter<()> for MdeSymbolLibraryItem {}
impl Focusable for MdeSymbolLibraryItem {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Item for MdeSymbolLibraryItem {
    type Event = ();
    fn tab_content_text(&self, _: usize, _: &App) -> SharedString {
        "Bellatrix · Symbol Library".into()
    }
    fn show_toolbar(&self) -> bool {
        false
    }
}

impl Render for MdeSymbolLibraryItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let family = self.family;
        let owner = cx.weak_entity();
        let families = [
            ("UML", Family::Uml),
            ("C4", Family::C4),
            ("Mind map", Family::MindMap),
            ("Flowchart", Family::Flowchart),
        ];
        div()
            .id("bellatrix-symbol-library")
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus)
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex_none()
                    .flex()
                    .gap_2()
                    .items_center()
                    .px_3()
                    .children(families.into_iter().map(|(label, value)| {
                        let owner = owner.clone();
                        Button::new(format!("bellatrix-symbol-family-{label}"), label).on_click(
                            move |_, _, cx| {
                                let _ = owner.update(cx, |item, cx| {
                                    item.family = value;
                                    cx.notify();
                                });
                            },
                        )
                    })),
            )
            .child(div().flex_1().min_h_0().child(SymbolGallery::new(family)))
    }
}
