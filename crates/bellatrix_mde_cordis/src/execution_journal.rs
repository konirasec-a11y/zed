//! Bounded MDE execution history shared by host consumers.
use crate::ModelingExecutionTrace;
use crate::{ExecutionEvent, TraceOutcome, TracePhase};
pub use bellatrix_mde_contracts::events::ExecutionJournalSnapshot;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tokio::sync::watch;

static NEXT_UI_SESSION: AtomicU64 = AtomicU64::new(1);

const CAPACITY: usize = 2_000;
const LOSS_NOTICE_CAPACITY: usize = 100;

#[derive(Default)]
pub(crate) struct ExecutionJournals {
    journals: Mutex<BTreeMap<bellatrix_project::ProjectId, Weak<ExecutionJournal>>>,
}

impl cordis::Service for ExecutionJournals {
    const NAME: &'static str = "bellatrix/mde-execution-journals/v1";
}

impl ExecutionJournals {
    pub(crate) fn journal(
        &self,
        project_id: &bellatrix_project::ProjectId,
    ) -> Arc<ExecutionJournal> {
        let mut journals = self
            .journals
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(journal) = journals.get(project_id).and_then(Weak::upgrade) {
            return journal;
        }
        journals.retain(|_, journal| journal.strong_count() > 0);
        let journal = Arc::new(ExecutionJournal::new(project_id.clone()));
        journals.insert(project_id.clone(), Arc::downgrade(&journal));
        journal
    }
}

pub(crate) fn register(context: &cordis::Context) -> Result<(), std::io::Error> {
    let journals = Arc::new(ExecutionJournals::default());
    let _service = context
        .provide(Arc::clone(&journals))
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let _listener = context
        .on::<crate::ModelingExecutionTraceEvent, _>(cordis::event::observer_sync(
            move |_, batch: ModelingExecutionTrace| {
                journals.journal(&batch.project_id).append(batch);
                Ok::<_, std::convert::Infallible>(())
            },
        ))
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(())
}

pub struct ExecutionJournal {
    project_id: bellatrix_project::ProjectId,
    state: Mutex<ExecutionJournalSnapshot>,
    changes: watch::Sender<u64>,
    interaction_run: String,
    interaction_started: Instant,
    interaction_sequence: AtomicU64,
}
impl ExecutionJournal {
    pub fn new(project_id: bellatrix_project::ProjectId) -> Self {
        let (changes, _) = watch::channel(0);
        Self {
            project_id,
            state: Mutex::new(ExecutionJournalSnapshot::default()),
            changes,
            interaction_run: format!("ui-{}", NEXT_UI_SESSION.fetch_add(1, Ordering::Relaxed)),
            interaction_started: Instant::now(),
            interaction_sequence: AtomicU64::new(0),
        }
    }
}
impl ExecutionJournal {
    pub fn append(&self, batch: ModelingExecutionTrace) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        // Every producer reaches this boundary once; the panel and stderr are consumers,
        // not independent histories. Keep publication in the same order as retention.
        crate::trace_output::write(&batch);
        if batch.dropped > 0 {
            state.producer_dropped_events =
                state.producer_dropped_events.saturating_add(batch.dropped);
            if state.producer_loss.len() == LOSS_NOTICE_CAPACITY {
                state.producer_loss.pop_front();
            }
            state.producer_loss.push_back((batch.run_id, batch.dropped));
        }
        for event in batch.events {
            if state.events.len() == CAPACITY {
                state.events.pop_front();
                state.evicted_events = state.evicted_events.saturating_add(1);
            }
            state.events.push_back(event);
        }
        state.version = state.version.saturating_add(1);
        self.changes.send_replace(state.version);
    }
    /// Only stable action names and model IDs belong here, never editor text or command payloads.
    pub fn record_interaction(
        &self,
        revision: u64,
        action: &'static str,
        identity: Option<&str>,
        outcome: TraceOutcome,
    ) {
        let event = ExecutionEvent {
            run_id: self.interaction_run.clone(),
            revision: Some(revision.to_string()),
            sequence: self.interaction_sequence.fetch_add(1, Ordering::Relaxed),
            elapsed_ns: self
                .interaction_started
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64,
            component: "ui".into(),
            action: action.into(),
            phase: TracePhase::Finish,
            identity: identity.map(str::to_owned),
            dependencies: Vec::new(),
            outcome: Some(outcome),
            message: None,
            context: Default::default(),
        };
        self.append(ModelingExecutionTrace {
            project_id: self.project_id.clone(),
            run_id: self.interaction_run.clone(),
            events: vec![event],
            dropped: 0,
        });
    }

    /// Retain host operation results through the same bounded trace formatter and journal.
    /// Result messages may contain model data; never pass document bytes or payloads here.
    pub fn record_result<T>(
        &self,
        project_id: bellatrix_project::ProjectId,
        revision: u64,
        action: &'static str,
        result: &Result<T, String>,
    ) {
        let run_id = format!(
            "{}-result-{}",
            self.interaction_run,
            self.interaction_sequence.fetch_add(1, Ordering::Relaxed)
        );
        let mut trace =
            bellatrix_mde::MdeExecutionTrace::new(run_id.clone(), Some(revision.to_string()), 2);
        let outcome = if let Err(error) = result {
            trace.diagnostic("host", action, None, error, TraceOutcome::Error);
            TraceOutcome::Error
        } else {
            TraceOutcome::Success
        };
        trace.emit(
            "host",
            action,
            if result.is_ok() {
                TracePhase::Finish
            } else {
                TracePhase::Error
            },
            None,
            &[],
            Some(outcome),
        );
        self.append(ModelingExecutionTrace {
            project_id,
            run_id,
            events: trace.events(),
            dropped: trace.dropped(),
        });
    }
    pub fn version(&self) -> u64 {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .version
    }
    pub fn snapshot(&self) -> ExecutionJournalSnapshot {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let version = state.version.saturating_add(1);
        *state = ExecutionJournalSnapshot {
            version,
            ..Default::default()
        };
        self.changes.send_replace(version);
    }
}
