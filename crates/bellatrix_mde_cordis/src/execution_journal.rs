//! Bounded MDE execution history shared by host consumers.
use crate::ModelingExecutionTrace;
use bellatrix_mde::ExecutionEvent;
use std::{collections::VecDeque, sync::Mutex};
use tokio::sync::watch;

const CAPACITY: usize = 2_000;
const LOSS_NOTICE_CAPACITY: usize = 100;

#[derive(Clone, Default)]
pub struct ExecutionJournalSnapshot {
    pub version: u64,
    pub events: VecDeque<ExecutionEvent>,
    pub evicted_events: u64,
    pub producer_dropped_events: u64,
    pub producer_loss: VecDeque<(String, u64)>,
}

pub struct ExecutionJournal {
    state: Mutex<ExecutionJournalSnapshot>,
    changes: watch::Sender<u64>,
}
impl Default for ExecutionJournal {
    fn default() -> Self {
        let (changes, _) = watch::channel(0);
        Self {
            state: Mutex::new(ExecutionJournalSnapshot::default()),
            changes,
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
