use crate::Scope;
use crate::api_contract::{API_VERSION, decode_model, error_response, invalid_request};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    element_ids: Option<Vec<String>>,
    skeleton: bool,
}

/// Resolve existing Rust emission tasks for the Cordis generation handler.
///
/// Input is `{api_version:1, model, element_ids, skeleton}`. Null or absent
/// `element_ids` selects the project; an empty array selects nothing. Output is
/// a versioned read-only `emission` projection, not a persisted model. It keeps
/// literal bytes, graph dependency order and resolved placement without exposing
/// mutable model objects to the renderer.
///
/// This preserves the existing graph-directed generator, not a proof of formal
/// generation correctness or UML conformance. Graph integrity remains required.
/// No I/O, mutation, callbacks, clocks or retained state. Errors use the document
/// error contract or `RUST_GENERATION_PROJECTION_FAILED`; no partial output.
pub fn project_rust_generation(input_json: &str) -> Result<String, String> {
    let request: Request = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    let scope = request.element_ids.map_or(Scope::Project, Scope::Elements);
    let emission = crate::kernel_rust_emission::project(&model, &scope, request.skeleton)
        .map_err(|error| error_response("RUST_GENERATION_PROJECTION_FAILED", error.to_string()))?;
    Ok(serde_json::json!({"api_version": API_VERSION, "emission": emission}).to_string())
}
