use crate::workbench_ui::property_editor::compact_row;
use crate::workbench_ui::style_drafts::{
    FIELDS, StyleEditState, StyleSubmission, field_error, style_value,
};
use crate::workbench_ui::{MdeWorkbenchItem, panels::MdeRightPanel};
use bellatrix_mde_cordis::projection::{
    InheritedStyleProperty, StyleInheritanceDiagnostic, StyleInheritanceSource,
    StyleInheritanceStatus, View,
};
use bellatrix_mde_cordis::{
    MdeUiCommand, ModelingReorderViewRequested, ModelingSetViewRoutingRequested,
    ModelingSetViewStyleRequested, RelationshipRouting, ViewOrder,
};
use gpui::{AnyElement, Context, Entity, IntoElement, Window, div, prelude::*, px};
use std::collections::HashMap;
use ui::{Button, IconButton, Label, Tooltip, prelude::*};
use ui_input::{ErasedEditorEvent, InputField};

type Completion = tokio::sync::watch::Receiver<Option<Result<u64, String>>>;
pub(crate) type StyleKey = (String, String, String);

pub(crate) struct StyleDraft {
    pub inputs: HashMap<String, Entity<InputField>>,
    state: StyleEditState,
    observed: HashMap<String, String>,
    errors: HashMap<String, Option<&'static str>>,
    completion: Option<Completion>,
    _listeners: Vec<gpui::Subscription>,
}

impl StyleDraft {
    fn capture_inputs(&mut self, admitted_field: Option<&str>, cx: &mut Context<MdeRightPanel>) {
        for (key, input) in &self.inputs {
            let text = input.read(cx).text(cx);
            if self.observed.get(key) != Some(&text) {
                if admitted_field == Some(key.as_str()) {
                    self.observed.insert(key.clone(), text.clone());
                    self.state.edit(key, text);
                } else {
                    self.state.preserve_input(key, text);
                }
            }
            let error = self
                .state
                .values
                .get(key)
                .and_then(|text| field_error(key, text));
            if self.errors.get(key) != Some(&error) {
                self.errors.insert(key.clone(), error);
                input.update(cx, |field, cx| field.set_error(error, cx));
            }
        }
    }

    fn acknowledge(&mut self, completion: &Completion, cx: &mut Context<MdeRightPanel>) {
        if !self
            .completion
            .as_ref()
            .is_some_and(|pending| pending.same_channel(completion))
        {
            return;
        }
        let receipt = completion.borrow().clone().or_else(|| {
            completion
                .has_changed()
                .is_err()
                .then(|| Err("Style command channel closed".into()))
        });
        if let Some(receipt) = receipt {
            self.capture_inputs(None, cx);
            self.state.acknowledge(receipt);
            self.completion = None;
        }
    }

    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<MdeRightPanel>) {
        for (key, input) in &self.inputs {
            let Some(value) = self.state.values.get(key) else {
                continue;
            };
            if input.read(cx).text(cx) != *value {
                // BufferEdited is delivered after set_text returns. Remember the value rather
                // than temporarily disabling a listener, which would replay the canonical edit.
                self.observed.insert(key.clone(), value.clone());
                input.update(cx, |field, cx| field.set_text(value, window, cx));
            }
            let error = field_error(key, value);
            if self.errors.get(key) != Some(&error) {
                self.errors.insert(key.clone(), error);
                input.update(cx, |field, cx| field.set_error(error, cx));
            }
        }
    }
}

fn target_is_active(
    panel: &MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    key: &StyleKey,
    cx: &gpui::App,
) -> bool {
    let item_state = item.read(cx);
    panel.is_bound_to(item)
        && !item_state.source().is_read_only()
        && item_state.project_id().as_str() == key.0
        && item_state.presentation.active_diagram() == Some(key.1.as_str())
        && item_state.presentation.selected_view() == Some(key.2.as_str())
        && item_state.source().snapshot().ok().is_some_and(|snapshot| {
            snapshot.section.diagrams.iter().any(|diagram| {
                diagram.id == key.1
                    && super::selection::supports_view_style_editing(diagram, &key.2)
            })
        })
}

