//! Transport events emitted by Cordis; engine event types never cross this API.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TracePhase { Start, Finish, Error, Diagnostic }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceOutcome { Success, Invalid, Unsupported, Error }

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity { Error, Warning }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelingDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub subject_id: String,
    pub message: String,
}
