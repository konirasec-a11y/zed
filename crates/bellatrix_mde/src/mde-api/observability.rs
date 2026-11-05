//! Opaque execution observation handle over shared trace contracts.
use bellatrix_mde_contracts::protocol::trace::{
    DiagnosticSeverity, ExecutionEvent, ModelingDiagnostic, TraceOutcome, TracePhase,
};

#[derive(Clone)]
pub struct MdeExecutionTrace(crate::ExecutionTrace);
impl MdeExecutionTrace {
    pub fn new(run_id: impl Into<String>, revision: Option<String>, capacity: usize) -> Self {
        Self(crate::ExecutionTrace::new(run_id, revision, capacity))
    }
    /// Sinks must not block; hosts should forward to bounded channels.
    pub fn set_sink(&mut self, sink: impl Fn(&ExecutionEvent) + Send + Sync + 'static) {
        self.0.set_sink(sink);
    }
    pub fn clear_sink(&mut self) {
        self.0.clear_sink();
    }
    pub fn run_id(&self) -> &str {
        self.0.run_id()
    }
    pub fn events(&self) -> Vec<ExecutionEvent> {
        self.0.events().to_vec()
    }
    pub fn dropped(&self) -> u64 {
        self.0.dropped()
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
        self.0
            .emit(component, action, phase, identity, dependencies, outcome);
    }
}

/// Check contract and presentation integrity for an existing diagram.
/// This operation does not evaluate UML conformance.
pub fn validate_model_section_diagram(
    project_id: &str,
    name: &str,
    section_json: &str,
    diagram_id: &str,
    mut trace: Option<&mut MdeExecutionTrace>,
) -> Result<Vec<ModelingDiagnostic>, String> {
    if let Some(trace) = trace.as_deref_mut() {
        trace.emit(
            "modeling",
            "diagram.checkPresentation",
            TracePhase::Start,
            Some(diagram_id),
            &[],
            None,
        );
    }
    let result = (|| {
        let section = crate::mde_api_projection::decode_section(project_id, name, section_json)?;
        let model = section.to_model(project_id, name);
        if !model
            .diagrams
            .iter()
            .any(|diagram| diagram.id == diagram_id)
        {
            return Err("diagram not found".to_owned());
        }
        Ok(match crate::Graph::new(&model).validate_presentation() {
            Ok(()) => Vec::new(),
            Err(error) => vec![ModelingDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: "PRESENTATION_INTEGRITY".into(),
                subject_id: diagram_id.into(),
                message: error.to_string(),
            }],
        })
    })();
    if let Some(trace) = trace {
        match &result {
            Ok(diagnostics) => {
                for diagnostic in diagnostics {
                    trace.emit(
                        "modeling",
                        &diagnostic.code,
                        TracePhase::Diagnostic,
                        Some(&diagnostic.subject_id),
                        &[],
                        Some(TraceOutcome::Invalid),
                    );
                }
                trace.emit(
                    "modeling",
                    "diagram.checkPresentation",
                    TracePhase::Finish,
                    Some(diagram_id),
                    &[],
                    Some(if diagnostics.is_empty() {
                        TraceOutcome::Success
                    } else {
                        TraceOutcome::Invalid
                    }),
                );
            }
            Err(_) => trace.emit(
                "modeling",
                "diagram.checkPresentation",
                TracePhase::Error,
                Some(diagram_id),
                &[],
                Some(TraceOutcome::Error),
            ),
        }
    }
    result
}
