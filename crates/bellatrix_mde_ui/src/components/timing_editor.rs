//! Authored timing samples are View presentation, not evaluated UML occurrences.
use crate::layouts::sidebar::MdeRightPanel;
use crate::pages::mde_page::MdeWorkbenchItem;
use crate::view_model::inspector_drafts::DraftState;
use bellatrix_mde_cordis::TimingMode;
use bellatrix_mde_cordis::TimingPoint;
use bellatrix_mde_cordis::TimingPresentation;
use bellatrix_mde_cordis::projection::View;
use gpui::AnyElement;
use gpui::Context;
use gpui::Entity;
use gpui::EntityId;
use gpui::Focusable;
use gpui::IntoElement;
use gpui::Subscription;
use gpui::Window;
use gpui::div;
use gpui::prelude::*;
use gpui::px;
use std::collections::HashMap;
use ui::Button;
use ui::Icon;
use ui::IconName;
use ui::IconSize;
use ui::Label;
use ui::Tooltip;
use ui::prelude::*;
use ui_input::ErasedEditorEvent;
use ui_input::InputField;

type TimingKey = (String, String, String);
type Completion = tokio::sync::watch::Receiver<Option<Result<u64, String>>>;
pub(crate) struct TimingDraft {
    mode: TimingMode,
    rows: Vec<(Entity<InputField>, Entity<InputField>)>,
    admitted_inputs: HashMap<EntityId, String>,
    queued_values: Option<TimingPresentation>,
    state: DraftState,
    pending: Option<(String, Completion)>,
    listeners: Vec<Subscription>,
}
impl TimingDraft {
    pub(crate) fn inspection_notice(&self) -> Option<&str> {
        self.state.error.as_deref().or_else(|| {
            (self.state.queued || self.pending.is_some())
                .then_some("Timing edits are retained while this view is read-only")
        })
    }
    pub(crate) fn has_focused_input(&self, window: &Window, cx: &gpui::App) -> bool {
        self.rows.iter().any(|(time, value)| {
            [time, value]
                .into_iter()
                .any(|input| input.read(cx).focus_handle(cx).contains_focused(window, cx))
        })
    }
    fn collect_values(
        &self,
        read: impl Fn(&Entity<InputField>) -> String,
    ) -> Result<TimingPresentation, String> {
        let points = self
            .rows
            .iter()
            .map(|(time, value)| {
                let time = read(time)
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Enter a finite time for each sample".to_owned())?;
                if !time.is_finite() {
                    return Err("Times must be finite".into());
                }
                Ok(TimingPoint {
                    time,
                    value: read(value),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(TimingPresentation {
            mode: self.mode,
            points,
        })
    }
    fn read_values(&self, cx: &Context<MdeRightPanel>) -> Result<TimingPresentation, String> {
        self.collect_values(|input| input.read(cx).text(cx))
    }
    fn admit_snapshot(&mut self) {
        let values = self.collect_values(|input| {
            self.admitted_inputs
                .get(&input.entity_id())
                .cloned()
                .unwrap_or_default()
        });
        self.state.edited();
        match values {
            Ok(values) => self.queued_values = Some(values),
            Err(error) => {
                self.queued_values = None;
                self.state.error = Some(error);
            }
        }
    }
    fn admit_input(&mut self, input_id: EntityId, text: String) {
        if self
            .rows
            .iter()
            .any(|(time, value)| time.entity_id() == input_id || value.entity_id() == input_id)
        {
            self.admitted_inputs.insert(input_id, text);
            self.admit_snapshot();
        }
    }
    fn acknowledge(&mut self) {
        let Some((submitted, receiver)) = &self.pending else {
            return;
        };
        let receipt = receiver.borrow().clone().or_else(|| {
            receiver
                .has_changed()
                .is_err()
                .then(|| Err("Timing command channel closed; your samples are preserved".into()))
        });
        if let Some(receipt) = receipt {
            let succeeded = receipt.is_ok();
            self.state.acknowledge(submitted, receipt);
            self.pending = None;
            if succeeded && self.state.queued && self.queued_values.is_none() {
                self.admit_snapshot();
            }
        }
    }
}
fn serialize_timing(timing: &TimingPresentation) -> String {
    serde_json::to_string(timing)
        .unwrap_or_else(|error| format!("Invalid timing presentation: {error}"))
}
fn is_active(
    panel: &MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    key: &TimingKey,
    cx: &gpui::App,
) -> bool {
    let item_state = item.read(cx);
    panel.is_bound_to(item)
        && !item_state.selected_view_is_inspection_only()
        && !item_state.source().is_read_only()
        && item_state.project_id().as_str() == key.0
        && item_state.presentation.active_diagram() == Some(key.1.as_str())
        && item_state.presentation.selected_view() == Some(key.2.as_str())
}
fn create_row(
    draft: &mut TimingDraft,
    key: &TimingKey,
    time: &str,
    value: &str,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) {
    let mut fields = Vec::new();
    for (placeholder, text) in [("Time", time), ("Value", value)] {
        let input = cx.new(|cx| {
            let input = InputField::new(window, cx, placeholder).label_min_width(px(0.0));
            input.set_text(text, window, cx);
            input
        });
        let input_id = input.entity_id();
        draft.admitted_inputs.insert(input_id, text.to_owned());
        let owner = cx.weak_entity();
        let key = key.clone();
        draft
            .listeners
            .push(input.read(cx).editor().clone().subscribe(
                Box::new(move |event, _, cx| {
                    if event != ErasedEditorEvent::BufferEdited {
                        return;
                    }
                    let Some(item) = owner
                        .upgrade()
                        .and_then(|panel| panel.read(cx).bound_item())
                    else {
                        return;
                    };
                    let admission = owner.read_with(cx, |panel, cx| {
                        if !is_active(panel, &item, &key, cx) {
                            return None;
                        }
                        panel.timing_drafts.get(&key).and_then(|draft| {
                            draft
                                .rows
                                .iter()
                                .flat_map(|(time, value)| [time, value])
                                .find(|input| input.entity_id() == input_id)
                                .map(|input| input.read(cx).text(cx))
                        })
                    });
                    let text = match admission {
                        Ok(Some(text)) => text,
                        Ok(None) => return,
                        Err(error) => {
                            eprintln!("MDE timing admission unavailable: {error}");
                            return;
                        }
                    };
                    let owner = owner.clone();
                    let key = key.clone();
                    cx.defer(move |cx| {
                        if let Err(error) = owner.update(cx, |panel, cx| {
                            if let Some(draft) = panel.timing_drafts.get_mut(&key)
                                && draft.rows.iter().any(|(time, value)| {
                                    time.entity_id() == input_id || value.entity_id() == input_id
                                })
                            {
                                draft.admit_input(input_id, text);
                            }
                            flush_timing_edits(panel, &item, &key, cx);
                            cx.notify();
                        }) {
                            eprintln!("MDE timing input update failed: {error}");
                        }
                    });
                }),
                window,
                cx,
            ));
        fields.push(input);
    }
    if let (Some(time), Some(value)) = (fields.first(), fields.get(1)) {
        draft.rows.push((time.clone(), value.clone()));
    }
}
pub(crate) fn reconcile_timing_inputs(
    drafts: &mut HashMap<TimingKey, TimingDraft>,
    item: &Entity<MdeWorkbenchItem>,
    diagram_id: &str,
    view: &View,
    revision: u64,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> TimingKey {
    let key = (
        item.read(cx).project_id().as_str().to_owned(),
        diagram_id.to_owned(),
        view.id.clone(),
    );
    let canonical = view.timing.clone().unwrap_or(TimingPresentation {
        mode: TimingMode::State,
        points: Vec::new(),
    });
    let canonical_text = serialize_timing(&canonical);
    let draft = drafts.entry(key.clone()).or_insert_with(|| {
        let mut draft = TimingDraft {
            mode: canonical.mode,
            rows: Vec::new(),
            admitted_inputs: HashMap::new(),
            queued_values: None,
            state: DraftState::new(canonical_text.clone(), revision),
            pending: None,
            listeners: Vec::new(),
        };
        for point in &canonical.points {
            create_row(
                &mut draft,
                &key,
                &point.time.to_string(),
                &point.value,
                window,
                cx,
            );
        }
        draft
    });
    draft.acknowledge();
    if draft.pending.is_none()
        && !draft.state.queued
        && let Ok(current) = draft.read_values(cx)
    {
        let text = serialize_timing(&current);
        let reconciled = draft.state.reconcile(&text, &canonical_text, revision);
        if reconciled != text {
            draft.mode = canonical.mode;
            draft.rows.clear();
            draft.listeners.clear();
            draft.admitted_inputs.clear();
            for point in &canonical.points {
                create_row(
                    draft,
                    &key,
                    &point.time.to_string(),
                    &point.value,
                    window,
                    cx,
                );
            }
        }
    }
    if draft.pending.is_none()
        && draft.state.queued
        && !draft.state.conflict
        && draft.state.error.is_none()
        && draft.queued_values.is_some()
    {
        let owner = cx.weak_entity();
        let item = item.clone();
        let key = key.clone();
        cx.defer(move |cx| {
            if let Err(error) =
                owner.update(cx, |panel, cx| flush_timing_edits(panel, &item, &key, cx))
            {
                eprintln!("MDE timing synchronization failed: {error}");
            }
        });
    }
    key
}
fn flush_timing_edits(
    panel: &mut MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    key: &TimingKey,
    cx: &mut Context<MdeRightPanel>,
) {
    let original_project = item.read(cx).project_id().as_str() == key.0;
    let source_suspended = item.read(cx).source().is_read_only();
    if !original_project {
        if let Some(draft) = panel.timing_drafts.get_mut(key) {
            draft.state.error =
                Some("The original document changed; admitted timing edits are preserved".into());
        }
        return;
    }
    let Some(draft) = panel.timing_drafts.get_mut(key) else {
        return;
    };
    if source_suspended {
        draft.state.error =
            Some("Timing source is suspended or read-only; admitted edits are preserved".into());
        return;
    }
    if draft.pending.is_some()
        || !draft.state.queued
        || draft.state.conflict
        || draft.state.error.is_some()
    {
        return;
    }
    let Some(timing) = draft.queued_values.clone() else {
        return;
    };
    let submitted = serialize_timing(&timing);
    let snapshot = match item.read(cx).source().snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            draft.state.error = Some(error);
            return;
        }
    };
    if !snapshot.section.diagrams.iter().any(|diagram| {
        diagram.id == key.1 && crate::view_model::selection::supports_view_editing(diagram, &key.2)
    }) {
        let error = format!(
            "Timing View {} is unavailable or inspection-only; admitted edits are preserved",
            key.2
        );
        draft.state.error = Some(error.clone());
        item.update(cx, |item, cx| {
            item.feedback = error;
            cx.notify();
        });
        cx.notify();
        return;
    }
    let Some(view) = snapshot
        .section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == key.1)
        .and_then(|diagram| diagram.all_views().find(|view| view.id == key.2))
    else {
        draft.state.error = Some("The original timing View no longer exists".into());
        return;
    };
    let canonical = view.timing.clone().unwrap_or(TimingPresentation {
        mode: TimingMode::State,
        points: Vec::new(),
    });
    draft
        .state
        .reconcile(&submitted, &serialize_timing(&canonical), snapshot.revision);
    if draft.state.conflict {
        cx.notify();
        return;
    }
    if !draft.state.dirty(&submitted) {
        draft.state.queued = false;
        draft.queued_values = None;
        return;
    }
    let expected_revision = draft.state.revision;
    draft.state.queued = false;
    draft.queued_values = None;
    let (sender, mut receiver) = tokio::sync::watch::channel(None);
    draft.pending = Some((submitted, receiver.clone()));
    item.update(cx, |item, cx| {
        let result =
            item.source
                .set_view_timing_tracked(&key.2, Some(timing), expected_revision, sender);
        item.report_command_result(result, cx);
    });
    let item = item.clone();
    let key = key.clone();
    let journal = item.read(cx).source().journal.clone();
    cx.spawn(async move |owner, cx| {
        let awaiting_receipt = receiver.borrow().is_none();
        if awaiting_receipt && receiver.changed().await.is_err() {
            journal.record_interaction(
                expected_revision,
                "view_timing.receipt_unavailable",
                Some(&key.2),
                bellatrix_mde_cordis::TraceOutcome::Uncertain,
            );
        }
        if let Err(error) = owner.update(cx, |panel, cx| {
            if let Some(draft) = panel.timing_drafts.get_mut(&key) {
                draft.acknowledge();
            }
            flush_timing_edits(panel, &item, &key, cx);
            cx.notify();
        }) {
            eprintln!("MDE timing completion update failed: {error}");
        }
    })
    .detach();
}
#[derive(Clone, Copy)]
enum TimingAction {
    Add,
    Remove(EntityId),
    Mode(TimingMode),
    Reload,
    Keep,
}
fn edit_timing(
    panel: &mut MdeRightPanel,
    item: &Entity<MdeWorkbenchItem>,
    key: &TimingKey,
    action: TimingAction,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) {
    if !is_active(panel, item, key, cx) {
        return;
    }
    let Some(draft) = panel.timing_drafts.get_mut(key) else {
        return;
    };
    match action {
        TimingAction::Add => create_row(draft, key, "", "", window, cx),
        TimingAction::Remove(id) => draft.rows.retain(|(time, _)| time.entity_id() != id),
        TimingAction::Mode(mode) => draft.mode = mode,
        TimingAction::Reload | TimingAction::Keep => {
            if draft.pending.is_some() {
                return;
            }
            let Ok(snapshot) = item.read(cx).source().snapshot() else {
                return;
            };
            let Some(view) = snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == key.1)
                .and_then(|diagram| diagram.all_views().find(|view| view.id == key.2))
            else {
                return;
            };
            let canonical = view.timing.clone().unwrap_or(TimingPresentation {
                mode: TimingMode::State,
                points: Vec::new(),
            });
            draft
                .state
                .reload(&serialize_timing(&canonical), snapshot.revision);
            if matches!(action, TimingAction::Reload) {
                draft.rows.clear();
                draft.listeners.clear();
                draft.admitted_inputs.clear();
                draft.mode = canonical.mode;
                for point in canonical.points {
                    create_row(
                        draft,
                        key,
                        &point.time.to_string(),
                        &point.value,
                        window,
                        cx,
                    );
                }
            } else {
                for (time, value) in &draft.rows {
                    for input in [time, value] {
                        draft
                            .admitted_inputs
                            .insert(input.entity_id(), input.read(cx).text(cx));
                    }
                }
            }
        }
    }
    draft.admit_snapshot();
    flush_timing_edits(panel, item, key, cx);
    cx.notify();
}
fn render_action_button(
    id: String,
    icon: IconName,
    tooltip: &'static str,
    action: TimingAction,
    key: &TimingKey,
    item: &Entity<MdeWorkbenchItem>,
    cx: &Context<MdeRightPanel>,
) -> Button {
    let owner = cx.weak_entity();
    let item = item.clone();
    let key = key.clone();
    let icon = match action {
        TimingAction::Mode(TimingMode::State) => {
            Icon::from_path("icons/bellatrix-timing-state.svg")
        }
        TimingAction::Mode(TimingMode::Value) => {
            Icon::from_path("icons/bellatrix-timing-value.svg")
        }
        _ => Icon::new(icon),
    };
    Button::new(id, "")
        .start_icon(icon.size(IconSize::Small))
        .tooltip(Tooltip::text(tooltip))
        .on_click(move |_, window, cx| {
            if let Err(error) = owner.update(cx, |panel, cx| {
                edit_timing(panel, &item, &key, action, window, cx)
            }) {
                eprintln!("MDE timing edit failed: {error}");
            }
        })
}
pub(crate) fn render_timing_editor(
    draft: &TimingDraft,
    key: &TimingKey,
    item: &Entity<MdeWorkbenchItem>,
    cx: &Context<MdeRightPanel>,
) -> AnyElement {
    let header = div()
        .flex()
        .gap_1()
        .child(Label::new("Timing"))
        .child(
            render_action_button(
                "timing-state".into(),
                IconName::LineHeight,
                "State / condition trace",
                TimingAction::Mode(TimingMode::State),
                key,
                item,
                cx,
            )
            .toggle_state(draft.mode == TimingMode::State),
        )
        .child(
            render_action_button(
                "timing-value".into(),
                IconName::CaseSensitive,
                "General value ribbon",
                TimingAction::Mode(TimingMode::Value),
                key,
                item,
                cx,
            )
            .toggle_state(draft.mode == TimingMode::Value),
        )
        .child(render_action_button(
            "timing-add".into(),
            IconName::Plus,
            "Add an authored time and value",
            TimingAction::Add,
            key,
            item,
            cx,
        ));
    let rows = draft.rows.iter().enumerate().map(|(index, (time, value))| {
        div()
            .flex()
            .gap_1()
            .child(div().w(px(60.0)).child(time.clone()))
            .child(div().flex_1().child(value.clone()))
            .child(render_action_button(
                format!("timing-remove-{index}"),
                IconName::Trash,
                "Remove this sample",
                TimingAction::Remove(time.entity_id()),
                key,
                item,
                cx,
            ))
    });
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(header)
        .children(rows)
        .when(
            draft.state.error.is_some() || draft.state.conflict,
            |element| {
                element
                    .child(Label::new(draft.state.error.clone().unwrap_or_else(|| {
                        "Timing changed elsewhere; your samples are preserved".into()
                    })))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(render_action_button(
                                "timing-reload".into(),
                                IconName::RefreshTitle,
                                "Reload current timing; discard these local edits",
                                TimingAction::Reload,
                                key,
                                item,
                                cx,
                            ))
                            .child(render_action_button(
                                "timing-keep".into(),
                                IconName::ArrowUp,
                                "Explicitly replace current timing with these samples",
                                TimingAction::Keep,
                                key,
                                item,
                                cx,
                            )),
                    )
            },
        )
        .into_any_element()
}