pub(crate) fn reconcile_style_inputs(
    drafts: &mut HashMap<StyleKey, StyleDraft>,
    item: &Entity<MdeWorkbenchItem>,
    diagram_id: &str,
    view: &View,
    revision: u64,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) {
    let key = (
        item.read(cx).project_id().as_str().to_owned(),
        diagram_id.into(),
        view.id.clone(),
    );
    let draft = drafts.entry(key.clone()).or_insert_with(|| {
        let state = StyleEditState::new(&view.style, revision);
        let observed = state.values.clone();
        let mut listeners = Vec::new();
        let inputs = FIELDS
            .iter()
            .filter(|(field, _)| {
                !matches!(
                    *field,
                    "text_align"
                        | "text_vertical_align"
                        | "line_mode"
                        | "word_wrap"
                        | "show_shadow"
                        | "stereotype_display"
                        | "show_visibility"
                        | "show_type"
                        | "show_property"
                        | "show_multiplicity"
                        | "show_operation_signature"
                        | "suppress_attributes"
                        | "suppress_operations"
                        | "suppress_receptions"
                )
            })
            .map(|(field, _)| {
                let input = cx.new(|cx| {
                    let input = InputField::new(window, cx, "Not set").label_min_width(px(0.0));
                    input.set_text(&style_value(&view.style, field), window, cx);
                    input
                });
                let owner = cx.weak_entity();
                let key = key.clone();
                let item = item.downgrade();
                let field = *field;
                listeners.push(input.read(cx).editor().clone().subscribe(
                    Box::new(move |event, _, cx| {
                        if event != ErasedEditorEvent::BufferEdited {
                            return;
                        }
                        let owner = owner.clone();
                        let key = key.clone();
                        let item = item.clone();
                        cx.defer(move |cx| {
                            let Some(item) = item.upgrade() else { return };
                            if let Err(error) = owner.update(cx, |panel, cx| {
                                if !target_is_active(panel, &item, &key, cx) {
                                    return;
                                }
                                if let Some(draft) = panel.style_drafts.get_mut(&key) {
                                    draft.capture_inputs(Some(field), cx);
                                }
                                crate::workbench_ui::property_editor::flush_property_edits(
                                    panel, &item, cx,
                                );
                                flush_style_edits(panel, &item, cx);
                                cx.notify();
                            }) {
                                eprintln!("MDE style input refresh failed: {error}");
                            }
                        });
                    }),
                    window,
                    cx,
                ));
                (field.to_owned(), input)
            })
            .collect();
        StyleDraft {
            inputs,
            state,
            observed,
            errors: HashMap::new(),
            completion: None,
            _listeners: listeners,
        }
    });
    draft.capture_inputs(None, cx);
    if let Some(completion) = draft.completion.clone() {
        draft.acknowledge(&completion, cx);
    }
    draft.state.reconcile(&view.style, revision);
    draft.sync_inputs(window, cx);
    if draft.state.needs_flush() {
        let owner = cx.weak_entity();
        let item = item.clone();
        cx.defer(move |cx| {
            if let Err(error) = owner.update(cx, |panel, cx| {
                crate::workbench_ui::property_editor::flush_property_edits(panel, &item, cx);
                flush_style_edits(panel, &item, cx);
            }) {
                eprintln!("MDE style synchronization failed: {error}");
            }
        });
    }
}

pub(crate) fn style_command_pending(panel: &MdeRightPanel) -> bool {
    panel
        .style_drafts
        .values()
        .any(|draft| draft.state.pending.is_some())
}

pub(crate) fn flush_style_edits(
    panel: &mut MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    cx: &mut Context<MdeRightPanel>,
) {
    if !panel.is_bound_to(item)
        || item.read(cx).source().is_read_only()
        || style_command_pending(panel)
        || crate::workbench_ui::property_editor::property_command_pending(panel)
    {
        return;
    }
    let project = item.read(cx).project_id().as_str().to_owned();
    let keys: Vec<_> = panel
        .style_drafts
        .iter()
        .filter(|(key, draft)| key.0 == project && draft.state.needs_flush())
        .map(|(key, _)| key.clone())
        .collect();
    if keys.is_empty() {
        return;
    }
    let snapshot = match item.read(cx).source().snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            for key in keys {
                if let Some(draft) = panel.style_drafts.get_mut(&key) {
                    draft.state.error = Some(error.clone());
                }
            }
            cx.notify();
            return;
        }
    };
    let mut ready = None;
    for key in keys {
        let Some(draft) = panel.style_drafts.get_mut(&key) else {
            continue;
        };
        if snapshot.revision < draft.state.revision {
            continue;
        }
        let Some(view) = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| diagram.id == key.1)
            .and_then(|diagram| diagram.all_views().find(|view| view.id == key.2))
        else {
            draft.state.error = Some("The original diagram view no longer exists".into());
            continue;
        };
        draft.capture_inputs(None, cx);
        draft.state.reconcile(&view.style, snapshot.revision);
        if let Some(submission) = draft.state.next_submission() {
            ready = Some((key, submission));
            break;
        }
    }
    let Some((key, submission)) = ready else {
        cx.notify();
        return;
    };
    let Some(draft) = panel.style_drafts.get_mut(&key) else {
        return;
    };
    let (sender, mut completion) = tokio::sync::watch::channel(None);
    let result = item.update(cx, |item, cx| {
        item.submit_style_edit(&key, &submission, sender, cx)
    });
    if let Err(error) = result {
        draft.state.acknowledge(Err(error));
        cx.notify();
        return;
    }
    draft.completion = Some(completion.clone());
    let item = item.clone();
    cx.spawn(async move |owner, cx| {
        if completion.borrow().is_none() {
            if let Err(error) = completion.changed().await {
                eprintln!("MDE style receipt unavailable: {error}");
            }
        }
        if let Err(error) = owner.update(cx, |panel, cx| {
            if let Some(draft) = panel.style_drafts.get_mut(&key) {
                draft.acknowledge(&completion, cx);
            }
            crate::workbench_ui::property_editor::flush_property_edits(panel, &item, cx);
            flush_style_edits(panel, &item, cx);
            cx.notify();
        }) {
            eprintln!("MDE style completion refresh failed: {error}");
        }
    })
    .detach();
    cx.notify();
}

