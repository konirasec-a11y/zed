//! Icon-only, context-sensitive UML editing controls on the Canvas.
//! Command discovery and execution are supplied by the host/engine. This
//! crate renders the view and never owns project state or Cordis services.
use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, RenderOnce, Window, div, prelude::*,
};
use std::rc::Rc;
use ui::{
    ContextMenu, DropdownMenu, Icon, IconButton, IconButtonShape, IconName, Tooltip, prelude::*,
};

type ActionHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;

#[derive(Clone)]
pub struct UmlQuickAction {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: UmlQuickActionIcon,
    pub row: u8,
    pub shortcut: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub enum UmlQuickActionIcon {
    Element(crate::elements::uml::UmlVisualDefinition),
    Action(IconName),
}

/// A small set of *available* icon buttons beside an active UML element.
/// Unsupported operations must never appear as clickable controls.
#[derive(IntoElement)]
pub struct UmlQuickEdit {
    actions: Vec<UmlQuickAction>,
    handler: ActionHandler,
    side: &'static str,
}

impl UmlQuickEdit {
    pub fn new(
        actions: Vec<UmlQuickAction>,
        on_action: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            actions,
            handler: Rc::new(on_action),
            side: "right",
        }
    }
    pub fn side(mut self, side: &'static str) -> Self {
        self.side = side;
        self
    }
}

impl RenderOnce for UmlQuickEdit {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Preserve the source Quick Edit row indices: Class has four rows,
        // Interface has two. Never collapse specialized rows onto one strip.
        let mut rows: [Vec<gpui::AnyElement>; 4] = std::array::from_fn(|_| Vec::new());
        for action in self.actions {
            let handler = self.handler.clone();
            let button = match action.icon {
                UmlQuickActionIcon::Element(visual) => {
                    ui::ButtonLike::new(format!("uml-action-{}", action.id))
                        .width(ui::IconSize::default().square(window, cx))
                        .height(ui::IconSize::default().square(window, cx).into())
                        .child(visual.render_icon(16.0, cx.theme().colors().panel_background))
                }
                UmlQuickActionIcon::Action(icon) => {
                    IconButton::new(format!("uml-action-{}", action.id), icon)
                        .shape(IconButtonShape::Square)
                        .render(window, cx)
                }
            }
            .aria_label(action.label)
            .tooltip(Tooltip::text(match action.shortcut {
                Some(keys) => format!("{} ({keys})", action.label),
                None => action.label.into(),
            }))
            .on_click(move |_, window, cx| {
                handler(action.id, window, cx);
                cx.stop_propagation();
            })
            .into_any_element();
            let row = usize::from(action.row.saturating_sub(1)).min(3);
            rows[row].push(button);
        }
        div()
            .id(format!("bellatrix-uml-quick-actions-{}", self.side))
            .flex()
            .flex_col()
            .gap_1()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(rows.into_iter().enumerate().filter_map(|(index, buttons)| {
                (!buttons.is_empty()).then(|| {
                    div()
                        .id(format!("bellatrix-quick-row-{index}"))
                        .flex()
                        .gap_1()
                        .children(buttons)
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
        Self {
            current: current.into(),
            on_change: Rc::new(on_change),
        }
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
                .trigger_tooltip(Tooltip::text("Change visibility")),
            )
    }
}
