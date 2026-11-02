//! Inspector Property Editor using native Zed controls, driven by MDE schemas.
//! GPUI renders a projection. Every edit crosses the typed Cordis bridge.
use crate::{MdeWorkbenchItem, WorkbenchPalette, panels::MdeRightPanel};
use bellatrix_mde_modeling::{ElementInspector, InspectorProperty, PropertyEditor};
use gpui::{AnyElement, Context, Entity, IntoElement, Window, div, prelude::*, px};
use serde_json::Value;
use std::collections::HashMap;
use ui::{Button, ContextMenu, DropdownMenu, Label, prelude::*};
use ui_input::InputField;

fn display(value: &Value) -> String {
    value.as_str().map_or_else(|| value.to_string(), str::to_owned)
}

fn section_title(group: &str) -> AnyElement {
    div().w_full().pt_3().pb_1()
        .child(Label::new(group.to_owned())).into_any_element()
}

fn property_row(
    property: &InspectorProperty,
    item: &Entity<MdeWorkbenchItem>,
    inputs: &HashMap<String, Entity<InputField>>,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> AnyElement {
    let palette = WorkbenchPalette::current(cx);
    let path = property.path.clone();
    let control: AnyElement = match property.editor {
        PropertyEditor::Text | PropertyEditor::Multiline | PropertyEditor::Number
            if property.editable => {
            if let Some(input) = inputs.get(&path) {
                let input = input.clone();
                let editor = input.clone();
                let target = item.clone();
                let numeric = property.editor == PropertyEditor::Number;
                div().flex().flex_col().gap_1().w_full()
                    .child(input)
                    .child(
                        Button::new(format!("mde-inspector-apply-{path}"), "Apply")
                            .on_click(move |_, _, cx| {
                                let text = editor.read(cx).text(cx);
                                let value = if numeric {
                                    serde_json::from_str::<Value>(&text)
                                        .unwrap_or(Value::String(text))
                                } else {
                                    Value::String(text)
                                };
                                target.update(cx, |workbench, cx| {
                                    workbench.set_selected_property(path.clone(), value, cx);
                                });
                            }),
                    ).into_any_element()
            } else {
                Label::new(display(&property.value)).into_any_element()
            }
        },
        PropertyEditor::Boolean if property.editable => {
            let target = item.clone();
            let selected = property.value.as_bool().unwrap_or(false);
            Button::new(format!("mde-inspector-boolean-{path}"),
                if selected { "☑ True" } else { "☐ False" })
                .toggle_state(selected)
                .on_click(move |_, _, cx| {
                    target.update(cx, |workbench, cx| {
                        workbench.set_selected_property(path.clone(),
                            Value::Bool(!selected), cx);
                    });
                }).into_any_element()
        },
        PropertyEditor::Choice(options) if property.editable => {
            let target = item.clone();
            let choices_path = path.clone();
            let menu = ContextMenu::build(window, cx, move |menu, _, _| {
                options.iter().fold(menu, |menu, option| {
                    let target = target.clone();
                    let path = choices_path.clone();
                    let selected = *option;
                    menu.entry(selected, None, move |_, cx| {
                        target.update(cx, |workbench, cx| {
                            workbench.set_selected_property(path.clone(),
                                Value::String(selected.into()), cx);
                        });
                    })
                })
            });
            DropdownMenu::new(format!("mde-inspector-choice-{path}"),
                display(&property.value), menu)
                .full_width(true)
                .into_any_element()
        },
        _ => div().text_color(palette.muted)
            .child(Label::new(display(&property.value)))
            .into_any_element(),
    };
    div().w_full().flex().flex_col().gap_1()
        .child(Label::new(property.label.clone()))
        .child(control)
        .into_any_element()
}

/// Recomputed from the selected semantic Element on every model revision.
/// Attribute and Operation groups navigate to owned model Elements.
pub(crate) fn render_inspector(
    inspector: Option<&ElementInspector>,
    item: Entity<MdeWorkbenchItem>,
    inputs: &HashMap<String, Entity<InputField>>,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> Vec<AnyElement> {
    let Some(inspector) = inspector else {
        return vec![Label::new("Select a model element to inspect its properties")
            .into_any_element()];
    };
    let mut children = vec![
        Label::new(format!("{} · {}", inspector.name, inspector.kind))
            .into_any_element(),
    ];
    let mut group = None;
    for property in &inspector.properties {
        if group != Some(property.group) {
            children.push(section_title(property.group));
            group = Some(property.group);
        }
        children.push(property_row(property, &item, inputs, window, cx));
    }
    for (title, members) in [
        ("Attributes", &inspector.attributes),
        ("Operations", &inspector.operations),
    ] {
        children.push(section_title(title));
        if members.is_empty() {
            children.push(Label::new("—").into_any_element());
        } else {
            for member in members {
                let target = item.clone();
                let id = member.id.clone();
                children.push(div().pl(px(8.0)).child(
                    Button::new(format!("mde-inspector-owned-{id}"), member.name.clone())
                        .on_click(move |_, _, cx| {
                            target.update(cx, |workbench, cx| {
                                workbench.select_element(id.clone(), cx);
                            });
                        })
                ).into_any_element());
            }
        }
    }
    children
}
