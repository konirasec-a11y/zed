//! Opt-in execution events. A caller owns each bounded run and chooses how to export it.
pub use bellatrix_mde_contracts::protocol::trace::{ExecutionEvent, TraceOutcome, TracePhase};

type EventSink = std::sync::Arc<dyn Fn(&ExecutionEvent) + Send + Sync>;

/// The caller supplies a unique run ID and an optional model revision. Events are
/// kept in execution order; dropped events are counted when the capacity is reached.
#[derive(Clone)]
pub struct ExecutionTrace {
    run_id: String,
    revision: Option<String>,
    capacity: usize,
    next_sequence: u64,
    started: std::time::Instant,
    dropped: u64,
    events: Vec<ExecutionEvent>,
    sink: Option<EventSink>,
}

impl ExecutionTrace {
    pub fn new(run_id: impl Into<String>, revision: Option<String>, capacity: usize) -> Self {
        Self {
            run_id: run_id.into(),
            revision,
            capacity,
            next_sequence: 0,
            started: std::time::Instant::now(),
            dropped: 0,
            events: Vec::new(),
            sink: None,
        }
    }
    /// The sink must not block; hosts can enqueue events to a bounded channel.
    pub fn set_sink(&mut self, sink: impl Fn(&ExecutionEvent) + Send + Sync + 'static) {
        self.sink = Some(std::sync::Arc::new(sink));
    }
    pub fn clear_sink(&mut self) {
        self.sink = None;
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn events(&self) -> &[ExecutionEvent] {
        &self.events
    }
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
    pub fn emit(
        &mut self,
        component: &str,
        action: &str,
        phase: TracePhase,
        identity: Option<&str>,
        dependencies: &[String],
        outcome: Option<TraceOutcome>,
    ) {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        let event = ExecutionEvent {
            run_id: self.run_id.clone(),
            revision: self.revision.clone(),
            sequence,
            elapsed_ns: self.started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
            component: component.into(),
            action: action.into(),
            phase,
            identity: identity.map(str::to_owned),
            dependencies: dependencies.to_vec(),
            outcome,
        };
        if let Some(sink) = &self.sink {
            sink(&event);
        }
        if self.events.len() >= self.capacity {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        self.events.push(event);
    }
}

/// Output adapter: returns newline-delimited JSON without performing filesystem I/O.
pub fn execution_trace_jsonl(trace: &ExecutionTrace) -> Result<String, serde_json::Error> {
    let mut output = String::new();
    for event in trace.events() {
        output.push_str(&serde_json::to_string(event)?);
        output.push('\n');
    }
    Ok(output)
}
