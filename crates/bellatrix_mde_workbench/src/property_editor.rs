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

fn same_value(editor: PropertyEditor, left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if editor != PropertyEditor::Number {
        return false;
    }
    matches!((serde_json::from_str::<Value>(left), serde_json::from_str::<Value>(right)),
        (Ok(left), Ok(right)) if left.is_number() && right.is_number() && left == right)
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
    pub input: Option<Entity<InputField>>,
    discrete: Option<Value>,
    last_observed: String,
    admitted_text: Option<String>,
    state: DraftState,
    pending: Option<(
        String,
        tokio::sync::watch::Receiver<Option<Result<u64, String>>>,
    )>,
    _listener: Option<gpui::Subscription>,
}

impl InspectorDraft {
    fn text(&self, cx: &gpui::App) -> String {
        self.input
            .as_ref()
            .map(|input| input.read(cx).text(cx))
            .or_else(|| self.discrete.as_ref().map(display))
            .unwrap_or_else(|| self.state.baseline.clone())
    }
}

pub(crate) fn reconcile_inputs(
    drafts: &mut HashMap<DraftKey, InspectorDraft>,
    item: Entity<MdeWorkbenchItem>,
    project: &str,
    inspector: Option<&ElementInspector>,
    revision: u64,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) {
    if drafts
        .iter()
        .any(|(key, draft)| key.project == project && draft.state.queued)
    {
        let item = item.clone();
        let owner = cx.weak_entity();
        cx.defer(move |cx| {
            if let Err(error) = owner.update(cx, |panel, cx| {
                flush_property_edits(panel, &item, cx);
                crate::style_editor::flush_style_edits(panel, &item, cx);
            }) {
                eprintln!("MDE property queue refresh failed: {error}");
            }
        });
    }
    let Some(inspector) = inspector else { return };
    for property in &inspector.properties {
        if !property.editable {
            continue;
        }
        let key = DraftKey {
            project: project.into(),
            element: inspector.element_id.clone(),
            path: property.path.clone(),
        };
        let canonical = display(&property.value);
        let draft = drafts.entry(key.clone()).or_insert_with(|| {
            let input = matches!(
                property.editor,
                PropertyEditor::Text | PropertyEditor::Multiline | PropertyEditor::Number
            )
            .then(|| {
                cx.new(|cx| {
                    let input =
                        InputField::new(window, cx, &property.label).label_min_width(px(0.0));
                    if property.editor == PropertyEditor::Multiline {
                        input.editor().set_multiline(Some(12), window, cx);
                    }
                    input.set_text(&canonical, window, cx);
                    input
                })
            });
            let listener = input.as_ref().map(|input| {
                let owner = cx.weak_entity();
                let event_key = key.clone();
                let event_item = item.clone();
                let numeric = property.editor == PropertyEditor::Number;
                input.read(cx).editor().clone().subscribe(
                    Box::new(move |event, _, cx| {
                        if event == ErasedEditorEvent::BufferEdited {
                            let owner = owner.clone();
                            let key = event_key.clone();
                            let item = event_item.clone();
                            cx.defer(move |cx| {
                                if let Err(error) = owner.update(cx, |panel, cx| {
                                    let admitted =
                                        property_admission_allowed(panel, &item, &key, cx);
                                    if let Some(draft) = panel.property_drafts.get_mut(&key) {
                                        let text = draft.text(cx);
                                        // Editor notifications are deferred, so a transient suppression flag cannot distinguish projection from typing.
                                        if text == draft.last_observed {
                                            return;
                                        }
                                        draft.last_observed = text.clone();
                                        if !admitted {
                                            draft.state.error = Some(
                                                "Edit not synced: the active selection changed"
                                                    .into(),
                                            );
                                            cx.notify();
                                            return;
                                        }
                                        if numeric
                                            && !serde_json::from_str::<Value>(&text)
                                                .is_ok_and(|value| value.is_number())
                                        {
                                            draft.state.error = Some("Enter a valid number".into());
                                        } else {
                                            draft.admitted_text = Some(text);
                                            draft.state.edited();
                                        }
                                    }
                                    flush_property_edits(panel, &item, cx);
                                    cx.notify();
                                }) {
                                    eprintln!("MDE inspector refresh failed: {error}");
                                }
                            });
                        }
                    }),
                    window,
                    cx,
                )
            });
            InspectorDraft {
                input,
                discrete: None,
                last_observed: canonical.clone(),
                admitted_text: None,
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
                draft.state.acknowledge(submitted, receipt);
                draft.pending = None;
            }
        }
        if draft.pending.is_none() {
            let text = draft.text(cx);
            // Numeric formatting is presentation state; acknowledging 1e2 as 100.0 must not move the caret.
            let reconciled = if revision >= draft.state.revision
                && same_value(property.editor, &text, &canonical)
            {
                draft.state.reload(&text, revision);
                text.clone()
            } else {
                draft.state.reconcile(&text, &canonical, revision)
            };
            if text != reconciled {
                draft.last_observed = reconciled.clone();
                if let Some(input) = &draft.input {
                    input.update(cx, |input, cx| input.set_text(&reconciled, window, cx));
                } else {
                    draft.discrete = None;
                }
            } else if text == canonical && !draft.state.queued {
                draft.discrete = None;
            }
        }
    }
}

fn property_admission_allowed(
    panel: &MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    key: &DraftKey,
    cx: &gpui::App,
) -> bool {
    panel.is_bound_to(item)
        && !item.read(cx).source().is_read_only()
        && item.read(cx).project_id().as_str() == key.project
        && item.read(cx).selected_element() == Some(key.element.as_str())
}

fn queue_discrete_property(
    panel: &mut MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    key: &DraftKey,
    value: Value,
    cx: &mut Context<MdeRightPanel>,
) {
    if !property_admission_allowed(panel, item, key, cx) {
        return;
    }
    if let Some(draft) = panel.property_drafts.get_mut(key) {
        draft.admitted_text = Some(display(&value));
        draft.discrete = Some(value);
        draft.state.edited();
    }
    flush_property_edits(panel, item, cx);
    cx.notify();
}

pub(crate) fn property_command_pending(panel: &MdeRightPanel) -> bool {
    panel
        .property_drafts
        .values()
        .any(|draft| draft.pending.is_some())
}

pub(crate) fn flush_property_edits(
    panel: &mut MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    cx: &mut Context<MdeRightPanel>,
) {
    if property_command_pending(panel) || crate::style_editor::style_command_pending(panel) {
        return;
    }
    let project = item.read(cx).project_id().clone();
    if item.read(cx).source().is_read_only() {
        let mut changed = false;
        for (key, draft) in &mut panel.property_drafts {
            if key.project == project.as_str() && draft.state.queued {
                let message = "Waiting for this document to become editable";
                changed |= draft.state.error.as_deref() != Some(message);
                draft.state.error = Some(message.into());
            }
        }
        if changed {
            cx.notify();
        }
        return;
    }
    let snapshot = match item.read(cx).source().snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let message = format!("Project snapshot unavailable: {error}");
            let mut changed = false;
            for (key, draft) in &mut panel.property_drafts {
                if key.project == project.as_str() && draft.state.queued {
                    changed |= draft.state.error.as_ref() != Some(&message);
                    draft.state.error = Some(message.clone());
                }
            }
            if changed {
                cx.notify();
            }
            return;
        }
    };
    let keys: Vec<_> = panel
        .property_drafts
        .iter()
        .filter(|(key, draft)| key.project == project.as_str() && draft.state.queued)
        .map(|(key, _)| key.clone())
        .collect();
    let mut changed = false;
    for key in keys {
        let Some(draft) = panel.property_drafts.get_mut(&key) else {
            continue;
        };
        if snapshot.revision < draft.state.revision {
            continue;
        }
        draft.state.queued = false;
        changed = true;
        let Some(text) = draft.admitted_text.clone() else {
            continue;
        };
        let Some(inspection) = bellatrix_mde::inspect_element(&snapshot.section, &key.element)
        else {
            draft.state.error = Some("Element no longer exists".into());
            continue;
        };
        let Some(property) = inspection
            .properties
            .iter()
            .find(|property| property.path == key.path)
        else {
            continue;
        };
        let canonical = display(&property.value);
        if same_value(property.editor, &text, &canonical) {
            draft.state.reload(&text, snapshot.revision);
            continue;
        }
        if !same_value(property.editor, &canonical, &draft.state.baseline) {
            draft.state.conflict = true;
            continue;
        }
        draft.state.conflict = false;
        draft.state.revision = snapshot.revision;
        let value = if let Some(value) = &draft.discrete {
            value.clone()
        } else if property.editor == PropertyEditor::Number {
            match serde_json::from_str::<Value>(&text) {
                Ok(value) if value.is_number() => value,
                _ => {
                    draft.state.error = Some("Enter a valid number".into());
                    continue;
                }
            }
        } else {
            Value::String(text.clone())
        };
        draft.state.error = None;
        let completion = item.update(cx, |workbench, cx| {
            workbench.submit_admitted_inspector_property(
                project.clone(),
                key.element.clone(),
                key.path.clone(),
                value,
                snapshot.revision,
                cx,
            )
        });
        match completion {
            Err(error) => {
                draft.state.error = Some(error);
            }
            Ok(mut completion) => {
                draft.pending = Some((text, completion.clone()));
                let item = item.clone();
                cx.spawn(async move |owner, cx| {
                    if completion.borrow().is_none() {
                        if let Err(error) = completion.changed().await {
                            eprintln!("MDE property receipt unavailable: {error}");
                        }
                    }
                    if let Err(error) = owner.update(cx, |panel, cx| {
                        let receipt = completion
                            .borrow()
                            .clone()
                            .unwrap_or_else(|| Err("Property command channel closed".into()));
                        if let Some(draft) = panel.property_drafts.get_mut(&key) {
                            if draft
                                .pending
                                .as_ref()
                                .is_some_and(|(_, current)| current.same_channel(&completion))
                            {
                                if let Some((submitted, _)) = draft.pending.take() {
                                    draft.state.acknowledge(&submitted, receipt);
                                }
                            }
                        }
                        flush_property_edits(panel, &item, cx);
                        crate::style_editor::flush_style_edits(panel, &item, cx);
                        cx.notify();
                    }) {
                        eprintln!("MDE property completion refresh failed: {error}");
                    }
                })
                .detach();
                break;
            }
        }
    }
    if changed {
        cx.notify();
    }
}

