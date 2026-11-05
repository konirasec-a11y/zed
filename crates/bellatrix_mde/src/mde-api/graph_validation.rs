use crate::Graph;
use serde::{Deserialize, Serialize};

use crate::api_contract::{API_VERSION, decode_model, error_response, invalid_request};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidationRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
}

#[derive(Serialize)]
pub(super) struct ValidationResponse {
    api_version: u32,
    coverage: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    validity: Option<&'static str>,
    pub(super) has_errors: bool,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize)]
struct Diagnostic {
    severity: &'static str,
    code: String,
    subject_id: String,
    message: String,
}

/// Validate graph integrity and presentation without structural UML rule evaluation.
///
/// Input uses a versioned `{api_version:1, model:...}` envelope.
/// Output reports graph diagnostics with coverage
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
        validity: None,
        has_errors: !diagnostics.is_empty(),
        diagnostics,
    };
    serde_json::to_string(&response).map_err(|_| {
        error_response(
            "RESPONSE_SERIALIZATION_FAILED",
            "validation response cannot be encoded as JSON",
        )
    })
}
