use crate::{Graph, StructuralSeverity, validate_structural_model};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::kernel_api_contract::{API_VERSION, decode_model, error_response, invalid_request};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidationRequest {
    api_version: u32,
    model: Value,
}

#[derive(Serialize)]
struct ValidationResponse {
    api_version: u32,
    coverage: &'static str,
    has_errors: bool,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize)]
struct Diagnostic {
    severity: &'static str,
    code: String,
    subject_id: String,
    message: String,
}

/// Validate a model through the versioned, function-only kernel boundary.
///
/// Input is a JSON object containing exactly `api_version: 1` and `model`, the
/// existing Bellatrix model document. Output contains `api_version`,
/// `coverage: "implemented_rules_only"`, `has_errors` and ordered `diagnostics`.
/// Each diagnostic contains `severity`, `code`, `subject_id` and `message`.
/// Diagnostics are ordered by code, subject identity, severity and message.
/// Array order inside the input model retains its existing semantic meaning.
///
/// # Validation scope
/// Graph integrity is checked first. A graph failure returns one error diagnostic
/// and skips semantic validation because ambiguous/missing identities invalidate
/// that stage's assumptions. Otherwise the existing structural-model validator
/// runs. No errors means no violations reported by these implemented checks,
/// not complete UML 2.5.1 conformance. Diagram-specific rules are not run here.
///
/// # Errors
/// Invalid JSON/envelopes, unsupported API/model versions and malformed model
/// documents return an error JSON string with `api_version` and an `error` object
/// containing `code` and `message`. A well-formed but invalid model is a successful
/// API call with `has_errors: true`, not a transport/contract error.
///
/// # Effects and determinism
/// No external side effects: no filesystem, clock, process, network, logging sink
/// or external callback is used. The input is not mutated; all indices and model
/// values are local to this invocation. For the same input and implementation,
/// the result is independent of preceding calls. No internal Rust model, index
/// or diagnostic type is part of this function's public signature.
pub fn validate_uml_model(input_json: &str) -> Result<String, String> {
    let request: ValidationRequest = serde_json::from_str(input_json)
        .map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    let mut diagnostics = match Graph::new(&model).validate() {
        Err(error) => vec![Diagnostic {
            severity: "error",
            code: "GRAPH_INTEGRITY".into(),
            subject_id: model.id.clone(),
            message: error.to_string(),
        }],
        Ok(()) => validate_structural_model(&model)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                severity: match diagnostic.severity {
                    StructuralSeverity::Error => "error",
                    StructuralSeverity::Warning => "warning",
                },
                code: diagnostic.code.into(),
                subject_id: diagnostic.subject_id,
                message: diagnostic.message,
            })
            .collect(),
    };
    diagnostics.sort_by(|left, right| {
        (&left.code, &left.subject_id, left.severity, &left.message)
            .cmp(&(&right.code, &right.subject_id, right.severity, &right.message))
    });
    let response = ValidationResponse {
        api_version: API_VERSION,
        coverage: "implemented_rules_only",
        has_errors: diagnostics.iter().any(|diagnostic| diagnostic.severity == "error"),
        diagnostics,
    };
    serde_json::to_string(&response)
        .map_err(|error| error_response("RESPONSE_SERIALIZATION_FAILED", error.to_string()))
}

/// Validate graph integrity and presentation without structural UML rule evaluation.
///
/// Input uses the same versioned `{api_version:1, model:...}` envelope as
/// `validate_uml_model`. Output has the same diagnostic shape, with coverage
/// `graph_integrity_and_presentation`. This deliberately preserves the complete
/// legacy Graph::validate behavior used by the CLI, including geometry/style,
/// rather than the topology-only precondition used by graph queries.
///
/// # Errors
/// Contract errors are versioned JSON errors. A decoded but invalid graph returns
/// Ok with `has_errors:true` and the first `GRAPH_INTEGRITY` diagnostic.
///
/// # Effects and determinism
/// No external effects, I/O, clock, sink or callbacks. No caller mutation or state
/// retention; the same input yields identical output independently of prior calls.
pub fn validate_graph_model(input_json: &str) -> Result<String, String> {
    let request: ValidationRequest = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    let diagnostics = match Graph::new(&model).validate() {
        Ok(()) => Vec::new(),
        Err(error) => vec![Diagnostic {
            severity: "error",
            code: "GRAPH_INTEGRITY".into(),
            subject_id: model.id.clone(),
            message: error.to_string(),
        }],
    };
    let response = ValidationResponse {
        api_version: API_VERSION,
        coverage: "graph_integrity_and_presentation",
        has_errors: !diagnostics.is_empty(),
        diagnostics,
    };
    serde_json::to_string(&response)
        .map_err(|_| error_response("RESPONSE_SERIALIZATION_FAILED", "validation response cannot be encoded as JSON"))
}
