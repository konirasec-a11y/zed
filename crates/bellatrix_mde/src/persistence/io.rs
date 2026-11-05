use crate::Project;
use thiserror::Error;

const MAGIC: &[u8; 4] = b"BLX2";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Protobuf,
}

#[derive(Debug, Error)]
pub enum ModelIoError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid protobuf: {0}")]
    Proto(#[from] prost::DecodeError),
    #[error("invalid base64 asset: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid utf8 asset: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("binary file does not begin with BLX2")]
    Magic,
    #[error("diagram uml_di_type is required and must not be blank")]
    DiagramType,
    #[error("invalid hierarchy: {0}")]
    Hierarchy(String),
    #[error("invalid presentation structure: {0}")]
    Presentation(String),
    #[error("invalid MDJ provenance: {0}")]
    Provenance(String),
    #[error("invalid authoring knowledge encoding: {0}")]
    AuthoringKnowledge(String),
    #[error("invalid semantic storage: {0}")]
    SemanticStorage(String),
    #[error("unsupported schema version: {0}")]
    Version(u32),
}

pub fn encode(project: &Project, format: Format) -> Result<Vec<u8>, ModelIoError> {
    ensure_version(project.schema_version)?;
    crate::composition_paths(&project).map_err(ModelIoError::Hierarchy)?;
    if let Some(provenance) = &project.mdj_provenance {
        provenance
            .validate_residual_ownership(&project)
            .map_err(ModelIoError::Provenance)?;
    }
    crate::Graph::new(&project)
        .validate_identifiers()
        .map_err(|error| ModelIoError::Presentation(error.to_string()))?;
    let stored =
        crate::contract_document::store_document(project).map_err(ModelIoError::SemanticStorage)?;
    match format {
        Format::Json => Ok(serde_json::to_vec_pretty(&stored)?),
        Format::Protobuf => {
            let mut out = MAGIC.to_vec();
            out.extend(crate::wire::encode(&stored)?);
            Ok(out)
        }
    }
}

pub fn decode(bytes: &[u8], format: Format) -> Result<Project, ModelIoError> {
    let stored = match format {
        Format::Json => serde_json::from_slice(bytes)?,
        Format::Protobuf => {
            let payload = bytes.strip_prefix(MAGIC).ok_or(ModelIoError::Magic)?;
            crate::wire::decode(payload)?
        }
    };
    let project = crate::contract_document::project_document(stored)
        .map_err(ModelIoError::SemanticStorage)?;
    ensure_version(project.schema_version)?;
    crate::composition_paths(&project).map_err(ModelIoError::Hierarchy)?;
    if let Some(provenance) = &project.mdj_provenance {
        provenance
            .validate_residual_ownership(&project)
            .map_err(ModelIoError::Provenance)?;
    }
    crate::Graph::new(&project)
        .validate_identifiers()
        .map_err(|error| ModelIoError::Presentation(error.to_string()))?;
    Ok(project)
}

fn ensure_version(version: u32) -> Result<(), ModelIoError> {
    if version == 2 {
        Ok(())
    } else {
        Err(ModelIoError::Version(version))
    }
}
