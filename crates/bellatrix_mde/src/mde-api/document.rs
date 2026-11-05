use crate::api_contract::{
    API_VERSION, decode_model, error_response, invalid_model, invalid_request,
};
use crate::{Format, Project, decode, encode};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
}

fn document_format(format: &str) -> Result<Format, String> {
    match format {
        "json" => Ok(Format::Json),
        "protobuf" => Ok(Format::Protobuf),
        _ => Err(error_response(
            "UNSUPPORTED_FORMAT",
            "format must be json or protobuf",
        )),
    }
}

pub(super) fn decode_model_request(input_json: &str) -> Result<Project, String> {
    let request: ModelRequest = serde_json::from_str(input_json).map_err(invalid_request)?;
    decode_model(request.api_version, &request.model)
}

/// Decode an in-memory JSON or BLX2 protobuf document into a versioned model envelope.
///
/// Input is the original document byte slice and an explicit `json` or `protobuf`
/// format string. Protobuf bytes include the existing BLX2 header.
/// Output is `{"api_version":1,"model":...}`; it can be passed directly to
/// `validate_graph_model`. Decoding checks storage structure, not UML semantics.
///
/// # Errors
/// Returns versioned errors: `UNSUPPORTED_FORMAT`, `INVALID_MODEL`, or
/// `RESPONSE_SERIALIZATION_FAILED`. Internal codec/Rust names are not exposed.
///
/// # Effects and determinism
/// No external effects: only in-memory document decoding and JSON encoding.
/// Does not read files, invoke callbacks, access clocks or retain caller state.
pub fn decode_model_document(document: &[u8], format: &str) -> Result<String, String> {
    let format = document_format(format)?;
    let model = decode(document, format).map_err(invalid_model)?;
    let model = model_value(model)?;
    Ok(serde_json::json!({"api_version":API_VERSION,"model":model}).to_string())
}

/// Encode a versioned model envelope as an in-memory JSON or BLX2 document.
///
/// Input contains exactly `api_version: 1` and `model`. The explicit format is
/// `json` or `protobuf`. Output is the existing codec's document bytes, not an
/// API envelope or a base64 string. Codec schema, composition, provenance and
/// identity checks remain in force; this does not validate UML semantics.
///
/// # Errors
/// Returns versioned `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`,
/// `UNSUPPORTED_FORMAT` or `INVALID_MODEL` errors. No bytes are returned on error.
///
/// # Effects and determinism
/// No I/O, clocks, callbacks, publication or retained state. The original model
/// is unchanged. Existing codec normalization applies, not source JSON replay.
pub fn encode_model_document(input_json: &str, format: &str) -> Result<Vec<u8>, String> {
    let format = document_format(format)?;
    let model = decode_model_request(input_json)?;
    encode(&model, format).map_err(invalid_model)
}

/// Convert document bytes with the existing codec and no JSON envelope in between.
///
/// Formats are `json` or `protobuf`; binary input/output uses BLX2.
/// Both formats carry the same canonical JSON contract document.
/// No UML validation, publication, filesystem access or retained state is added.
/// Errors use the versioned `UNSUPPORTED_FORMAT`, `INVALID_MODEL` or
/// `RESPONSE_SERIALIZATION_FAILED` envelope.
pub fn convert_model_document(
    document: &[u8],
    source_format: &str,
    destination_format: &str,
) -> Result<Vec<u8>, String> {
    let source_format = document_format(source_format)?;
    let destination_format = document_format(destination_format)?;
    let model = decode(document, source_format).map_err(invalid_model)?;
    if matches!(destination_format, Format::Json) {
        ensure_json_model(&model)?;
    }
    encode(&model, destination_format).map_err(invalid_model)
}

fn ensure_json_model(model: &Project) -> Result<(), String> {
    // Both encodings carry JSON; reject non-finite dimensions rather than
    // allowing serialization to replace authored values with null.
    if model
        .diagrams
        .iter()
        .any(|diagram| !diagram.width.is_finite() || !diagram.height.is_finite())
    {
        return Err(error_response(
            "RESPONSE_SERIALIZATION_FAILED",
            "non-finite diagram dimensions cannot be represented in JSON",
        ));
    }
    Ok(())
}

pub(super) fn model_value(model: Project) -> Result<serde_json::Value, String> {
    ensure_json_model(&model)?;
    let stored = crate::contract_document::store_document(&model)
        .map_err(|error| error_response("INVALID_MODEL", error))?;
    serde_json::to_value(stored).map_err(|_| {
        error_response(
            "RESPONSE_SERIALIZATION_FAILED",
            "decoded model cannot be encoded as JSON",
        )
    })
}

/// Prepare a decoded model envelope for a platform document without leaking domain types.
/// The section is the same canonical contract, encoded as opaque JSON, not a read projection.
/// Checks presentation integrity, but does not assert full UML validity or retain state.
pub fn prepare_model_document(
    input_json: &str,
) -> Result<bellatrix_mde_contracts::protocol::ModelDocumentParts, String> {
    let model = decode_model_request(input_json)?;
    ensure_json_model(&model)?;
    crate::Graph::new(&model)
        .validate_presentation()
        .map_err(|error| error.to_string())?;
    let section = crate::contract_document::store_section(
        &crate::ModelingSection::from_model(&model),
        &model.id,
    )?;
    let section_json = serde_json::to_string(&section).map_err(|error| error.to_string())?;
    Ok(bellatrix_mde_contracts::protocol::ModelDocumentParts {
        id: model.id,
        name: model.name,
        section_json,
    })
}

/// Check a canonical section before a platform document is opened. No UML validation or I/O.
pub fn validate_model_section(
    project_id: &str,
    name: &str,
    section_json: &str,
) -> Result<(), String> {
    let section = crate::mde_api_projection::decode_section(project_id, name, section_json)?;
    crate::Graph::new(&section.to_model(project_id, name))
        .validate_presentation()
        .map_err(|error| error.to_string())
}

/// Reconstitute document bytes from canonical section JSON and its platform identity.
/// Preserves authoring knowledge, composition and original MDJ bytes in provenance.
/// Both formats use the same canonical JSON contract; binary output wraps it in BLX2.
/// No persistence, publication or UML validation occurs here.
pub fn encode_model_section(
    project_id: &str,
    name: &str,
    section_json: &str,
    format: &str,
) -> Result<Vec<u8>, String> {
    let format = document_format(format)?;
    let section = crate::mde_api_projection::decode_section(project_id, name, section_json)?;
    let model = section.to_model(project_id, name);
    if matches!(format, Format::Json) {
        ensure_json_model(&model)?;
    }
    encode(&model, format).map_err(invalid_model)
}
