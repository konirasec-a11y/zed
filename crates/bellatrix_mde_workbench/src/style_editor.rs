use crate::property_editor::compact_row;
use crate::style_drafts::{FIELDS, StyleEditState, StyleSubmission, field_error, style_value};
use crate::{MdeWorkbenchItem, panels::MdeRightPanel};
use bellatrix_mde::{RelationshipRouting, View};
use bellatrix_mde_cordis::{
    MdeUiCommand, ModelingSetViewRoutingRequested, ModelingSetViewStyleRequested,
};
use gpui::{AnyElement, Context, Entity, IntoElement, Window, div, prelude::*, px};
use std::collections::HashMap;
use ui::{Button, ContextMenu, DropdownMenu, Label, prelude::*};
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
        && item_state.active_diagram.as_deref() == Some(key.1.as_str())
        && item_state.selected_view.as_deref() == Some(key.2.as_str())
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
            .map(|(field, _)| {
                let input = cx.new(|cx| {
                    let input = InputField::new(window, cx, "Inherited").label_min_width(px(0.0));
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
                                crate::property_editor::flush_property_edits(panel, &item, cx);
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
                crate::property_editor::flush_property_edits(panel, &item, cx);
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
        || crate::property_editor::property_command_pending(panel)
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
            .and_then(|diagram| diagram.views.iter().find(|view| view.id == key.2))
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
            crate::property_editor::flush_property_edits(panel, &item, cx);
            flush_style_edits(panel, &item, cx);
            cx.notify();
        }) {
            eprintln!("MDE style completion refresh failed: {error}");
        }
    })
    .detach();
    cx.notify();
}

pub(crate) fn render_styles(
    target: Option<(&str, &View, bool)>,
    item: Entity<MdeWorkbenchItem>,
    draft: Option<&StyleDraft>,
    open: bool,
    window: &mut Window,
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
    if item.read(cx).source().is_read_only() {
        for (key, label) in FIELDS {
            if is_edge && *key == "fill" {
                continue;
            }
            let value = style_value(&view.style, key);
            rows.push(compact_row(
                *label,
                Label::new(if value.is_empty() {
                    "Inherited".into()
                } else {
                    value
                })
                .size(LabelSize::Small),
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
    let key = (
        item.read(cx).project_id().as_str().to_owned(),
        diagram_id.to_owned(),
        view.id.clone(),
    );
    for (key, label) in FIELDS {
        if is_edge && *key == "fill" {
            continue;
        }
        if let Some(input) = draft.inputs.get(*key) {
            let color = input.read(cx).text(cx);
            let swatch = gpui::Rgba::try_from(color.trim()).ok();
            rows.push(compact_row(
                *label,
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(matches!(*key, "fill" | "stroke" | "text_color"), |row| {
                        row.child(
                            div()
                                .size(px(16.0))
                                .flex_none()
                                .border_1()
                                .border_color(cx.theme().colors().border)
                                .when_some(swatch, |swatch, color| swatch.bg(color)),
                        )
                    })
                    .child(div().flex_1().min_w_0().child(input.clone())),
            ));
        }
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
    let theme_key = key.clone();
    let theme_item = item.clone();
    rows.push(
        div()
            .px_3()
            .child(
                Button::new(
                    "mde-style-restore-theme",
                    if draft.state.dirty() {
                        "Restore IDE theme and discard draft"
                    } else {
                        "Restore IDE theme"
                    },
                )
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
            )
            .into_any_element(),
    );
    rows.push(
        div()
            .px_3()
            .child(
                Label::new("Changes update immediately. Blank values inherit the theme.")
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .into_any_element(),
    );
    if is_edge {
        let owner = cx.weak_entity();
        let menu = ContextMenu::build(window, cx, {
            move |menu, _, _| {
                [
                    ("Straight", RelationshipRouting::Straight),
                    ("Automatic", RelationshipRouting::Automatic),
                    ("Curve", RelationshipRouting::Curve),
                ]
                .into_iter()
                .fold(menu, |menu, (label, routing)| {
                    let owner = owner.clone();
                    let item = item.clone();
                    let key = key.clone();
                    menu.entry(label, None, move |_, cx| {
                        if let Err(error) = owner.update(cx, |panel, cx| {
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
                        }) {
                            eprintln!("MDE routing edit failed: {error}");
                        }
                    })
                })
            }
        });
        rows.push(compact_row(
            "Routing",
            DropdownMenu::new(
                "mde-style-routing",
                routing_label(draft.state.displayed_routing()),
                menu,
            )
            .full_width(true),
        ));
        rows.push(
            div()
                .px_3()
                .child(
                    Label::new("UML controls arrowheads and line patterns.")
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
                .into_any_element(),
        );
    }
    rows
}

fn routing_label(routing: Option<RelationshipRouting>) -> &'static str {
    match routing {
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
        let command = if let Some(routing) = submission.routing {
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