fn edit_style_choice(
    panel: &mut MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    key: &StyleKey,
    field: &str,
    value: &str,
    cx: &mut Context<MdeRightPanel>,
) {
    if !target_is_active(panel, item, key, cx) {
        return;
    }
    if let Some(draft) = panel.style_drafts.get_mut(key) {
        if draft.state.conflict {
            return;
        }
        draft.state.edit(field, value.into());
    }
    flush_style_edits(panel, item, cx);
    cx.notify();
}

fn inheritance_property_label(property: InheritedStyleProperty) -> &'static str {
    match property {
        InheritedStyleProperty::Fill => "Fill",
        InheritedStyleProperty::Stroke => "Stroke",
        InheritedStyleProperty::TextColor => "Text color",
        InheritedStyleProperty::FontFamily => "Font family",
        InheritedStyleProperty::FontSize => "Font size",
        InheritedStyleProperty::FontWeight => "Font weight",
        InheritedStyleProperty::FontItalic => "Font italic",
    }
}

fn inherited_render_value(view: &View, property: InheritedStyleProperty) -> String {
    let style = &view.render_style;
    match property {
        InheritedStyleProperty::Fill => style.fill.clone(),
        InheritedStyleProperty::Stroke => style.stroke.clone(),
        InheritedStyleProperty::TextColor => style.text_color.clone(),
        InheritedStyleProperty::FontFamily => style.font_family.clone(),
        InheritedStyleProperty::FontSize => style.font_size.map(|value| value.to_string()),
        InheritedStyleProperty::FontWeight => style.font_weight.map(|value| value.to_string()),
        InheritedStyleProperty::FontItalic => style.font_italic.map(|value| value.to_string()),
    }
    .unwrap_or_else(|| "Theme / notation default".into())
}

fn field_inheritance_note(view: &View, field: &str) -> Option<(&'static str, Color)> {
    let property = match field {
        "fill" => InheritedStyleProperty::Fill,
        "stroke" => InheritedStyleProperty::Stroke,
        "text_color" => InheritedStyleProperty::TextColor,
        "font_family" => InheritedStyleProperty::FontFamily,
        "font_size" => InheritedStyleProperty::FontSize,
        _ => return None,
    };
    let inheritance = view.presentation.style_inheritance.as_ref()?;
    if inheritance.status == StyleInheritanceStatus::Inactive
        || !inheritance.properties.contains(&property)
    {
        return None;
    }
    if inheritance.origins.contains_key(&property) {
        Some(("Authored only · appearance inherited", Color::Muted))
    } else {
        Some(("Authored only · inheritance unresolved", Color::Warning))
    }
}

