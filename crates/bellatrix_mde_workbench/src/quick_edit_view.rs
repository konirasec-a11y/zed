//! Contextual Quick Edit for the selected element. This is not the long-lived
//! Property Editor: controls here are actions supplied by the selected *type*.
//!
//! StarUML quickedits/uml.json is the behavior reference. Only actions with a
//! real MDE/Cordis contract are interactive. Planned actions are plainly
//! labeled rather than rendering misleading buttons.
use crate::{MdeWorkbenchItem, WorkbenchPalette};
use bellatrix_mde_cordis::ModelingMemberKind;
use bellatrix_mde_model::Element;
use bellatrix_mde_modeling::{
    QuickEditAvailability, QuickEditControl, quick_edit_for,
};
use gpui::{
    AnyElement, App, Entity, IntoElement, MouseButton, StatefulInteractiveElement,
    div, prelude::*, px,
};
use serde_json::Value;
use ui::{Button, Label, prelude::*};
use ui_input::InputField;

pub(crate) fn render_quick_edit(
    element: &Element,
    item: Entity<MdeWorkbenchItem>,
    name_input: Option<Entity<InputField>>,
    cx: &mut App,
) -> AnyElement {
    let palette = WorkbenchPalette::current(cx);
    let mut active = Vec::<AnyElement>::new();
    let mut pending = Vec::<AnyElement>::new();
    let current_visibility = element.properties.get("visibility")
        .and_then(Value::as_str).unwrap_or("public");
    for action in quick_edit_for(&element.kind) {
        if action.availability == QuickEditAvailability::NeedsDomainContract {
            pending.push(
                div().px_2().py_1().text_xs().text_color(palette.muted)
                    .child(format!("{} · not implemented", action.title))
                    .into_any_element(),
            );
            continue;
        }
        match action.control {
            QuickEditControl::Input => {
                if let Some(input) = name_input.clone() {
                    let owner = item.clone();
                    let editor = input.clone();
                    active.push(
                        div().flex().flex_col().gap_1()
                            .child(Label::new("Name"))
                            .child(input)
                            .child(
                                Button::new("mde-quick-apply-name", "Apply name")
                                    .on_click(move |_, _, cx| {
                                        let name = editor.read(cx).text(cx);
                                        owner.update(cx, |item, cx| {
                                            item.set_selected_property(
                                                "name".into(), Value::String(name), cx
                                            );
                                        });
                                    }),
                            ).into_any_element(),
                    );
                }
            },
            QuickEditControl::Visibility => {
                let choices = ["public", "protected", "private", "package"];
                active.push(
                    div().flex().flex_col().gap_1()
                        .child(Label::new("Visibility"))
                        .child(
                            div().flex().flex_wrap().gap_1()
                                .children(choices.into_iter().map(|choice| {
                                    let owner = item.clone();
                                    Button::new(format!("mde-quick-visibility-{choice}"),choice)
                                        .toggle_state(current_visibility==choice)
                                        .on_click(move |_, _, cx| {
                                            owner.update(cx, |item,cx| {
                                                item.set_selected_property(
                                                    "properties.visibility".into(),
                                                    Value::String(choice.into()),cx
                                                );
                                            });
                                        })
                                })),
                        ).into_any_element(),
                );
            },
            QuickEditControl::Button => {
                let kind = match action.id {
                    "uml.add-attribute" => ModelingMemberKind::Attribute,
                    "uml.add-operation" => ModelingMemberKind::Operation,
                    _ => continue,
                };
                let owner = item.clone();
                active.push(
                    Button::new(format!("mde-quick-{}",action.id), action.title)
                        .on_click(move |_, _, cx| {
                            owner.update(cx, |item,cx| {
                                item.add_member_to_selected(kind,cx);
                            });
                        })
                        .into_any_element(),
                );
            },
        }
    }
    div()
        .id("bellatrix-mde-quick-edit")
        .w(px(300.0))
        .max_h(px(340.0))
        .flex().flex_col()
        .overflow_y_scroll()
        .border_1().border_color(palette.border)
        .rounded_sm()
        .bg(palette.panel)
        .p_2().gap_2()
        // The Quick Edit does not begin a Canvas move gesture.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div().font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(palette.text)
                .child(format!("Quick Edit · {}", element.kind))
        )
        .children(active)
        .when(!pending.is_empty(),|this|this.child(
            div().pt_2().border_t_1().border_color(palette.border)
                .child(Label::new("Other actions — domain contracts pending"))
                .children(pending)
        ))
        .into_any_element()
}
