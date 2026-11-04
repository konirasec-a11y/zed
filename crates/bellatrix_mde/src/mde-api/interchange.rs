use crate::api_contract::{API_VERSION, error_response};
use crate::mde_api_document::{decode_model_request, model_value};
use crate::mdj_conversion;
use serde_json::json;

/// Import original MDJ bytes into the universal document and explicit diagnostics.
///
/// Output is `{"api_version":1,"model":...,"diagnostics":[...]}`. Each diagnostic
/// contains `path` and `message`, preserving the adapter's order and content.
/// The model retains original bytes in provenance, source residues, composition
/// and authoring knowledge. This is the existing conservative MDJ mapping, not
/// full StarUML parity or UML validation.
///
/// Export, codec and validation requests accept only `{api_version, model}`.
/// Extract that envelope without interpreting the model through a lossy numeric
/// representation; keep diagnostics separately. In Rust, retain `model` as
/// `serde_json::value::RawValue` while extracting the response envelope.
///
/// # Errors
/// Returns versioned `MDJ_IMPORT_FAILED` or `RESPONSE_SERIALIZATION_FAILED`
/// errors. Import rejection retains the existing adapter's explanatory message.
///
/// # Effects and determinism
/// No I/O, clocks, callbacks or retained state. The caller owns the source bytes
/// and decides whether to publish or save the returned document.
pub fn import_mdj_document(document: &[u8]) -> Result<String, String> {
    let imported = mdj_conversion::import(document)
        .map_err(|error| error_response("MDJ_IMPORT_FAILED", error.to_string()))?;
    let model = model_value(imported.project)?;
    let diagnostics = imported
        .diagnostics
        .into_iter()
        .map(|diagnostic| json!({"path": diagnostic.path, "message": diagnostic.message}))
        .collect::<Vec<_>>();
    Ok(json!({"api_version": API_VERSION, "model": model, "diagnostics": diagnostics}).to_string())
}

/// Experimental: export a universal model envelope through the supported MDJ mappings.
/// Unsupported edits are rejected; this does not guarantee full StarUML compatibility.
///
/// Input contains exactly `api_version: 1` and `model`, without import diagnostics.
/// The current adapter requires imported provenance. An unchanged projection
/// returns original bytes. Edited export reimports its actual output and checks
/// the supported inverse mapping; unsupported edits fail rather than discard data.
/// No new mappings, composition edits or UML conformance claims are introduced.
///
/// # Errors
/// Returns versioned `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`, `INVALID_MODEL`
/// or `MDJ_EXPORT_FAILED` errors, with no partial document bytes on failure.
///
/// # Effects and determinism
/// No I/O, clocks, callbacks, publication, save acknowledgement or retained state.
/// The input remains unchanged. The caller must guard the source snapshot before
/// publishing or saving the resulting bytes.
pub fn export_mdj_document(input_json: &str) -> Result<Vec<u8>, String> {
    let model = decode_model_request(input_json)?;
    mdj_conversion::export(&model)
        .map_err(|error| error_response("MDJ_EXPORT_FAILED", error.to_string()))
}

/// Experimental: export an immutable canonical section through the supported MDJ mappings.
/// Unsupported edits are rejected; this does not guarantee full StarUML compatibility.
/// The integration layer must reject unrelated platform sections and guard the snapshot.
/// Original provenance bytes and the existing inverse-mapping verification are preserved.
pub fn export_mdj_section(
    project_id: &str,
    name: &str,
    section_json: &str,
) -> Result<Vec<u8>, String> {
    let section = crate::mde_api_projection::decode_section(project_id, name, section_json)?;
    let model = section.to_model(project_id, name);
    if model.mdj_provenance.is_none() {
        return Err("MDJ interchange requires an imported StarUML document".into());
    }
    mdj_conversion::export(&model)
        .map_err(|error| error_response("MDJ_EXPORT_FAILED", error.to_string()))
}
