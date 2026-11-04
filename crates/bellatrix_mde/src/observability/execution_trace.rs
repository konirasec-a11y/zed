//! Opt-in execution events. A caller owns each bounded run and chooses how to export it.
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TracePhase {
    Start,
    Finish,
    Error,
    Diagnostic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceOutcome {
    Success,
    Invalid,
    Unsupported,
    Error,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExecutionEvent {
    pub run_id: String,
    pub revision: Option<String>,
    pub sequence: u64,
    /// Monotonic nanoseconds since this run was created.
    pub elapsed_ns: u64,
    pub component: String,
    pub action: String,
    pub phase: TracePhase,
    pub identity: Option<String>,
    pub dependencies: Vec<String>,
    pub outcome: Option<TraceOutcome>,
}

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

use crate::{
    Project, StructuralDiagnostic, StructuralDiagramKind, StructuralSeverity,
    validate_structural_diagram, validate_structural_model,
};

/// Records the validation call and emitted diagnostic codes, not unobserved rule checks.
pub fn validate_structural_model_traced(
    project: &Project,
    trace: &mut crate::ExecutionTrace,
) -> Vec<StructuralDiagnostic> {
    use crate::{TraceOutcome, TracePhase};
    trace.emit(
        "kernel",
        "structural.validate",
        TracePhase::Start,
        Some(&project.id),
        &[],
        None,
    );
    let issues = validate_structural_model(project);
    for issue in &issues {
        let outcome = if issue.code.contains("UNSUPPORTED") {
            TraceOutcome::Unsupported
        } else if issue.severity == StructuralSeverity::Error {
            TraceOutcome::Invalid
        } else {
            TraceOutcome::Success
        };
        trace.emit(
            "kernel",
            issue.code,
            TracePhase::Diagnostic,
            Some(&issue.subject_id),
            &[],
            Some(outcome),
        );
    }
    let outcome = if issues.iter().any(|issue| {
        issue.severity == StructuralSeverity::Error && !issue.code.contains("UNSUPPORTED")
    }) {
        TraceOutcome::Invalid
    } else if issues
        .iter()
        .any(|issue| issue.code.contains("UNSUPPORTED"))
    {
        TraceOutcome::Unsupported
    } else {
        TraceOutcome::Success
    };
    trace.emit(
        "kernel",
        "structural.validate",
        TracePhase::Finish,
        Some(&project.id),
        &[],
        Some(outcome),
    );
    issues
}

/// Records the projection validation boundary and resulting diagnostic identities.
pub fn validate_structural_diagram_traced(
    project: &Project,
    diagram_id: &str,
    kind: StructuralDiagramKind,
    trace: &mut crate::ExecutionTrace,
) -> Vec<StructuralDiagnostic> {
    use crate::{TraceOutcome, TracePhase};
    trace.emit(
        "kernel",
        "diagram.validate",
        TracePhase::Start,
        Some(diagram_id),
        &[],
        None,
    );
    let issues = validate_structural_diagram(project, diagram_id, kind);
    for issue in &issues {
        let outcome = if issue.code.contains("UNSUPPORTED") {
            TraceOutcome::Unsupported
        } else if issue.severity == StructuralSeverity::Error {
            TraceOutcome::Invalid
        } else {
            TraceOutcome::Success
        };
        trace.emit(
            "kernel",
            issue.code,
            TracePhase::Diagnostic,
            Some(&issue.subject_id),
            &[],
            Some(outcome),
        );
    }
    let outcome = if issues.iter().any(|issue| {
        issue.severity == StructuralSeverity::Error && !issue.code.contains("UNSUPPORTED")
    }) {
        TraceOutcome::Invalid
    } else if issues
        .iter()
        .any(|issue| issue.code.contains("UNSUPPORTED"))
    {
        TraceOutcome::Unsupported
    } else {
        TraceOutcome::Success
    };
    trace.emit(
        "kernel",
        "diagram.validate",
        TracePhase::Finish,
        Some(diagram_id),
        &[],
        Some(outcome),
    );
    issues
}
