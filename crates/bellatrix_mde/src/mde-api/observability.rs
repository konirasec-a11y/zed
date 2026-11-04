//! Opaque execution observation handle. Domain trace types remain private.
use bellatrix_mde_protocol::trace::{ExecutionEvent, TracePhase, TraceOutcome, ModelingDiagnostic, DiagnosticSeverity};

#[derive(Clone)]
pub struct MdeExecutionTrace(crate::ExecutionTrace);
impl MdeExecutionTrace {
    pub fn new(run_id: impl Into<String>, revision: Option<String>, capacity: usize) -> Self {
        Self(crate::ExecutionTrace::new(run_id, revision, capacity))
    }
    /// Sinks must not block; hosts should forward to bounded channels.
    pub fn set_sink(&mut self, sink: impl Fn(&ExecutionEvent) + Send + Sync + 'static) {
        self.0.set_sink(move |event| sink(&event_contract(event)));
    }
    pub fn clear_sink(&mut self) { self.0.clear_sink(); }
    pub fn run_id(&self) -> &str { self.0.run_id() }
    pub fn events(&self) -> Vec<ExecutionEvent> { self.0.events().iter().map(event_contract).collect() }
    pub fn dropped(&self) -> u64 { self.0.dropped() }
    pub fn emit(&mut self, component: &str, action: &str, phase: TracePhase,
        identity: Option<&str>, dependencies: &[String], outcome: Option<TraceOutcome>) {
        self.0.emit(component, action, match phase {
            TracePhase::Start => crate::TracePhase::Start,
            TracePhase::Finish => crate::TracePhase::Finish,
            TracePhase::Error => crate::TracePhase::Error,
            TracePhase::Diagnostic => crate::TracePhase::Diagnostic,
        }, identity, dependencies, outcome.map(|outcome| match outcome {
            TraceOutcome::Success => crate::TraceOutcome::Success,
            TraceOutcome::Invalid => crate::TraceOutcome::Invalid,
            TraceOutcome::Unsupported => crate::TraceOutcome::Unsupported,
            TraceOutcome::Error => crate::TraceOutcome::Error,
        }));
    }
}

fn event_contract(event: &crate::ExecutionEvent) -> ExecutionEvent {
    ExecutionEvent {
        run_id: event.run_id.clone(), revision: event.revision.clone(), sequence: event.sequence,
        elapsed_ns: event.elapsed_ns, component: event.component.clone(), action: event.action.clone(),
        phase: match event.phase {
            crate::TracePhase::Start => TracePhase::Start,
            crate::TracePhase::Finish => TracePhase::Finish,
            crate::TracePhase::Error => TracePhase::Error,
            crate::TracePhase::Diagnostic => TracePhase::Diagnostic,
        },
        identity: event.identity.clone(), dependencies: event.dependencies.clone(),
        outcome: event.outcome.map(|outcome| match outcome {
            crate::TraceOutcome::Success => TraceOutcome::Success,
            crate::TraceOutcome::Invalid => TraceOutcome::Invalid,
            crate::TraceOutcome::Unsupported => TraceOutcome::Unsupported,
            crate::TraceOutcome::Error => TraceOutcome::Error,
        }),
    }
}

/// Validate the selected stored diagram kind. Results are diagnostics, not a
/// certificate of complete UML conformance; unverified rules retain warnings.
pub fn validate_model_section_diagram(project_id: &str, name: &str, section_json: &str,
    diagram_id: &str, trace: Option<&mut MdeExecutionTrace>) -> Result<Vec<ModelingDiagnostic>, String> {
    let section = crate::mde_api_projection::decode_section(project_id, name, section_json)?;
    let model = section.to_model(project_id, name);
    let diagram = model.diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let kind = crate::StructuralDiagramKind::from_diagram(diagram)
        .ok_or_else(|| format!("unsupported UMLDI diagram type: {}", diagram.uml_di_type))?;
    let diagnostics = match trace {
        Some(trace) => crate::validate_structural_diagram_traced(&model, diagram_id, kind, &mut trace.0),
        None => crate::validate_structural_diagram(&model, diagram_id, kind),
    };
    Ok(diagnostics.into_iter().map(|diagnostic| ModelingDiagnostic {
        severity: match diagnostic.severity {
            crate::StructuralSeverity::Error => DiagnosticSeverity::Error,
            crate::StructuralSeverity::Warning => DiagnosticSeverity::Warning,
        },
        code: diagnostic.code.into(), subject_id: diagnostic.subject_id, message: diagnostic.message,
    }).collect())
}
