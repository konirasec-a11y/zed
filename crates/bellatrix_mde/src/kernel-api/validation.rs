use crate::{Format, Graph, StructuralSeverity, decode, validate_structural_model};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const API_VERSION: u32 = 1;

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

fn error_response(code: &str, message: impl Into<String>) -> String {
    serde_json::json!({
        "api_version": API_VERSION,
        "error": { "code": code, "message": message.into() },
    })
    .to_string()
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
        .map_err(|error| error_response("INVALID_REQUEST", error.to_string()))?;
    if request.api_version != API_VERSION {
        return Err(error_response(
            "UNSUPPORTED_API_VERSION",
            format!("expected api_version {API_VERSION}, received {}", request.api_version),
        ));
    }
    let model_json = serde_json::to_vec(&request.model)
        .map_err(|error| error_response("INVALID_MODEL", error.to_string()))?;
    let model = decode(&model_json, Format::Json)
        .map_err(|error| error_response("INVALID_MODEL", error.to_string()))?;
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
