//! Native Zed controls driven by the single MDE inspector schema.
use crate::{
    MdeWorkbenchItem, WorkbenchPalette,
    inspector_drafts::{DraftKey, DraftState},
    panels::{MdeRightPanel, element_glyph},
};
use bellatrix_mde::{ElementInspector, InspectorProperty, PropertyEditor};
use bellatrix_project::ProjectId;
use bellatrix_ui_symbols::glyph;
use gpui::{AnyElement, Context, Entity, Focusable, IntoElement, Window, div, prelude::*, px, svg};
use serde_json::Value;
use std::collections::HashMap;
use ui::{
    Button, Checkbox, ContextMenu, DropdownMenu, IconName, Label, ToggleState, Tooltip, prelude::*,
};
use ui_input::{ErasedEditorEvent, InputField};

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

pub(crate) struct InspectorDraft {
    pub input: Entity<InputField>,
    state: DraftState,
    pending: Option<(
        String,
        tokio::sync::watch::Receiver<Option<Result<u64, String>>>,
    )>,
    _listener: gpui::Subscription,
}

pub(crate) fn reconcile_inputs(
    drafts: &mut HashMap<DraftKey, InspectorDraft>,
    project: &str,
    inspector: Option<&ElementInspector>,
    revision: u64,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) {
    let Some(inspector) = inspector else { return };
    for property in &inspector.properties {
        if !property.editable
            || !matches!(
                property.editor,
                PropertyEditor::Text | PropertyEditor::Multiline | PropertyEditor::Number
            )
        {
            continue;
        }
        let key = DraftKey {
            project: project.into(),
            element: inspector.element_id.clone(),
            path: property.path.clone(),
        };
        let canonical = display(&property.value);
        let draft = drafts.entry(key).or_insert_with(|| {
            let input = cx.new(|cx| {
                let input = InputField::new(window, cx, &property.label).label_min_width(px(0.0));
                if property.editor == PropertyEditor::Multiline {
                    input.editor().set_multiline(Some(12), window, cx);
                }
                input.set_text(&canonical, window, cx);
                input
            });
            let owner = cx.weak_entity();
            let listener = input.read(cx).editor().clone().subscribe(
                Box::new(move |event, _, cx| {
                    if event == ErasedEditorEvent::BufferEdited {
                        let owner = owner.clone();
                        cx.defer(move |cx| {
                            if let Err(error) = owner.update(cx, |_, cx| cx.notify()) {
                                eprintln!("MDE inspector refresh failed: {error}");
                            }
                        });
                    }
                }),
                window,
                cx,
            );
            InspectorDraft {
                input,
                state: DraftState::new(canonical.clone(), revision),
                pending: None,
                _listener: listener,
            }
        });
        if let Some((submitted, completion)) = &draft.pending {
            let receipt = completion.borrow().clone().or_else(|| {
                completion
                    .has_changed()
                    .is_err()
                    .then(|| Err("Property command channel closed".into()))
            });
            if let Some(receipt) = receipt {
                // Wait for the snapshot to reach the acknowledged revision before reconciling.
                if !matches!(&receipt, Ok(applied) if revision < *applied) {
                    draft.state.acknowledge(submitted, receipt);
                    draft.pending = None;
                }
            }
        }
        if draft.pending.is_none() {
            let text = draft.input.read(cx).text(cx);
            let reconciled = draft.state.reconcile(&text, &canonical, revision);
            if text != reconciled {
                draft
                    .input
                    .update(cx, |input, cx| input.set_text(&reconciled, window, cx));
            }
        }
    }
}

