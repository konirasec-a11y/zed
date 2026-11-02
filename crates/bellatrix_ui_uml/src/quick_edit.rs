//! Icon-only, context-sensitive UML editing controls on the Canvas.
//! Command discovery and execution are supplied by the host/engine. This
//! crate renders the view and never owns project state or Cordis services.
use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, RenderOnce, Window,
    div, prelude::*,
};
use std::rc::Rc;
use ui::{
    ContextMenu, DropdownMenu, Icon, IconButton, IconButtonShape, IconName,
    Tooltip, prelude::*,
};

type ActionHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;

#[derive(Clone)]
pub struct UmlQuickAction {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: IconName,
}

/// A small set of *available* icon buttons beside an active UML element.
/// Unsupported operations must never appear as clickable controls.
#[derive(IntoElement)]
pub struct UmlQuickEdit {
    actions: Vec<UmlQuickAction>,
    handler: ActionHandler,
}

impl UmlQuickEdit {
    pub fn new(
        actions: Vec<UmlQuickAction>,
        on_action: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self { actions, handler: Rc::new(on_action) }
    }
}

impl RenderOnce for UmlQuickEdit {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = bellatrix_design_system::Palette::current(cx);
        div()
            .id("bellatrix-uml-quick-actions")
            .flex()
            .flex_wrap()
            .gap_1()
            .p_1()
            .rounded_sm()
            .border_1()
            .border_color(gpui::rgb(palette.border))
            .bg(gpui::rgb(palette.panel))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(self.actions.into_iter().map(|action| {
                let handler = self.handler.clone();
                IconButton::new(format!("uml-action-{}", action.id), action.icon)
                    .shape(IconButtonShape::Square)
                    .aria_label(action.label)
                    .tooltip(Tooltip::text(action.label))
                    .on_click(move |_, window, cx| {
                        handler(action.id, window, cx);
                        cx.stop_propagation();
                    })
            }))
    }
}

/// StarUML-style left-side visibility control: one icon opens a native Zed
/// dropdown with public/protected/private/package choices.
#[derive(IntoElement)]
pub struct UmlVisibilityPicker {
    current: String,
    on_change: ActionHandler,
}

impl UmlVisibilityPicker {
    pub fn new(
        current: impl Into<String>,
        on_change: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self { current: current.into(), on_change: Rc::new(on_change) }
    }
}

impl RenderOnce for UmlVisibilityPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let current = self.current;
        let callback = self.on_change;
        let menu = ContextMenu::build(window, cx, move |menu, _, _| {
            ["public", "protected", "private", "package"]
                .into_iter()
                .fold(menu, |menu, option| {
                    let cb = callback.clone();
                    let label = if current == option {
                        format!("✓ {option}")
                    } else {
                        option.to_owned()
                    };
                    menu.entry(label, None, move |window, cx| {
                        cb(option, window, cx);
                    })
                })
        });
        div()
            .id("bellatrix-uml-visibility")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                DropdownMenu::new_with_element(
                    "bellatrix-uml-visibility-picker",
                    Icon::new(IconName::Eye).into_any_element(),
                    menu,
                )
                .aria_label("Visibility")
                .trigger_tooltip(Tooltip::text("Change visibility"))
            )
    }
}
