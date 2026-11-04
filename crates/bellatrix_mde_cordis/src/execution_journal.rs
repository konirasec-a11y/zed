//! Bounded MDE execution history shared by host consumers.
pub use bellatrix_mde_contracts::events::{ExecutionJournalSnapshot};
use crate::ModelingExecutionTrace;
use crate::{ExecutionEvent, TraceOutcome, TracePhase};
use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tokio::sync::watch;

static NEXT_UI_SESSION: AtomicU64 = AtomicU64::new(1);

const CAPACITY: usize = 2_000;
const LOSS_NOTICE_CAPACITY: usize = 100;

pub struct ExecutionJournal {
    state: Mutex<ExecutionJournalSnapshot>,
    changes: watch::Sender<u64>,
    interaction_run: String,
    interaction_started: Instant,
    interaction_sequence: AtomicU64,
}
impl Default for ExecutionJournal {
    fn default() -> Self {
        let (changes, _) = watch::channel(0);
        Self {
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
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
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
        };
        if state.events.len() == CAPACITY {
            state.events.pop_front();
            state.evicted_events = state.evicted_events.saturating_add(1);
        }
        state.events.push_back(event);
        state.version = state.version.saturating_add(1);
        self.changes.send_replace(state.version);
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