fn render_style_inheritance(view: &View) -> Vec<AnyElement> {
    let Some(inheritance) = &view.presentation.style_inheritance else {
        return Vec::new();
    };
    let (status, color) = match inheritance.status {
        StyleInheritanceStatus::Inactive => ("Inactive", Color::Muted),
        StyleInheritanceStatus::Resolved => ("Resolved", Color::Muted),
        StyleInheritanceStatus::Unresolved if inheritance.origins.is_empty() => {
            ("Unresolved", Color::Warning)
        }
        StyleInheritanceStatus::Unresolved => ("Partially resolved", Color::Warning),
    };
    let source = match &inheritance.source {
        StyleInheritanceSource::Missing => "Missing source · no inheritance".to_owned(),
        StyleInheritanceSource::Null => "Null source · no inheritance".to_owned(),
        StyleInheritanceSource::Reference(source_id) => format!("Reference {source_id}"),
    };
    let properties = if inheritance.properties.is_empty() {
        "None · no inheritance".to_owned()
    } else {
        inheritance
            .properties
            .iter()
            .map(|property| inheritance_property_label(*property))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut rows = vec![
        compact_row(
            "Style inheritance",
            Label::new(status).size(LabelSize::Small).color(color),
        ),
        compact_row("Source", Label::new(source).size(LabelSize::Small)),
        compact_row("Fields", Label::new(properties).size(LabelSize::Small)),
        div()
            .px_3()
            .py_1()
            .child(
                Label::new("Limited inheritance: fill, stroke, text color and font family, size, weight and italic. Other fields remain local. Full StarUML appearance is not reproduced.")
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .into_any_element(),
    ];
    if inheritance.status == StyleInheritanceStatus::Inactive {
        if let Some(source_id) = inheritance.diagnostics.values().find_map(|diagnostic| {
            match diagnostic {
                StyleInheritanceDiagnostic::NonViewSource { source_id } => Some(source_id),
                _ => None,
            }
        }) {
            rows.push(
                div()
                    .px_3()
                    .py_1()
                    .child(
                        Label::new(format!("Source {source_id} has no view style. Inheritance is inactive; this view's authored styles remain in use."))
                            .size(LabelSize::XSmall)
                            .color(Color::Warning),
                    )
                    .into_any_element(),
            );
        }
        return rows;
    }
    for property in &inheritance.properties {
        let value = inherited_render_value(view, *property);
        let (detail, color) = if let Some(origin) = inheritance.origins.get(property) {
            (format!("{value} · inherited from view {origin}"), Color::Muted)
        } else {
            let diagnostic = match inheritance.diagnostics.get(property) {
                Some(StyleInheritanceDiagnostic::MissingSource { source_id }) => {
                    format!("Missing source {source_id}")
                }
                Some(StyleInheritanceDiagnostic::NonViewSource { source_id }) => {
                    format!("Source {source_id} is not a view")
                }
                Some(StyleInheritanceDiagnostic::Cycle { view_id }) => {
                    format!("Inheritance cycle at view {view_id}")
                }
                None => "Inheritance unresolved".to_owned(),
            };
            (
                format!("{diagnostic} · renderer / theme fallback; authored value is not used"),
                Color::Warning,
            )
        };
        rows.push(compact_row(
            inheritance_property_label(*property),
            Label::new(detail).size(LabelSize::Small).color(color),
        ));
    }
    rows.push(
        div()
            .px_3()
            .py_1()
            .child(
                Label::new("Values below are authored on this view. Inherited fields do not use these local values, even when their source is unresolved. Editing or clearing authored styles does not change inheritance.")
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .into_any_element(),
    );
    rows
}

pub(crate) fn render_styles(
    target: Option<(&str, &View, bool)>,
    item: Entity<MdeWorkbenchItem>,
    draft: Option<&StyleDraft>,
    read_only: bool,
    open: bool,
    _window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> Vec<AnyElement> {
    let mut rows = vec![
        div()
            .px_3()
            .pt_2()
            .child(
                Button::new("mde-styles-toggle", "Styles")
                    .start_icon(
                        Icon::new(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(IconSize::Small),
                    )
                    .on_click(cx.listener(|panel, _, window, cx| {
                        if panel.inspector_styles_open {
                            cx.focus_self(window);
                        }
                        panel.inspector_styles_open = !panel.inspector_styles_open;
                        cx.notify();
                    })),
            )
            .into_any_element(),
    ];
    if !open {
        return rows;
    }
    let Some((diagram_id, view, is_edge)) = target else {
        rows.push(
            div()
                .px_3()
                .py_1()
                .child(
                    Label::new("Select a diagram view to inspect its appearance.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
        );
        return rows;
    };
    rows.extend(render_style_inheritance(view));
    if read_only {
        for (key, label) in FIELDS {
            if is_edge && *key == "fill" {
                continue;
            }
            let value = style_value(&view.style, key);
            rows.push(compact_row(
                *label,
                div()
                    .child(
                        Label::new(if value.is_empty() {
                            "Not set".into()
                        } else {
                            value
                        })
                        .size(LabelSize::Small),
                    )
                    .when_some(field_inheritance_note(view, key), |row, (note, color)| {
                        row.child(Label::new(note).size(LabelSize::XSmall).color(color))
                    }),
            ));
        }
        if is_edge {
            rows.push(compact_row(
                "Routing",
                Label::new(routing_label(view.style.routing)).size(LabelSize::Small),
            ));
        }
        return rows;
    }
    let Some(draft) = draft else { return rows };
    let layout_editable = !item.read(cx).source().is_mdj()
        && item.read(cx).source().snapshot().ok().is_some_and(|snapshot| {
            snapshot.section.diagrams.iter().any(|diagram| {
                diagram.id == diagram_id
                    && super::selection::supports_view_layout_editing(diagram, &view.id)
            })
        });
    let key = (
        item.read(cx).project_id().as_str().to_owned(),
        diagram_id.to_owned(),
        view.id.clone(),
    );
    let mut fields = div()
        .w_full()
        .min_w_0()
        .px_3()
        .py_1()
        .flex()
        .flex_wrap()
        .gap_2();
    for field in [
        "font_family",
        "font_size",
        "fill",
        "stroke",
        "text_color",
        "stroke_width",
        "opacity",
    ] {
        if is_edge && field == "fill" {
            continue;
        }
        let Some(input) = draft.inputs.get(field) else {
            continue;
        };
        let (icon, description) = match field {
            "font_family" => (
                IconName::Font,
                "Authored font family · blank leaves it unset",
            ),
            "font_size" => (
                IconName::FontSize,
                "Authored font size · blank leaves it unset",
            ),
            "fill" => (IconName::Box, "Fill color · #RRGGBB or #RRGGBBAA"),
            "stroke" => (IconName::Pencil, "Outline color · #RRGGBB or #RRGGBBAA"),
            "text_color" => (IconName::CaseSensitive, "Text color · #RRGGBB or #RRGGBBAA"),
            "stroke_width" => (IconName::LineHeight, "Outline width"),
            _ => (IconName::Eye, "Opacity · 0 to 1"),
        };
        let inheritance_note = field_inheritance_note(view, field);
        let description = match inheritance_note {
            Some((note, _)) => {
                format!("{description}. {note}. This input edits the authored value only.")
            }
            None => description.to_owned(),
        };
        let color = input.read(cx).text(cx);
        let swatch = gpui::Rgba::try_from(color.trim()).ok();
        fields = fields.child(
            div()
                .flex_1()
                .flex_basis(px(if field == "font_family" { 180.0 } else { 150.0 }))
                .min_w_0()
                .max_w(px(240.0))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            IconButton::new(format!("mde-style-{field}-hint"), icon)
                                .aria_label(description.clone())
                                .tooltip(Tooltip::text(description)),
                        )
                        .when(matches!(field, "fill" | "stroke" | "text_color"), |cell| {
                            cell.child(
                                div()
                                    .size(px(10.0))
                                    .flex_none()
                                    .border_1()
                                    .border_color(cx.theme().colors().border)
                                    .when_some(swatch, |swatch, color| swatch.bg(color)),
                            )
                        })
                        .child(div().flex_1().min_w_0().child(input.clone())),
                )
                .when_some(inheritance_note, |cell, (note, color)| {
                    cell.child(Label::new(note).size(LabelSize::XSmall).color(color))
                }),
        );
    }
    rows.push(fields.into_any_element());
    let controls_index = rows.len();
    let mut controls = div()
        .w_full()
        .min_w_0()
        .px_3()
        .py_1()
        .flex()
        .flex_wrap()
        .items_start()
        .gap_2();

    let mut format = div()
        .max_w_full()
        .flex_none()
        .p_1()
        .flex()
        .flex_wrap()
        .gap_1()
        .border_1()
        .border_color(cx.theme().colors().border)
        .rounded_sm();
    for (field, icon, description) in [
        (
            "word_wrap",
            IconName::TextWrap,
            "Wrap text · click to toggle; restore theme to inherit",
        ),
        (
            "show_shadow",
            IconName::Copy,
            "Show view shadow · click to toggle; restore theme to inherit",
        ),
    ] {
        let enabled = draft
            .state
            .values
            .get(field)
            .is_some_and(|value| value == "true");
        let item = item.clone();
        let key = key.clone();
        format = format.child(
            Button::new(format!("mde-style-{field}"), "")
                .start_icon(if field == "show_shadow" {
                    Icon::from_path("icons/bellatrix-show-shadow.svg").size(IconSize::Small)
                } else {
                    Icon::new(icon).size(IconSize::Small)
                })
                .aria_label(description)
                .tooltip(Tooltip::text(description))
                .toggle_state(enabled)
                .disabled(draft.state.conflict)
                .on_click(cx.listener(move |panel, _, _, cx| {
                    edit_style_choice(
                        panel,
                        &item,
                        &key,
                        field,
                        if enabled { "false" } else { "true" },
                        cx,
                    );
                })),
        );
    }
    if !is_edge {
        for (field, description) in [
            (
                "suppress_attributes",
                "Hide attribute compartment · semantic attributes are preserved",
            ),
            (
                "suppress_operations",
                "Hide operation compartment · semantic operations are preserved",
            ),
            (
                "suppress_receptions",
                "Hide reception compartment · semantic receptions are preserved",
            ),
        ] {
            let enabled = draft
                .state
                .values
                .get(field)
                .is_some_and(|value| value == "true");
            let item = item.clone();
            let key = key.clone();
            format = format.child(
                Button::new(format!("mde-style-{field}"), "")
                    .start_icon(
                        Icon::from_path(format!("icons/bellatrix-{field}.svg"))
                            .size(IconSize::Small),
                    )
                    .aria_label(description)
                    .tooltip(Tooltip::text(description))
                    .toggle_state(enabled)
                    .disabled(draft.state.conflict)
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        edit_style_choice(
                            panel,
                            &item,
                            &key,
                            field,
                            if enabled { "false" } else { "true" },
                            cx,
                        )
                    })),
            );
        }
    }
    controls = controls.child(format);
    if !is_edge {
        let mut stereotypes = div()
            .max_w_full()
            .flex_none()
            .p_1()
            .flex()
            .flex_wrap()
            .gap_1()
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded_sm();
        for (value, icon, description) in [
            ("", IconName::RefreshTitle, "Inherit stereotype display"),
            (
                "label",
                IconName::CaseSensitive,
                "Show applied stereotype label",
            ),
            (
                "none",
                IconName::EyeOff,
                "Hide applied stereotype label · UML keywords remain visible",
            ),
        ] {
            let item = item.clone();
            let key = key.clone();
            stereotypes = stereotypes.child(
                IconButton::new(format!("mde-style-stereotype-{value}"), icon)
                    .aria_label(description)
                    .tooltip(Tooltip::text(description))
                    .toggle_state(
                        draft
                            .state
                            .values
                            .get("stereotype_display")
                            .is_some_and(|current| current == value),
                    )
                    .disabled(draft.state.conflict)
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        edit_style_choice(panel, &item, &key, "stereotype_display", value, cx)
                    })),
            );
        }
        if view.interface_icon.is_some() {
            for (value, label) in [("icon", "Interface icon"), ("icon_label", "Icon + label")] {
                let item = item.clone();
                let key = key.clone();
                stereotypes = stereotypes.child(
                    Button::new(format!("mde-style-stereotype-{value}"), label)
                        .tooltip(Tooltip::text("Use interface icon notation; stored compartment layout remains unchanged"))
                        .toggle_state(draft.state.values.get("stereotype_display").is_some_and(|current| current == value))
                        .disabled(draft.state.conflict)
                        .on_click(cx.listener(move |panel, _, _, cx| {
                            edit_style_choice(panel, &item, &key, "stereotype_display", value, cx)
                        })),
                );
            }
        }
        controls = controls.child(stereotypes);
    }
    if !is_edge {
        let mut labels = div()
            .max_w_full()
            .flex_none()
            .p_1()
            .flex()
            .flex_wrap()
            .gap_1()
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded_sm();
        for (field, description) in [
            ("show_visibility", "Show member visibility"),
            ("show_type", "Show member and parameter types"),
            (
                "show_property",
                "Show property annotations such as readOnly and query",
            ),
            ("show_multiplicity", "Show member multiplicity"),
            ("show_operation_signature", "Show operation signature"),
        ] {
            let enabled = draft
                .state
                .values
                .get(field)
                .is_none_or(|value| value != "false");
            let item = item.clone();
            let key = key.clone();
            let caption = match field {
                "show_multiplicity" => "Multiplicity",
                "show_operation_signature" => "Signature",
                _ => "",
            };
            let icon_field = match field {
                "show_multiplicity" | "show_operation_signature" => "show_type",
                _ => field,
            };
            labels = labels.child(
                Button::new(format!("mde-style-{field}"), caption)
                    .start_icon(
                        Icon::from_path(format!("icons/bellatrix-{icon_field}.svg"))
                            .size(IconSize::Small),
                    )
                    .aria_label(description)
                    .tooltip(Tooltip::text(description))
                    .toggle_state(enabled)
                    .disabled(draft.state.conflict)
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        edit_style_choice(
                            panel,
                            &item,
                            &key,
                            field,
                            if enabled { "false" } else { "true" },
                            cx,
                        )
                    })),
            );
        }
        controls = controls.child(labels);
    }
    let mut patterns = div()
        .max_w_full()
        .flex_none()
        .p_1()
        .flex()
        .flex_wrap()
        .gap_1()
        .border_1()
        .border_color(cx.theme().colors().border)
        .rounded_sm();
    for (value, glyph, description) in [
        ("", "inherit", "Inherit the notation's line pattern"),
        ("solid", "solid", "Solid outline · visual override only"),
        ("dotted", "dotted", "Dotted outline · visual override only"),
        ("dashed", "dashed", "Dashed outline · visual override only"),
    ] {
        let item = item.clone();
        let key = key.clone();
        patterns = patterns.child(
            Button::new(format!("mde-style-line-mode-{glyph}"), "")
                .start_icon(
                    Icon::from_path(format!("icons/bellatrix-line-{glyph}.svg"))
                        .size(IconSize::Small),
                )
                .aria_label(description)
                .tooltip(Tooltip::text(description))
                .toggle_state(draft.state.values.get("line_mode").is_some_and(|current| {
                    current == value
                        && (!value.is_empty() || draft.state.baseline.dash_pattern.is_empty())
                }))
                .disabled(draft.state.conflict)
                .on_click(cx.listener(move |panel, _, _, cx| {
                    edit_style_choice(panel, &item, &key, "line_mode", value, cx);
                })),
        );
    }
    if ["line_mode", "text_align", "text_vertical_align"]
        .iter()
        .any(|field| {
            draft
                .state
                .values
                .get(*field)
                .is_some_and(|value| !value.is_empty())
        })
        || !draft.state.baseline.dash_pattern.is_empty()
    {
        patterns = patterns.child(IconButton::new("mde-custom-notation-warning", IconName::Warning)
            .aria_label("Custom line or text placement may differ from standard UML notation")
            .icon_color(Color::Warning)
            .tooltip(Tooltip::text("Custom line or text placement may differ from standard UML notation. Semantic meaning is unchanged. Restore IDE theme / inherit to return to the renderer’s notation defaults.")));
    }
    controls = controls.child(patterns);
    let mut alignment = div()
        .max_w_full()
        .flex_none()
        .p_1()
        .flex()
        .flex_wrap()
        .gap_1()
        .border_1()
        .border_color(cx.theme().colors().border)
        .rounded_sm();
    for (field, value, glyph, description) in [
        ("text_align", "left", "left", "Align text left"),
        ("text_align", "center", "center", "Center text horizontally"),
        ("text_align", "right", "right", "Align text right"),
        ("text_vertical_align", "top", "top", "Align text top"),
        (
            "text_vertical_align",
            "center",
            "middle",
            "Center text vertically",
        ),
        (
            "text_vertical_align",
            "bottom",
            "bottom",
            "Align text bottom",
        ),
    ] {
        let item = item.clone();
        let key = key.clone();
        alignment = alignment.child(
            Button::new(format!("mde-style-{field}-{value}"), "")
                .start_icon(
                    Icon::from_path(format!("icons/bellatrix-text-align-{glyph}.svg"))
                        .size(IconSize::Small),
                )
                .aria_label(description)
                .tooltip(Tooltip::text(description))
                .toggle_state(
                    draft
                        .state
                        .values
                        .get(field)
                        .is_some_and(|current| current == value),
                )
                .disabled(draft.state.conflict)
                .on_click(cx.listener(move |panel, _, _, cx| {
                    edit_style_choice(panel, &item, &key, field, value, cx);
                })),
        );
    }
    controls = controls.child(alignment);
    let mut ordering = div()
        .max_w_full()
        .flex_none()
        .p_1()
        .flex()
        .flex_wrap()
        .gap_1()
        .border_1()
        .border_color(cx.theme().colors().border)
        .rounded_sm();
    for (id, description, order) in [
        ("front", "Bring to front", ViewOrder::BringToFront),
        (
            "forward",
            "Bring forward one layer",
            ViewOrder::BringForward,
        ),
        (
            "backward",
            "Send backward one layer",
            ViewOrder::SendBackward,
        ),
        ("back", "Send to back", ViewOrder::SendToBack),
    ] {
        let item = item.clone();
        let key = key.clone();
        ordering = ordering.child(
            Button::new(format!("mde-style-order-{id}"), "")
                .start_icon(
                    Icon::from_path(format!("icons/bellatrix-order-{id}.svg"))
                        .size(IconSize::Small),
                )
                .aria_label(description)
                .tooltip(Tooltip::text(description))
                .disabled(draft.state.conflict || draft.state.pending.is_some())
                .on_click(cx.listener(move |panel, _, _, cx| {
                    if !target_is_active(panel, &item, &key, cx) {
                        return;
                    }
                    if let Some(draft) = panel.style_drafts.get_mut(&key) {
                        if draft.state.conflict || draft.state.pending.is_some() {
                            return;
                        }
                        draft.state.reorder(order);
                    }
                    flush_style_edits(panel, &item, cx);
                    cx.notify();
                })),
        );
    }
    if layout_editable {
        controls = controls.child(ordering);
    }
    if draft.state.conflict || draft.state.error.is_some() {
        let reload_key = key.clone();
        let target = item.clone();
        let status = if draft.state.conflict {
            "This style changed elsewhere. Your text is preserved; reload to continue.".to_owned()
        } else {
            format!(
                "Not updated: {}",
                draft.state.error.as_deref().unwrap_or_default()
            )
        };
        rows.push(
            div()
                .px_3()
                .child(
                    Label::new(status)
                        .size(LabelSize::XSmall)
                        .color(Color::Warning),
                )
                .child(
                    Button::new("mde-style-reload", "Discard draft and reload").on_click(
                        cx.listener(move |panel, _, _, cx| {
                            if !panel.is_bound_to(&target)
                                || target.read(cx).source().is_read_only()
                            {
                                return;
                            }
                            if panel
                                .style_drafts
                                .get(&reload_key)
                                .is_some_and(|draft| draft.state.pending.is_none())
                            {
                                panel.style_drafts.remove(&reload_key);
                            }
                            cx.notify();
                        }),
                    ),
                )
                .into_any_element(),
        );
    }
    if draft.state.pending.is_some() {
        rows.push(
            div()
                .px_3()
                .child(
                    Label::new("Updating…")
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
                .into_any_element(),
        );
    }
    if is_edge && layout_editable {
        let mut routing_row = div()
            .max_w_full()
            .flex_none()
            .p_1()
            .flex()
            .flex_wrap()
            .gap_1()
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded_sm();
        for (id, description, routing) in [
            ("straight", "Straight line", RelationshipRouting::Straight),
            (
                "automatic",
                "Automatic orthogonal routing",
                RelationshipRouting::Automatic,
            ),
            ("curve", "Curved line", RelationshipRouting::Curve),
        ] {
            let item = item.clone();
            let key = key.clone();
            routing_row = routing_row.child(
                Button::new(format!("mde-style-routing-{id}"), "")
                    .start_icon(
                        Icon::from_path(format!("icons/bellatrix-route-{id}.svg"))
                            .size(IconSize::Small),
                    )
                    .aria_label(description)
                    .tooltip(Tooltip::text(description))
                    .toggle_state(draft.state.displayed_routing() == Some(routing))
                    .disabled(draft.state.conflict)
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        if !target_is_active(panel, &item, &key, cx) {
                            return;
                        }
                        if let Some(draft) = panel.style_drafts.get_mut(&key) {
                            if draft.state.conflict {
                                return;
                            }
                            draft.state.set_routing(routing);
                        }
                        flush_style_edits(panel, &item, cx);
                        cx.notify();
                    })),
            );
        }
        controls = controls.child(routing_row);
    }
    let theme_key = key.clone();
    let theme_item = item.clone();
    let inheritance_active = view
        .presentation
        .style_inheritance
        .as_ref()
        .is_some_and(|inheritance| inheritance.status != StyleInheritanceStatus::Inactive);
    let restore_description = if inheritance_active {
        if draft.state.dirty() {
            "Clear authored styles and discard draft; inheritance remains unchanged"
        } else {
            "Clear authored styles; inheritance remains unchanged"
        }
    } else if draft.state.dirty() {
        "Restore IDE theme and discard draft"
    } else {
        "Restore IDE theme"
    };
    controls = controls.child(
        div().flex_none().p_1().child(
            IconButton::new("mde-style-restore-theme", IconName::RefreshTitle)
                .aria_label(restore_description)
                .tooltip(Tooltip::text(restore_description))
                .disabled(draft.state.conflict || draft.state.pending.is_some())
                .on_click(cx.listener(move |panel, _, _, cx| {
                    if !target_is_active(panel, &theme_item, &theme_key, cx) {
                        return;
                    }
                    if let Some(draft) = panel.style_drafts.get_mut(&theme_key) {
                        if draft.state.conflict || draft.state.pending.is_some() {
                            return;
                        }
                        draft.state.restore_theme();
                    }
                    flush_style_edits(panel, &theme_item, cx);
                    cx.notify();
                })),
        ),
    );
    rows.insert(controls_index, controls.into_any_element());
    rows
}

