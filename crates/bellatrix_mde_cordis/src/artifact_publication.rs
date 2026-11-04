use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreparedArtifacts {
    api_version: u32,
    manifest: Box<serde_json::value::RawValue>,
    pub artifact_count: usize,
    pub warning_count: usize,
    pub report: String,
}

impl PreparedArtifacts {
    pub(crate) fn publish(&self, destination: &Path, replace_generated: bool) -> Result<(), String> {
        bellatrix_mde::publish_artifact_manifest(self.manifest.get(), destination, replace_generated)
    }
}

pub(crate) fn prepare(manifest: &str) -> Result<PreparedArtifacts, String> {
    let response = bellatrix_mde::prepare_artifact_publication(manifest)?;
    let prepared: PreparedArtifacts = serde_json::from_str(&response)
        .map_err(|error| error.to_string())?;
    if prepared.api_version != 1 {
        return Err("unsupported artifact preparation response version".into());
    }
    Ok(prepared)
}
