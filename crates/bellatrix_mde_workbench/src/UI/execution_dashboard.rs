//! Native, bounded view of execution events emitted by the MDE runtime.
use crate::workbench_ui::{MdeWorkbenchItem, WorkbenchPalette};
use bellatrix_mde_cordis::{ExecutionJournal, ExecutionJournalSnapshot};
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, Render,
    Subscription, Task, WeakEntity, Window, actions, div, prelude::*, px, relative,
};
use std::{sync::Arc, time::Duration};
use ui::{CopyButton, IconButton, IconName, Label, prelude::*};
use ui_input::{ErasedEditorEvent, InputField};

actions!(bellatrix_mde, [ToggleMdeExecutionDashboard]);

pub struct MdeExecutionDashboard {
    focus: FocusHandle,
    item: Option<WeakEntity<MdeWorkbenchItem>>,
    search: Entity<InputField>,
    journal: Option<Arc<ExecutionJournal>>,
    project_label: Option<String>,
    snapshot: ExecutionJournalSnapshot,
    paused: bool,
    errors_only: bool,
    details_open: bool,
    _item_listener: Option<Subscription>,
    _search_listener: Subscription,
    _receive_task: Task<()>,
}

impl MdeExecutionDashboard {
    pub fn new(
        item: Entity<MdeWorkbenchItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| {
            InputField::new(window, cx, "Filter run, function, identity or dependency…")
                .start_icon(IconName::MagnifyingGlass)
                .label_min_width(px(0.0))
        });
        let owner = cx.weak_entity();
        let editor = search.read(cx).editor().clone();
        let search_listener = editor.subscribe(
            Box::new(move |event, _, cx| {
                if event == ErasedEditorEvent::BufferEdited {
                    if let Err(error) = owner.update(cx, |_, cx| cx.notify()) {
                        eprintln!("MDE execution search failed: {error}");
                    }
                }
            }),
            window,
            cx,
        );
        let journal = Arc::clone(&item.read(cx).source().journal);
        let snapshot = journal.snapshot();
        let executor = cx.background_executor().clone();
        let receive_task = cx.spawn(async move |this, cx| {
            loop {
                executor.timer(Duration::from_millis(100)).await;
                if this
                    .update(cx, |dashboard, cx| dashboard.refresh_snapshot(cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            focus: cx.focus_handle(),
            item: Some(item.downgrade()),
            search,
            journal: Some(journal),
            project_label: Some(item.read(cx).source().project_id.as_str().to_owned()),
            snapshot,
            paused: false,
            errors_only: false,
            details_open: false,
            _item_listener: Some(cx.observe(&item, |_, _, cx| cx.notify())),
            _search_listener: search_listener,
            _receive_task: receive_task,
        }
    }
    pub(crate) fn bind(&mut self, item: Option<&Entity<MdeWorkbenchItem>>, cx: &mut Context<Self>) {
        let previous_item = self.item.as_ref().map(|item| item.entity_id());
        let next_item = item.map(Entity::entity_id);
        if previous_item != next_item {
            self.item = item.map(Entity::downgrade);
            self._item_listener = item.map(|item| cx.observe(item, |_, _, cx| cx.notify()));
            cx.notify();
        }
        let project_label = item.map(|item| item.read(cx).source().project_id.as_str().to_owned());
        let journal = item.map(|item| Arc::clone(&item.read(cx).source().journal));
        let same = match (&self.journal, &journal) {
            (Some(current), Some(next)) => Arc::ptr_eq(current, next),
            (None, None) => true,
            _ => false,
        };
        if !same || self.project_label != project_label {
            self.project_label = project_label;
            self.snapshot = journal
                .as_ref()
                .map(|journal| journal.snapshot())
                .unwrap_or_default();
            self.journal = journal;
            cx.notify();
        }
    }
    fn event_line(event: &bellatrix_mde::ExecutionEvent) -> String {
        format!(
            "{}  #{}  t+{}ns  rev={}  {}::{}  {:?}  id={}  recorded deps=[{}]  outcome={:?}",
            event.run_id,
            event.sequence,
            event.elapsed_ns,
            event.revision.as_deref().unwrap_or("unknown"),
            event.component,
            event.action,
            event.phase,
            event.identity.as_deref().unwrap_or("unknown"),
            event.dependencies.join(", "),
            event.outcome
        )
    }
    fn refresh_snapshot(&mut self, cx: &mut Context<Self>) {
        if self.paused {
            return;
        }
        let Some(journal) = &self.journal else {
            return;
        };
        if journal.version() != self.snapshot.version {
            self.snapshot = journal.snapshot();
            cx.notify();
        }
    }
}
impl Focusable for MdeExecutionDashboard {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for MdeExecutionDashboard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let item = self.item.as_ref().and_then(WeakEntity::upgrade);
        let validation_status = item.as_ref().map(|item| item.read(cx).validation_status().to_owned());
        let feedback = item.as_ref().map(|item| item.read(cx).feedback().to_owned());
        let query = self.search.read(cx).text(cx).trim().to_lowercase();
        let matching: Vec<_> = self
            .snapshot
            .events
            .iter()
            .filter(|event| {
                let text = format!(
                    "{} {} {} {} {:?} {:?} {}",
                    event.run_id,
                    event.component,
                    event.action,
                    event.identity.as_deref().unwrap_or(""),
                    event.phase,
                    event.outcome,
                    event.dependencies.join(" ")
                )
                .to_lowercase();
                text.contains(&query)
                    && (!self.errors_only
                        || matches!(event.phase, bellatrix_mde::TracePhase::Error)
                        || matches!(
                            event.outcome,
                            Some(
                                bellatrix_mde::TraceOutcome::Error
                                    | bellatrix_mde::TraceOutcome::Invalid
                                    | bellatrix_mde::TraceOutcome::Unsupported
                            )
                        ))
            })
            .collect();
        let mut run_maxima = std::collections::HashMap::<&str, u64>::new();
        let mut run_elapsed_maxima = std::collections::HashMap::<&str, u64>::new();
        for event in &matching {
            let maximum = run_maxima.entry(event.run_id.as_str()).or_default();
            *maximum = (*maximum).max(event.sequence);
            let elapsed = run_elapsed_maxima.entry(event.run_id.as_str()).or_default();
            *elapsed = (*elapsed).max(event.elapsed_ns);
        }
        let owner = cx.weak_entity();
        let pause_owner = owner.clone();
        let filter_owner = owner.clone();
        let clear_owner = owner;
        let rows: Vec<_> = matching
            .iter()
            .skip(matching.len().saturating_sub(300))
            .map(|event| {
                let max_sequence = run_maxima.get(event.run_id.as_str()).copied().unwrap_or(0);
                let fraction = ((event.sequence as f64 + 1.0) / (max_sequence as f64 + 1.0)) as f32;
                let max_elapsed = run_elapsed_maxima.get(event.run_id.as_str()).copied().unwrap_or(0).max(1);
                let time_fraction = (event.elapsed_ns as f64 / max_elapsed as f64) as f32;
                let line = Self::event_line(event);
                div()
                    .w_full()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w(px(100.0))
                            .h(px(6.0))
                            .bg(palette.surface)
                            .child(div().h_full().w(relative(fraction)).bg(palette.accent)),
                    )
                    .child(div().w(px(100.0)).h(px(6.0)).bg(palette.surface)
                        .child(div().h_full().w(relative(time_fraction)).bg(palette.selection)))
                    .child(div().flex_1().min_w_0().whitespace_normal().child(line.clone()))
                    .child(CopyButton::new(
                        SharedString::from(format!("mde-execution-copy-{}-{}", event.run_id, event.sequence)),
                        line,
                    ).tooltip_label("Copy log line"))
            })
            .collect();
        let producer_loss = self
            .snapshot
            .producer_loss
            .iter()
            .map(|(run, count)| format!("Producer dropped {count} event(s) in {run}"))
            .collect::<Vec<_>>();
        let copy_disabled = self.snapshot.events.is_empty()
            && producer_loss.is_empty()
            && validation_status.is_none()
            && feedback.as_deref().is_none_or(str::is_empty);
        let copied_lines = producer_loss.iter().cloned()
            .chain(self.snapshot.events.iter().map(Self::event_line))
            .collect::<Vec<_>>()
            .join("\n");
        let copied_logs = format!(
            "MDE logs and diagnostics\nproject_id={}\npaused={} (events only; validation and latest feedback are current)\nretained={} evicted={} dropped={}\ncopied_events={} (all retained events in this snapshot, unfiltered, received order)\n\nValidation\n{}\n\nLatest operation\n{}\n\nExecution events\n{}",
            self.project_label.as_deref().unwrap_or("none"),
            self.paused,
            self.snapshot.events.len(),
            self.snapshot.evicted_events,
            self.snapshot.producer_dropped_events,
            self.snapshot.events.len(),
            validation_status.as_deref().unwrap_or("No active MDE document"),
            feedback.as_deref().filter(|text| !text.is_empty()).unwrap_or("No operation feedback"),
            copied_lines,
        );
        div().id("mde-execution-dashboard").track_focus(&self.focus).size_full().flex().flex_col()
            .bg(palette.panel).text_color(palette.text)
            .child(div().h(px(36.0)).flex_none().px_2().flex().items_center().gap_2().border_b_1().border_color(palette.border)
                .child(Label::new("Logs & diagnostics"))
                .child(div().flex_1().min_w_0().overflow_hidden().text_xs().text_color(palette.muted).child(self.project_label.clone().unwrap_or_else(|| "No active MDE document".to_owned())))
                .child(CopyButton::new("mde-execution-copy", copied_logs)
                    .tooltip_label("Copy diagnostics, latest feedback and all logs in this snapshot (ignores filters)")
                    .disabled(copy_disabled))
                .child(IconButton::new("mde-execution-pause", if self.paused { IconName::PlayFilled } else { IconName::DebugPause })
                    .aria_label(if self.paused { "Resume logs" } else { "Pause logs" })
                    .tooltip(ui::Tooltip::text(if self.paused { "Resume logs" } else { "Pause logs" }))
                    .toggle_state(self.paused)
                    .on_click(move |_, _, cx| { if let Err(error) = pause_owner.update(cx, |panel, cx| { panel.paused = !panel.paused; cx.notify(); }) { eprintln!("MDE execution pause failed: {error}"); } }))
                .child(IconButton::new("mde-execution-filter", IconName::Filter)
                    .aria_label(if self.errors_only { "Show all events" } else { "Show errors only" })
                    .tooltip(ui::Tooltip::text(if self.errors_only { "Show all events" } else { "Show errors only" }))
                    .toggle_state(self.errors_only)
                    .on_click(move |_, _, cx| { if let Err(error) = filter_owner.update(cx, |panel, cx| { panel.errors_only = !panel.errors_only; cx.notify(); }) { eprintln!("MDE execution filter failed: {error}"); } }))
                .child(IconButton::new("mde-execution-clear", IconName::Trash)
                    .aria_label("Clear execution events")
                    .tooltip(ui::Tooltip::text("Clear execution events; keep validation and latest feedback"))
                    .on_click(move |_, _, cx| { if let Err(error) = clear_owner.update(cx, |panel, cx| { if let Some(journal) = &panel.journal { journal.clear(); panel.snapshot = journal.snapshot(); } cx.notify(); }) { eprintln!("MDE execution clear failed: {error}"); } })))
            .child(div().flex_none().px_2().py_1().child(self.search.clone()))
            .child(div().flex_none().px_2().flex().items_center().gap_2()
                .child(div().flex_1().text_xs().text_color(palette.muted).child(format!("{} shown / {} matching · {} retained · {} evicted · {} dropped", rows.len(), matching.len(), self.snapshot.events.len(), self.snapshot.evicted_events, self.snapshot.producer_dropped_events)))
                .child(IconButton::new("mde-execution-details", IconName::Info)
                    .aria_label("Trace details")
                    .aria_expanded(self.details_open)
                    .tooltip(ui::Tooltip::text("Trace details"))
                    .toggle_state(self.details_open)
                    .on_click(cx.listener(|panel, _, _, cx| { panel.details_open = !panel.details_open; cx.notify(); }))))
            .child(div().id("mde-execution-rows").flex_1().min_h_0().overflow_y_scroll().p_2()
                .when_some(validation_status, |view, status| view.child(
                    div().pb_2().whitespace_normal()
                        .child(Label::new("Validation"))
                        .child(div().text_xs().text_color(palette.muted).child("Read-only implemented UML checks using the stored diagram type. This is not full conformance certification."))
                        .children(status.lines().map(|line| div().child(line.to_owned())).collect::<Vec<_>>())))
                .when_some(feedback.filter(|text| !text.is_empty()), |view, feedback| view.child(
                    div().pb_2().whitespace_normal()
                        .child(Label::new("Latest operation"))
                        .children(feedback.lines().map(|line| div().child(line.to_owned())).collect::<Vec<_>>())))
                .children(producer_loss.into_iter().map(|line| div().pb_1().child(line)))
                .when(self.details_open, |view| view.child(div().pb_2().text_xs().text_color(palette.muted).whitespace_normal()
                    .child("Received order; sequence and elapsed-time plots show the latest 300 matches, with relative scale per run. Copy includes all retained events in the current snapshot, regardless of the search and error filters. Pause freezes only the event snapshot; recording continues and validation and latest feedback remain current. UI events record clicks, selections and command enqueue results, not typed text or pointer motion. Successful enqueue does not mean the command was applied. The in-memory journal retains up to 2,000 events while this panel is closed and resets when the app exits. Mutation and diagram validation boundaries are observed outside the kernel. Rust generation and HTML export have task boundaries only. Kernel calculations have no internal tracing.")))
                .when(rows.is_empty(), |view| view.child(div().text_xs().text_color(palette.muted).child(if self.snapshot.events.is_empty() {
                    "No execution events yet. Run an instrumented MDE operation to see its trace."
                } else {
                    "No matching events. Adjust the search or turn off Errors only."
                })))
                .children(rows))
    }
}