fn routing_label(routing: Option<RelationshipRouting>) -> &'static str {
    match routing {
        Some(RelationshipRouting::Manual) => "Manual",
        Some(RelationshipRouting::Straight) => "Straight",
        Some(RelationshipRouting::Automatic) => "Automatic",
        Some(RelationshipRouting::Curve) => "Curve",
        None => "Stored geometry",
    }
}

impl MdeWorkbenchItem {
    fn submit_style_edit(
        &mut self,
        key: &StyleKey,
        submission: &StyleSubmission,
        completion: tokio::sync::watch::Sender<Option<Result<u64, String>>>,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if self.project_id().as_str() != key.0 || self.source().is_read_only() {
            return Err(
                "Select the original editable diagram view before changing its style".into(),
            );
        }
        let snapshot = self.source().snapshot()?;
        if !snapshot.section.diagrams.iter().any(|diagram| {
            diagram.id == key.1 && super::selection::supports_view_style_editing(diagram, &key.2)
        }) {
            return Err("This view supports inspection only".into());
        }
        if (submission.order.is_some() || submission.routing.is_some())
            && (self.source().is_mdj()
                || !snapshot.section.diagrams.iter().any(|diagram| {
                    diagram.id == key.1
                        && super::selection::supports_view_layout_editing(diagram, &key.2)
                }))
        {
            return Err("Routing and stacking changes require a supported flat editable diagram".into());
        }
        let command = if let Some(order) = submission.order {
            MdeUiCommand::ReorderViewTracked {
                request: ModelingReorderViewRequested {
                    project_id: self.project_id().clone(),
                    expected_revision: submission.expected_revision,
                    diagram_id: key.1.clone(),
                    view_id: key.2.clone(),
                    order,
                },
                completion,
            }
        } else if let Some(routing) = submission.routing {
            MdeUiCommand::SetViewRoutingTracked {
                request: ModelingSetViewRoutingRequested {
                    project_id: self.project_id().clone(),
                    expected_revision: submission.expected_revision,
                    diagram_id: key.1.clone(),
                    view_id: key.2.clone(),
                    routing,
                },
                completion,
            }
        } else {
            MdeUiCommand::SetViewStyleTracked {
                request: ModelingSetViewStyleRequested {
                    project_id: self.project_id().clone(),
                    expected_revision: submission.expected_revision,
                    diagram_id: key.1.clone(),
                    view_id: key.2.clone(),
                    style: submission.style.clone(),
                },
                completion,
            }
        };
        self.source.submit(command)?;
        cx.notify();
        Ok(())
    }
}
