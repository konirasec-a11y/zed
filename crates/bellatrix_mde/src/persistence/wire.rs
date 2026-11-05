use super::io::ModelIoError;
use bellatrix_mde_contracts::model::Project;
use prost::Message;

/// Binary transport carries the same typed document as JSON. It does not define
/// a second set of UML fields or flatten the supplied contracts into wire nodes.
#[derive(Clone, PartialEq, Message)]
struct WireProject {
    #[prost(bytes, tag = "1")]
    document_json: Vec<u8>,
}

pub(crate) fn encode(project: &Project) -> Result<Vec<u8>, ModelIoError> {
    Ok(WireProject {
        document_json: serde_json::to_vec(project)?,
    }
    .encode_to_vec())
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Project, ModelIoError> {
    let wire = WireProject::decode(bytes)?;
    Ok(serde_json::from_slice(&wire.document_json)?)
}
