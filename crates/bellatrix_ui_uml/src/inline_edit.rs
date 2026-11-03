//! Native Zed inline text editing inside a UML element compartment.
//! No model storage, IDs, commands, or Cordis types are owned here.
use gpui::{
    App, Entity, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, RenderOnce, Window,
    div, prelude::*,
};
use std::rc::Rc;
use ui::{IconButton, IconButtonShape, IconName, Tooltip, prelude::*};
use ui_input::InputField;

type EditorAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// Replaces the name label of an element or member while editing, in place.
/// Enter confirms; Escape cancels. Focus and validation belong to the host.
#[derive(IntoElement)]
pub struct UmlInlineNameEditor {
    field: Entity<InputField>,
    accept: EditorAction,
    cancel: EditorAction,
}

impl UmlInlineNameEditor {
    pub fn new(
        field: Entity<InputField>,
        accept: impl Fn(&mut Window, &mut App) + 'static,
        cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            field,
            accept: Rc::new(accept),
            cancel: Rc::new(cancel),
        }
    }
}

impl RenderOnce for UmlInlineNameEditor {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let confirm_key = self.accept.clone();
        let cancel_key = self.cancel.clone();
        let confirm_button = self.accept.clone();
        let cancel_button = self.cancel.clone();
        div()
            .id("bellatrix-uml-inline-editor")
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap_1()
            .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "enter" if !event.keystroke.modifiers.control => {
                        confirm_key(window, cx);
                        cx.stop_propagation();
                    }
                    "escape" => {
                        cancel_key(window, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            })
            // Never turn editing a name into a Canvas move gesture.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().flex_1().min_w_0().child(self.field))
            .child(
                IconButton::new("bellatrix-uml-accept-name", IconName::Check)
                    .shape(IconButtonShape::Square)
                    .aria_label("Confirm name")
                    .tooltip(Tooltip::text("Apply name (Enter)"))
                    .on_click(move |_, window, cx| confirm_button(window, cx)),
            )
            .child(
                IconButton::new("bellatrix-uml-cancel-name", IconName::Close)
                    .shape(IconButtonShape::Square)
                    .aria_label("Cancel name edit")
                    .tooltip(Tooltip::text("Cancel (Esc)"))
                    .on_click(move |_, window, cx| cancel_button(window, cx)),
            )
    }
}
