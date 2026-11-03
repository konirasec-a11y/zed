//! Native, bounded view of execution events emitted by the MDE runtime.
use crate::workbench_ui::{MdeWorkbenchItem, WorkbenchPalette};
use bellatrix_mde_cordis::{ExecutionJournal, ExecutionJournalSnapshot};
use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, Pixels, Render, Subscription, Task, Window, actions, div, prelude::*, px,
    relative,
};
use std::{sync::Arc, time::Duration};
use ui::{Button, IconName, Label, prelude::*};
use ui_input::{ErasedEditorEvent, InputField};
use workspace::dock::{DockPosition, Panel, PanelEvent};

actions!(bellatrix_mde, [ToggleMdeExecutionDashboard]);

pub struct MdeExecutionDashboard {
    focus: FocusHandle,
    search: Entity<InputField>,
    journal: Option<Arc<ExecutionJournal>>,
    project_label: Option<String>,
    snapshot: ExecutionJournalSnapshot,
    paused: bool,
    errors_only: bool,
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
            search,
            journal: Some(journal),
            project_label: Some(item.read(cx).source().project_id.as_str().to_owned()),
            snapshot,
            paused: false,
            errors_only: false,
            _search_listener: search_listener,
            _receive_task: receive_task,
        }
    }
    pub(crate) fn bind(&mut self, item: Option<&Entity<MdeWorkbenchItem>>, cx: &mut Context<Self>) {
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
impl EventEmitter<PanelEvent> for MdeExecutionDashboard {}
impl Focusable for MdeExecutionDashboard {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Panel for MdeExecutionDashboard {
    fn persistent_name() -> &'static str {
        "MDE Live Execution"
    }
    fn panel_key() -> &'static str {
        "bellatrix.mde.execution.v1"
    }
    fn position(&self, _: &Window, _: &App) -> DockPosition {
        DockPosition::Bottom
    }
    fn position_is_valid(&self, position: DockPosition) -> bool {
        position == DockPosition::Bottom
    }
    fn set_position(&mut self, _: DockPosition, _: &mut Window, _: &mut Context<Self>) {}
    fn default_size(&self, _: &Window, _: &App) -> Pixels {
        px(300.0)
    }
    fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
        Some(IconName::ListTree)
    }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Bellatrix MDE - Live execution logs and sequence plot")
    }
    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleMdeExecutionDashboard)
    }
    fn starts_open(&self, _: &Window, _: &App) -> bool {
        false
    }
    fn activation_priority(&self) -> u32 {
        10_003
    }
}
impl Render for MdeExecutionDashboard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
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
                let line = format!(
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
                );
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
                    .child(div().flex_1().whitespace_normal().child(line))
            })
            .collect();
        let producer_loss = self
            .snapshot
            .producer_loss
            .iter()
            .map(|(run, count)| format!("Producer dropped {count} event(s) in {run}"))
            .collect::<Vec<_>>();
        div().id("mde-execution-dashboard").track_focus(&self.focus).size_full().flex().flex_col()
            .bg(palette.panel).text_color(palette.text)
            .child(div().h(px(36.0)).flex().items_center().gap_2()
                .child(Label::new("LIVE LOGS · ENGINE"))
                .child(Label::new(self.project_label.clone().unwrap_or_else(|| "No active MDE document".to_owned())))
                .child(Button::new("mde-execution-pause", if self.paused { "Resume" } else { "Pause" })
                    .on_click(move |_, _, cx| { if let Err(error) = pause_owner.update(cx, |panel, cx| { panel.paused = !panel.paused; cx.notify(); }) { eprintln!("MDE execution pause failed: {error}"); } }))
                .child(Button::new("mde-execution-filter", if self.errors_only { "Show all" } else { "Errors only" })
                    .on_click(move |_, _, cx| { if let Err(error) = filter_owner.update(cx, |panel, cx| { panel.errors_only = !panel.errors_only; cx.notify(); }) { eprintln!("MDE execution filter failed: {error}"); } }))
                .child(Button::new("mde-execution-clear", "Clear")
                    .on_click(move |_, _, cx| { if let Err(error) = clear_owner.update(cx, |panel, cx| { if let Some(journal) = &panel.journal { journal.clear(); panel.snapshot = journal.snapshot(); } cx.notify(); }) { eprintln!("MDE execution clear failed: {error}"); } })))
            .child(div().px_2().child(self.search.clone()))
            .child(div().px_2().child(format!("Received order; sequence and elapsed-time plots (latest 300 matches, relative scale per run) · {} retained · {} evicted · {} producer events dropped", self.snapshot.events.len(), self.snapshot.evicted_events, self.snapshot.producer_dropped_events)))
            .child(div().px_2().child("The in-memory MDE journal retains events while this panel is closed and resets when the app exits. Mutation boundaries, Rust generation and diagram validation are instrumented; HTML export has task boundaries only. Internal helper calls are not all traced."))
            .children(producer_loss.into_iter().map(|line| div().px_2().child(line)))
            .child(div().id("mde-execution-rows").flex_1().min_h_0().overflow_y_scroll().p_2()
                .when(rows.is_empty(), |view| view.child("No matching execution events. Select an MDE document and run an instrumented operation."))
                .children(rows))
    }
}
