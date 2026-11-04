use crate::artifact_publication::{self, PreparedArtifacts};

pub(crate) fn prepare(
    model_envelope: &str,
    element_ids: Option<Vec<String>>,
    skeleton: bool,
) -> Result<PreparedArtifacts, String> {
    let mut request: serde_json::Value =
        serde_json::from_str(model_envelope).map_err(|error| error.to_string())?;
    let fields = request
        .as_object_mut()
        .ok_or("model envelope must be an object")?;
    fields.insert(
        "element_ids".into(),
        serde_json::to_value(element_ids).map_err(|error| error.to_string())?,
    );
    fields.insert("skeleton".into(), skeleton.into());
    let projection = bellatrix_mde::project_rust_generation(&request.to_string())?;
    artifact_publication::prepare(&bellatrix_mde_rust_generator::render(&projection)?)
}
