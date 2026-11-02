use crate::Project;
use thiserror::Error;

const MAGIC: &[u8; 4] = b"BLX1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format { Json, Protobuf }

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
    #[error("binary file does not begin with BLX1")]
    Magic,
    #[error("unsupported schema version: {0}")]
    Version(u32),
}

pub fn encode(project: &Project, format: Format) -> Result<Vec<u8>, ModelIoError> {
    ensure_version(project.schema_version)?;
    match format {
        Format::Json => Ok(serde_json::to_vec_pretty(project)?),
        Format::Protobuf => {
            let mut out = MAGIC.to_vec();
            out.extend(crate::wire::encode(project)?);
            Ok(out)
        }
    }
}

pub fn decode(bytes: &[u8], format: Format) -> Result<Project, ModelIoError> {
    let project = match format {
        Format::Json => serde_json::from_slice(bytes)?,
        Format::Protobuf => {
            let payload = bytes.strip_prefix(MAGIC).ok_or(ModelIoError::Magic)?;
            crate::wire::decode(payload)?
        }
    };
    ensure_version(project.schema_version)?;
    Ok(project)
}

fn ensure_version(version: u32) -> Result<(), ModelIoError> {
    if version == 1 { Ok(()) } else { Err(ModelIoError::Version(version)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_and_protobuf_are_equivalent() {
        let project: Project = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/todo.json"))).unwrap();
        let json = decode(&encode(&project, Format::Json).unwrap(), Format::Json).unwrap();
        let binary = decode(&encode(&project, Format::Protobuf).unwrap(), Format::Protobuf).unwrap();
        assert_eq!(json, project);
        assert_eq!(binary, project);
    }
    #[test]
    fn rejects_unknown_schema() {
        let bytes = br#"{"schema_version":99,"id":"x","name":"y"}"#;
        assert!(matches!(decode(bytes, Format::Json), Err(ModelIoError::Version(99))));
    }
}