fn property_row(
    property: &InspectorProperty,
    item: &Entity<MdeWorkbenchItem>,
    drafts: &HashMap<DraftKey, InspectorDraft>,
    project: &ProjectId,
    element: &str,
    _revision: u64,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> AnyElement {
    if item.read(cx).source().is_read_only() {
        // A captured preview presents canonical values, never another document's drafts.
        return compact_row(
            property.label.clone(),
            Label::new(display(&property.value)).size(LabelSize::Small),
        );
    }
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
            let dirty = draft.state.dirty(&draft.text(cx));
            let pending = draft.pending.is_some();
            let conflict = draft.state.conflict;
            let status = if let Some(error) = &draft.state.error {
                format!("Not synced: {error}")
            } else if pending {
                "Updating…".to_owned()
            } else if conflict {
                "Conflict: model changed. Reload to continue.".into()
            } else if dirty {
                "Waiting to sync…".into()
            } else {
                "".into()
            };
            let Some(field) = &draft.input else {
                return compact_row(label, Label::new(display(&property.value)));
            };
            let input = if property.editor == PropertyEditor::Multiline {
                div()
                    .min_h(px(112.0))
                    .child(field.read(cx).editor().render(window, cx))
                    .into_any_element()
            } else {
                field.clone().into_any_element()
            };
            let reload_key = key.clone();
            let reload_item = item.clone();
            div()
                .flex()
                .flex_col()
                .gap_1()
                .w_full()
                .min_w_0()
                .child(input)
                .when(!status.is_empty(), |row| {
                    row.child(
                        Label::new(status)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    )
                })
                .when(conflict, |row| {
                    row.child(
                        Button::new(format!("mde-inspector-reload-{path}"), "Reload model value")
                            .disabled(pending)
                            .on_click(cx.listener(move |panel, _, window, cx| {
                                let Ok(snapshot) = reload_item.read(cx).source().snapshot() else {
                                    return;
                                };
                                if reload_item.read(cx).project_id().as_str() != reload_key.project
                                {
                                    return;
                                }
                                let Some(inspection) = bellatrix_mde::inspect_element(
                                    &snapshot.section,
                                    &reload_key.element,
                                ) else {
                                    return;
                                };
                                let Some(property) = inspection
                                    .properties
                                    .iter()
                                    .find(|property| property.path == reload_key.path)
                                else {
                                    return;
                                };
                                let Some(draft) = panel.property_drafts.get_mut(&reload_key) else {
                                    return;
                                };
                                if draft.pending.is_some() {
                                    return;
                                }
                                let canonical = display(&property.value);
                                draft.state.reload(&canonical, snapshot.revision);
                                draft.state.queued = false;
                                draft.last_observed = canonical.clone();
                                if let Some(input) = &draft.input {
                                    input.update(cx, |input, cx| {
                                        input.set_text(&canonical, window, cx)
                                    });
                                }
                                draft.discrete = None;
                                cx.notify();
                            })),
                    )
                })
                .into_any_element()
        }
        PropertyEditor::Boolean if property.editable => {
            let target = item.clone();
            let selected = drafts
                .get(&key)
                .and_then(|draft| draft.discrete.as_ref())
                .unwrap_or(&property.value)
                .as_bool()
                .unwrap_or(false);
            Checkbox::new(
                format!("mde-inspector-boolean-{path}"),
                ToggleState::from(selected),
            )
            .tooltip(Tooltip::text(label.clone()))
            .on_click(cx.listener(move |panel, state, _, cx| {
                queue_discrete_property(
                    panel,
                    &target,
                    &key,
                    Value::Bool(*state == ToggleState::Selected),
                    cx,
                );
            }))
            .into_any_element()
        }
        PropertyEditor::Choice(options) if property.editable => {
            let target = item.clone();
            let panel = cx.weak_entity();
            let choice_key = key.clone();
            let menu = ContextMenu::build(window, cx, move |menu, _, _| {
                options.iter().fold(menu, |menu, option| {
                    let target = target.clone();
                    let key = choice_key.clone();
                    let panel = panel.clone();
                    let selected = *option;
                    menu.entry(selected, None, move |_, cx| {
                        if let Err(error) = panel.update(cx, |panel, cx| {
                            queue_discrete_property(
                                panel,
                                &target,
                                &key,
                                Value::String(selected.into()),
                                cx,
                            )
                        }) {
                            eprintln!("MDE property choice update failed: {error}");
                        }
                    })
                })
            });
            let value = drafts
                .get(&key)
                .and_then(|draft| draft.discrete.as_ref())
                .unwrap_or(&property.value);
            DropdownMenu::new(format!("mde-inspector-choice-{path}"), display(value), menu)
                .full_width(true)
                .into_any_element()
        }
        _ => Label::new(display(&property.value))
            .size(LabelSize::Small)
            .into_any_element(),
    };
    if matches!(
        property.editor,
        PropertyEditor::Boolean | PropertyEditor::Choice(_)
    ) {
        let key = DraftKey {
            project: project.as_str().into(),
            element,
            path,
        };
        let status = drafts.get(&key).and_then(|draft| {
            if let Some(error) = &draft.state.error {
                Some(format!("Not synced: {error}"))
            } else if draft.state.conflict {
                Some("Conflict: model changed".into())
            } else if draft.pending.is_some() || draft.state.queued {
                Some("Updating…".into())
            } else {
                None
            }
        });
        let conflict = drafts
            .get(&key)
            .is_some_and(|draft| draft.state.conflict && draft.pending.is_none());
        let reload_item = item.clone();
        return compact_row(
            label,
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(control)
                .when_some(status, |row, status| {
                    row.child(
                        Label::new(status)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    )
                })
                .when(conflict, |row| {
                    row.child(
                        Button::new(
                            format!("mde-inspector-reload-{}", key.path),
                            "Reload model value",
                        )
                        .on_click(cx.listener(move |panel, _, _, cx| {
                            let Ok(snapshot) = reload_item.read(cx).source().snapshot() else {
                                return;
                            };
                            if reload_item.read(cx).project_id().as_str() != key.project {
                                return;
                            }
                            let Some(inspection) =
                                bellatrix_mde::inspect_element(&snapshot.section, &key.element)
                            else {
                                return;
                            };
                            let Some(property) = inspection
                                .properties
                                .iter()
                                .find(|property| property.path == key.path)
                            else {
                                return;
                            };
                            let Some(draft) = panel.property_drafts.get_mut(&key) else {
                                return;
                            };
                            if draft.pending.is_some() {
                                return;
                            }
                            draft
                                .state
                                .reload(&display(&property.value), snapshot.revision);
                            draft.state.queued = false;
                            draft.discrete = None;
                            cx.notify();
                        })),
                    )
                }),
        );
    }
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
    let selection_status = if item.read(cx).source().is_read_only() {
        "Read-only"
    } else {
        "Selected"
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
                        Label::new(format!(
                            "{} · {selection_status}",
                            display_kind(&inspector.kind)
                        ))
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
