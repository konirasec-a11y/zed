use crate::api_contract::{API_VERSION, decode_model, error_response, invalid_request};
use crate::{Graph, commands};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReparentRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    element_id: String,
    owner_id: String,
}

/// Compute an atomic semantic ownership change without publishing it.
///
/// Input contains exactly `api_version: 1`, `model`, `element_id` and `owner_id`.
/// Both identities are required strings. Detaching to an absent owner is not
/// part of this contract. Output is `{api_version: 1, model: ...}`, directly
/// consumable by the model validation functions.
///
/// The existing SetOwner command enforces supported UML composite ownership,
/// root restrictions, graph integrity/cycles and its semantic change checks.
/// It preserves pre-existing semantic diagnostics according to that command's
/// change-validation policy; success does not certify complete UML conformance.
/// This is a semantic move, not an Explorer placement or filesystem move.
/// Presentation overrides are preserved and may still mask the semantic owner.
///
/// # Errors
/// Contract errors use the standard versioned JSON envelope. Invalid input graph
/// integrity returns `GRAPH_INTEGRITY`; rejected changes return
/// `OWNERSHIP_REPARENT_REJECTED`. No partial model is returned on any failure.
///
/// # Effects and determinism
/// No I/O, clock, sink, callback or retained state. The input is unchanged and the
/// result depends only on it. This function computes a candidate, not a commit:
/// the caller must check the source revision and publish with compare-and-swap.
/// No revision supplied by a client can replace that application-level check.
/// Internal model and command types do not appear in the public signature.
pub fn reparent_uml_element(input_json: &str) -> Result<String, String> {
    let request: ReparentRequest = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    Graph::new(&model)
        .validate()
        .map_err(|error| error_response("GRAPH_INTEGRITY", error.to_string()))?;
    let command = commands::Command::SetOwner {
        element_id: request.element_id,
        owner_id: Some(request.owner_id),
    };
    // SetOwner allocates ownership identities from the model, not this revision
    // seed. Revision locking belongs to the application publishing the candidate.
    let updated = commands::apply(&model, &command, 0)
        .map_err(|error| error_response("OWNERSHIP_REPARENT_REJECTED", error))?;
    let model = serde_json::to_value(updated).map_err(|_| {
        error_response(
            "RESPONSE_SERIALIZATION_FAILED",
            "updated model cannot be encoded as JSON",
        )
    })?;
    Ok(json!({"api_version": API_VERSION, "model": model}).to_string())
}

/// Apply an integration intent to an immutable section and return a candidate section.
/// The caller owns revision checks and atomic publication. Imported and native
/// documents use the same command semantics; MDJ export is a separate operation.
/// No publication, I/O, or alternate persistent model is performed here.
pub fn apply_model_command(
    project_id: &str,
    name: &str,
    section_json: &str,
    revision: u64,
    command: bellatrix_mde_protocol::command::Command,
) -> Result<serde_json::Value, String> {
    let section = crate::mde_api_projection::decode_section(project_id, name, section_json)?;
    let source = section.to_model(project_id, name);
    let command = crate::mde_api_command_conversion::command(command)?;
    let updated = commands::apply(&source, &command, revision)?;
    serde_json::to_value(crate::ModelingSection::from_model(&updated))
        .map_err(|error| error.to_string())
}