fn property_row(
    property: &InspectorProperty,
    item: &Entity<MdeWorkbenchItem>,
    drafts: &HashMap<DraftKey, InspectorDraft>,
    project: &ProjectId,
    element: &str,
    revision: u64,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> AnyElement {
    let path = property.path.clone();
    let key = DraftKey {
        project: project.as_str().into(),
        element: element.into(),
        path: path.clone(),
    };
    let label = property.label.clone();
    let project = project.clone();
    let element = element.to_owned();
    let control: AnyElement = match property.editor {
        PropertyEditor::Text | PropertyEditor::Multiline | PropertyEditor::Number
            if property.editable =>
        {
            let Some(draft) = drafts.get(&key) else {
                return compact_row(label, Label::new(display(&property.value)));
            };
            let dirty = draft.state.dirty(&draft.input.read(cx).text(cx));
            let pending = draft.pending.is_some();
            let conflict = draft.state.conflict;
            let status = if pending {
                "Applying…".to_owned()
            } else if conflict {
                "Conflict: model changed. Reload to continue.".into()
            } else if let Some(error) = &draft.state.error {
                format!("Not applied: {error}")
            } else if dirty {
                "Not applied".into()
            } else {
                "".into()
            };
            let input = if property.editor == PropertyEditor::Multiline {
                div()
                    .min_h(px(112.0))
                    .child(draft.input.read(cx).editor().render(window, cx))
                    .into_any_element()
            } else {
                draft.input.clone().into_any_element()
            };
            let target = item.clone();
            let apply_key = key.clone();
            let reload_key = key.clone();
            let reload_item = item.clone();
            let numeric = property.editor == PropertyEditor::Number;
            div().flex().flex_col().gap_1().w_full().min_w_0()
                .child(input)
                .when(!status.is_empty(), |row| row.child(Label::new(status).size(LabelSize::XSmall).color(Color::Muted)))
                .child(div().flex().gap_2()
                    .child(Button::new(format!("mde-inspector-apply-{path}"), "Apply")
                        .disabled(!dirty || conflict || pending)
                        .on_click(cx.listener(move |panel, _, _, cx| {
                            let Some(draft) = panel.property_drafts.get_mut(&apply_key) else { return };
                            if draft.pending.is_some() || draft.state.conflict { return; }
                            let text = draft.input.read(cx).text(cx);
                            let value = if numeric {
                                match serde_json::from_str::<Value>(&text) {
                                    Ok(value) if value.is_number() => value,
                                    _ => { draft.state.error = Some("Enter a valid number".into()); cx.notify(); return; }
                                }
                            } else { Value::String(text.clone()) };
                            let completion = target.update(cx, |workbench, cx| workbench.set_inspector_property(project.clone(), apply_key.element.clone(), apply_key.path.clone(), value, draft.state.revision, cx));
                            match completion {
                            Ok(mut completion) => {
                                draft.pending = Some((text, completion.clone()));
                                cx.spawn(async move |owner, cx| {
                                    if completion.borrow().is_none() {
                                        if let Err(error) = completion.changed().await { eprintln!("MDE property receipt unavailable: {error}"); }
                                    }
                                    if let Err(error) = owner.update(cx, |_, cx| cx.notify()) { eprintln!("MDE property completion refresh failed: {error}"); }
                                }).detach();
                            }
                            Err(error) => { draft.state.error = Some(error); }
                            }
                            cx.notify();
                        })))
                    .child(Button::new(format!("mde-inspector-reload-{path}"), if conflict { "Reload" } else { "Cancel" })
                        .disabled(pending || (!dirty && !conflict && draft.state.error.is_none()))
                        .on_click(cx.listener(move |panel, _, window, cx| {
                            let Ok(snapshot) = reload_item.read(cx).source().snapshot() else { return };
                            if reload_item.read(cx).project_id().as_str() != reload_key.project { return; }
                            let Some(inspection) = bellatrix_mde::inspect_element(&snapshot.section, &reload_key.element) else { return };
                            let Some(property) = inspection.properties.iter().find(|property| property.path == reload_key.path) else { return };
                            let Some(draft) = panel.property_drafts.get_mut(&reload_key) else { return };
                            if draft.pending.is_some() { return; }
                            let canonical = display(&property.value);
                            draft.state.reload(&canonical, snapshot.revision);
                            draft.input.update(cx, |input, cx| input.set_text(&canonical, window, cx));
                            cx.notify();
                        }))))
                .into_any_element()
        }
        PropertyEditor::Boolean if property.editable => {
            let target = item.clone();
            Checkbox::new(
                format!("mde-inspector-boolean-{path}"),
                ToggleState::from(property.value.as_bool().unwrap_or(false)),
            )
            .tooltip(Tooltip::text(label.clone()))
            .on_click(move |state, _, cx| {
                target.update(cx, |workbench, cx| {
                    if let Err(error) = workbench.set_inspector_property(
                        project.clone(),
                        element.clone(),
                        path.clone(),
                        Value::Bool(*state == ToggleState::Selected),
                        revision,
                        cx,
                    ) {
                        workbench.feedback = error;
                        cx.notify();
                    }
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
                    let project = project.clone();
                    let element = element.clone();
                    let selected = *option;
                    menu.entry(selected, None, move |_, cx| {
                        target.update(cx, |workbench, cx| {
                            if let Err(error) = workbench.set_inspector_property(
                                project.clone(),
                                element.clone(),
                                path.clone(),
                                Value::String(selected.into()),
                                revision,
                                cx,
                            ) {
                                workbench.feedback = error;
                                cx.notify();
                            }
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
        _ => Label::new(display(&property.value))
            .size(LabelSize::Small)
            .into_any_element(),
    };
    compact_row(label, control)
}

fn is_general(path: &str) -> bool {
    matches!(
        path,
        "name"
            | "kind"
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
    drafts: &HashMap<DraftKey, InspectorDraft>,
    revision: u64,
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
    let project = item.read(cx).project_id().clone();
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
        children.push(property_row(
            property,
            &item,
            drafts,
            &project,
            &inspector.element_id,
            revision,
            window,
            cx,
        ));
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
                children.push(property_row(
                    property,
                    &item,
                    drafts,
                    &project,
                    &inspector.element_id,
                    revision,
                    window,
                    cx,
                ));
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
                children.push(property_row(
                    property,
                    &item,
                    drafts,
                    &project,
                    &inspector.element_id,
                    revision,
                    window,
                    cx,
                ));
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
