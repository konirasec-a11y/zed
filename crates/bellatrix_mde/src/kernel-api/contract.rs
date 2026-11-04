use crate::ModelIoError;
use crate::{Format, Project, decode};
use serde::Deserialize;

pub(super) const API_VERSION: u32 = 1;

pub(super) fn error_response(code: &str, message: impl Into<String>) -> String {
    serde_json::json!({
        "api_version": API_VERSION,
        "error": { "code": code, "message": message.into() },
    })
    .to_string()
}

pub(super) fn check_version(api_version: u32) -> Result<(), String> {
    if api_version != API_VERSION {
        return Err(error_response(
            "UNSUPPORTED_API_VERSION",
            format!("expected api_version {API_VERSION}, received {api_version}"),
        ));
    }
    Ok(())
}

pub(super) fn decode_model(api_version: u32, model: &serde_json::value::RawValue) -> Result<Project, String> {
    check_version(api_version)?;
    decode(model.get().as_bytes(), Format::Json).map_err(invalid_model)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UmlValidationDocument {
    #[serde(default = "model_schema_version")]
    schema_version: u32,
    id: String,
    name: String,
    #[serde(default)]
    root_model: Option<String>,
    #[serde(default)]
    authoring_knowledge: crate::AuthoringKnowledge,
    #[serde(default)]
    elements: Vec<crate::Element>,
    #[serde(default)]
    relations: Vec<crate::Relation>,
    #[serde(default, rename = "diagrams")]
    _diagrams: serde::de::IgnoredAny,
    #[serde(default, rename = "hierarchy")]
    _hierarchy: serde::de::IgnoredAny,
    #[serde(default, rename = "composition")]
    _composition: serde::de::IgnoredAny,
    #[serde(default, rename = "mdj_provenance")]
    _mdj_provenance: serde::de::IgnoredAny,
}

fn model_schema_version() -> u32 {
    1
}

pub(super) fn decode_uml_model(api_version: u32, model: &serde_json::value::RawValue) -> Result<Project, String> {
    check_version(api_version)?;
    let document: UmlValidationDocument = serde_json::from_str(model.get())
        .map_err(|error| invalid_model(ModelIoError::Json(error)))?;
    if document.schema_version != 1 {
        return Err(invalid_model(ModelIoError::Version(
            document.schema_version,
        )));
    }
    // This is an evaluation projection, not persistence decoding or repair. Even
    // malformed visual payloads must not suppress semantic UML diagnostics.
    Ok(Project {
        schema_version: document.schema_version,
        id: document.id,
        name: document.name,
        root_model: document.root_model,
        authoring_knowledge: document.authoring_knowledge,
        elements: document.elements,
        relations: document.relations,
        diagrams: Vec::new(),
        hierarchy: Default::default(),
        composition: Default::default(),
        mdj_provenance: None,
    })
}

pub(super) fn invalid_model(error: ModelIoError) -> String {
    match error {
        ModelIoError::Version(version) => error_response(
            "INVALID_MODEL",
            format!("unsupported model schema_version: {version}"),
        ),
        _ => error_response(
            "INVALID_MODEL",
            "model does not match the supported document schema",
        ),
    }
}

pub(super) fn invalid_request(error: serde_json::Error) -> String {
    error_response(
        "INVALID_REQUEST",
        format!(
            "invalid JSON or request fields at line {}, column {}",
            error.line(),
            error.column()
        ),
    )
}
