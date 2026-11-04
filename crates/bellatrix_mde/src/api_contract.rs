use crate::{Format, ModelIoError, Project, decode};

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

pub(super) fn decode_model(
    api_version: u32,
    model: &serde_json::value::RawValue,
) -> Result<Project, String> {
    check_version(api_version)?;
    decode(model.get().as_bytes(), Format::Json).map_err(invalid_model)
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
