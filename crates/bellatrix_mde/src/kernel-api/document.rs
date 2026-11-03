use crate::{Format, decode};
use crate::kernel_api_contract::{API_VERSION, error_response, invalid_model};
/// Decode an in-memory JSON or BLX1 protobuf document into a versioned model envelope.
///
/// Input is the original document byte slice and an explicit `json` or `protobuf`
/// format string. Protobuf bytes include the existing BLX1 header.
/// Output is `{"api_version":1,"model":...}`; it can be passed directly to
/// `validate_graph_model` or `validate_uml_model`. Existing codec defaults and
/// normalization apply. Decoding does not validate graph or UML semantics.
///
/// # Errors
/// Returns versioned errors: `UNSUPPORTED_FORMAT`, `INVALID_MODEL`, or
/// `RESPONSE_SERIALIZATION_FAILED`. Internal codec/Rust names are not exposed.
///
/// # Effects and determinism
/// No external effects: only in-memory document decoding and JSON encoding.
/// Does not read files, invoke callbacks, access clocks or retain caller state.
pub fn decode_model_document(document: &[u8], format: &str) -> Result<String, String> {
    let format = match format {
        "json" => Format::Json,
        "protobuf" => Format::Protobuf,
        _ => return Err(error_response("UNSUPPORTED_FORMAT", "format must be json or protobuf")),
    };
    let model = decode(document, format).map_err(invalid_model)?;
    // Protobuf can carry non-finite dimensions, while JSON would silently turn
    // them into null. Reject that representation loss instead of changing data.
    if model.diagrams.iter().any(|diagram| !diagram.width.is_finite() || !diagram.height.is_finite()) {
        return Err(error_response("RESPONSE_SERIALIZATION_FAILED", "non-finite diagram dimensions cannot be represented in JSON"));
    }
    let model = serde_json::to_value(model)
        .map_err(|_| error_response("RESPONSE_SERIALIZATION_FAILED", "decoded model cannot be encoded as JSON"))?;
    Ok(serde_json::json!({"api_version":API_VERSION,"model":model}).to_string())
}
