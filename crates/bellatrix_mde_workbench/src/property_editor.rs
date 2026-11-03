//! Native Zed controls driven by the single MDE inspector schema.
use crate::{
    MdeWorkbenchItem, WorkbenchPalette,
    panels::{MdeRightPanel, element_glyph},
};
use bellatrix_mde::{ElementInspector, InspectorProperty, PropertyEditor};
use bellatrix_ui_panels::PropertyRow;
use bellatrix_ui_symbols::glyph;
use gpui::{AnyElement, Context, Entity, Focusable, IntoElement, Window, div, prelude::*, px, svg};
use serde_json::Value;
use std::collections::HashMap;
use ui::{
    Button, Checkbox, ContextMenu, DropdownMenu, IconButton, IconName, Label, ToggleState, Tooltip,
    prelude::*,
};
use ui_input::InputField;

fn display(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

fn display_kind(kind: &str) -> &str {
    bellatrix_mde::semantic_identity(kind).type_label
}

fn section_title(group: &str) -> AnyElement {
    div()
        .w_full()
        .px_3()
        .pt_3()
        .pb_1()
        .child(
            Label::new(group.to_uppercase())
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
        .into_any_element()
}

pub(crate) fn compact_row(
    label: impl Into<gpui::SharedString>,
    control: impl IntoElement,
) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .child(
            div().w(px(112.0)).flex_none().child(
                Label::new(label)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            ),
        )
        .child(div().flex_1().min_w_0().child(control))
        .into_any_element()
}

fn property_row(
    property: &InspectorProperty,
    item: &Entity<MdeWorkbenchItem>,
    inputs: &HashMap<String, Entity<InputField>>,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> AnyElement {
    let path = property.path.clone();
    if path == "kind" {
        let kind = display(&property.value);
        return div()
            .id("mde-inspector-kind")
            .w_full()
            .tooltip(Tooltip::text(format!("Stored type: {kind}")))
            .child(PropertyRow::new(
                property.label.clone(),
                display_kind(&kind).to_owned(),
            ))
            .into_any_element();
    }
    if property.editor == PropertyEditor::Multiline && property.editable {
        if let Some(input) = inputs.get(&path) {
            let editor = input.clone();
            let target = item.clone();
            let palette = WorkbenchPalette::current(cx);
            let text_area = input.read(cx).editor().render(window, cx);
            return div()
                .px_3()
                .py_1()
                .w_full()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .min_h(px(112.0))
                        .w_full()
                        .p_2()
                        .border_1()
                        .border_color(palette.border)
                        .bg(palette.surface)
                        .rounded_sm()
                        .child(text_area),
                )
                .child(
                    Button::new("mde-save-documentation", "Save documentation").on_click(
                        move |_, _, cx| {
                            let text = editor.read(cx).text(cx);
                            target.update(cx, |workbench, cx| {
                                workbench.set_selected_property(
                                    path.clone(),
                                    Value::String(text),
                                    cx,
                                )
                            });
                        },
                    ),
                )
                .into_any_element();
        }
    }
    let control: AnyElement = match property.editor {
        PropertyEditor::Text | PropertyEditor::Multiline | PropertyEditor::Number
            if property.editable =>
        {
            if let Some(input) = inputs.get(&path) {
                let input = input.clone();
                let editor = input.clone();
                let target = item.clone();
                let numeric = property.editor == PropertyEditor::Number;
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .w_full()
                    .min_w_0()
                    .child(div().flex_1().min_w_0().child(input))
                    .child(
                        IconButton::new(format!("mde-inspector-apply-{path}"), IconName::Check)
                            .icon_size(IconSize::Small)
                            .aria_label(format!("Apply {}", property.label))
                            .tooltip(Tooltip::text(format!("Apply {}", property.label)))
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
                    )
                    .into_any_element()
            } else {
                Label::new(display(&property.value))
                    .size(LabelSize::Small)
                    .into_any_element()
            }
        }
        PropertyEditor::Boolean if property.editable => {
            let target = item.clone();
            let selected = property.value.as_bool().unwrap_or(false);
            Checkbox::new(
                format!("mde-inspector-boolean-{path}"),
                ToggleState::from(selected),
            )
            .tooltip(Tooltip::text(property.label.clone()))
            .on_click(move |state, _, cx| {
                target.update(cx, |workbench, cx| {
                    workbench.set_selected_property(
                        path.clone(),
                        Value::Bool(*state == ToggleState::Selected),
                        cx,
                    );
                });
            })
            .into_any_element()
        }
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
                            workbench.set_selected_property(
                                path.clone(),
                                Value::String(selected.into()),
                                cx,
                            );
                        });
                    })
                })
            });
            DropdownMenu::new(
                format!("mde-inspector-choice-{path}"),
                display(&property.value),
                menu,
            )
            .full_width(true)
            .into_any_element()
        }
        _ => {
            return compact_row(
                property.label.clone(),
                Label::new(display(&property.value)).size(LabelSize::Small),
            );
        }
    };
    if property.editor == PropertyEditor::Multiline {
        div()
            .px_3()
            .py_1()
            .w_full()
            .child(control)
            .into_any_element()
    } else {
        compact_row(property.label.clone(), control)
    }
}

fn is_general(path: &str) -> bool {
    matches!(
        path,
        "name"
            | "kind"
            | "label"
            | "from"
            | "to"
            | "properties.visibility"
            | "properties.stereotype"
            | "properties.namespace"
            | "properties.declaration"
            | "properties.signature"
            | "properties.type"
            | "properties.defaultValue"
            | "properties.name"
            | "properties.sourceMultiplicity"
            | "properties.targetMultiplicity"
            | "properties.sourceRole"
            | "properties.targetRole"
            | "properties.navigable"
            | "properties.isAbstract"
            | "properties.isFinalSpecialization"
            | "properties.isLeaf"
            | "properties.isActive"
    )
}

pub(crate) fn render_inspector(
    inspector: Option<&ElementInspector>,
    item: Entity<MdeWorkbenchItem>,
    inputs: &HashMap<String, Entity<InputField>>,
    documentation_open: bool,
    advanced_open: bool,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> Vec<AnyElement> {
    let Some(inspector) = inspector else {
        return vec![
            div()
                .p_3()
                .child(
                    Label::new("Select an element in the diagram or model explorer to inspect it.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
        ];
    };
    let palette = WorkbenchPalette::current(cx);
    let mut children = vec![
        div()
            .px_3()
            .py_1()
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(palette.border)
            .child(
                svg()
                    .data(glyph(element_glyph(&inspector.kind)))
                    .size(px(20.0))
                    .flex_none()
                    .text_color(palette.muted),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        Label::new(inspector.display_label.clone())
                            .weight(gpui::FontWeight::SEMIBOLD),
                    )
                    .child(
                        Label::new(format!("{} · Selected", display_kind(&inspector.kind)))
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
            .into_any_element(),
        section_title("Properties"),
    ];
    // Keep every schema field reachable without letting uncommon metadata bury the primary controls.
    for property in inspector
        .properties
        .iter()
        .filter(|property| is_general(&property.path) && property.path != "kind")
    {
        children.push(property_row(property, &item, inputs, window, cx));
    }
    for (title, members) in [
        ("Attributes", &inspector.attributes),
        ("Operations", &inspector.operations),
        ("Receptions", &inspector.receptions),
        ("Template parameters", &inspector.template_parameters),
        ("Ports", &inspector.ports),
        ("Parts", &inspector.parts),
    ] {
        if members.is_empty() {
            continue;
        }
        children.push(section_title(&format!("{title} ({})", members.len())));
        for member in members {
            let target = item.clone();
            let id = member.id.clone();
            children.push(
                div()
                    .px_3()
                    .child(
                        Button::new(
                            format!("mde-inspector-owned-{id}"),
                            member.display_label.clone(),
                        )
                        .on_click(move |_, _, cx| {
                            target.update(cx, |workbench, cx| {
                                workbench.select_element(id.clone(), cx)
                            });
                        }),
                    )
                    .into_any_element(),
            );
        }
    }
    if inspector
        .properties
        .iter()
        .any(|property| property.path == "description")
    {
        children.push(
            div()
                .px_3()
                .pt_2()
                .child(
                    Button::new("mde-inspector-documentation", "Documentation")
                        .start_icon(
                            Icon::new(if documentation_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .size(IconSize::Small),
                        )
                        .on_click(cx.listener(|panel, _, window, cx| {
                            if panel.inspector_documentation_open {
                                window.focus(&panel.focus_handle(cx), cx);
                            }
                            panel.inspector_documentation_open =
                                !panel.inspector_documentation_open;
                            cx.notify();
                        })),
                )
                .into_any_element(),
        );
        if documentation_open {
            for property in inspector
                .properties
                .iter()
                .filter(|property| property.path == "description")
            {
                children.push(property_row(property, &item, inputs, window, cx));
            }
        }
    }
    let advanced: Vec<_> = inspector
        .properties
        .iter()
        .filter(|property| !is_general(&property.path) && property.path != "description")
        .collect();
    if !advanced.is_empty() {
        children.push(
            div()
                .px_3()
                .pt_2()
                .child(
                    Button::new(
                        "mde-inspector-advanced",
                        format!("Advanced ({})", advanced.len()),
                    )
                    .start_icon(
                        Icon::new(if advanced_open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(IconSize::Small),
                    )
                    .on_click(cx.listener(|panel, _, window, cx| {
                        if panel.inspector_advanced_open {
                            window.focus(&panel.focus_handle(cx), cx);
                        }
                        panel.inspector_advanced_open = !panel.inspector_advanced_open;
                        cx.notify();
                    })),
                )
                .into_any_element(),
        );
        if advanced_open {
            let mut group = None;
            for property in advanced {
                if group != Some(property.group) {
                    children.push(section_title(property.group));
                    group = Some(property.group);
                }
                children.push(property_row(property, &item, inputs, window, cx));
            }
        }
    }
    children
}

#[cfg(test)]
mod tests {
    use super::{display_kind, is_general};

    #[test]
    fn identity_uses_domain_metaclasses_without_changing_unknown_types() {
        assert_eq!(display_kind("rust.struct"), "Class");
        assert_eq!(display_kind("rust.trait"), "Interface");
        assert_eq!(display_kind("rust.function"), "Operation");
        assert_eq!(display_kind("custom.type"), "custom.type");
        assert_eq!(display_kind("uml.association"), "Association");
        assert_eq!(
            display_kind("uml.interface_realization"),
            "Interface Realization"
        );
    }

    #[test]
    fn primary_properties_stay_visible_and_metadata_is_advanced() {
        for path in [
            "name",
            "kind",
            "properties.visibility",
            "from",
            "to",
            "properties.declaration",
            "properties.signature",
            "properties.type",
            "properties.defaultValue",
            "properties.name",
            "properties.sourceMultiplicity",
            "properties.targetRole",
            "properties.navigable",
            "properties.isAbstract",
            "properties.isFinalSpecialization",
            "properties.isLeaf",
            "properties.isActive",
        ] {
            assert!(is_general(path));
        }
        for path in ["id", "description", "properties.custom"] {
            assert!(!is_general(path));
        }
    }
}
