use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MdjProvenance {
    original: MdjOriginalDocument,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub object_paths: BTreeMap<String, String>,
    // Views own their residue in PresentationSource; duplicating it here would
    // create competing editable values for the same source property.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub object_residuals: BTreeMap<String, MdjObjectResidual>,
}

impl MdjProvenance {
    /// The projection must come from a complete converter of the imported
    /// contract, including exported residues, never from the original JSON.
    pub fn capture(
        original_bytes: Vec<u8>,
        projection_version: u32,
        export_projection: &Value,
        object_paths: BTreeMap<String, String>,
        object_residuals: BTreeMap<String, MdjObjectResidual>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            original: MdjOriginalDocument {
                bytes: original_bytes,
                fingerprint: MdjProjectionFingerprint::from_export_projection(
                    projection_version,
                    export_projection,
                )?,
            },
            object_paths,
            object_residuals,
        })
    }

    /// This persisted claim is not proof that the source bytes match a contract.
    /// An exporter must first convert those bytes and verify their projection;
    /// raw-source access is deliberately unavailable outside this crate.
    pub fn recorded_fingerprint(&self) -> &MdjProjectionFingerprint {
        &self.original.fingerprint
    }

    // Only the conversion module may use this as input to reimport and compare
    // complete projections; the persisted fingerprint alone is never proof.
    pub fn source_bytes_for_verification(&self) -> &[u8] {
        &self.original.bytes
    }

    pub fn validate_residual_ownership(
        &self,
        project: &crate::contracts::protocol::model_snapshot::Project,
    ) -> Result<(), String> {
        let view_ids: HashSet<_> = project
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .map(|view| view.id.as_str())
            .collect();
        let view_paths = view_ids
            .iter()
            .filter_map(|id| self.object_paths.get(*id))
            .collect::<Vec<_>>();
        for (id, path) in &self.object_paths {
            if id == &project.id
                || view_ids.contains(id.as_str())
                || view_paths
                    .iter()
                    .any(|view_path| path.starts_with(&format!("{view_path}/")))
            {
                continue;
            }
            if !project.composition.contains_key(id) {
                return Err(format!(
                    "MDJ object {id} has no authored composition. Keep a backup of this edited file; automatic migration is not available. Reimporting the original MDJ does not preserve later edits"
                ));
            }
        }
        for (id, object) in &self.object_residuals {
            if view_ids.contains(id.as_str()) {
                return Err(format!(
                    "MDJ residue for view {id} belongs in PresentationSource, not document provenance"
                ));
            }
            if let Some(field) = object
                .mapped_fields
                .iter()
                .find(|field| object.residual.contains_key(*field))
            {
                return Err(format!(
                    "MDJ field {field} on {id} cannot be both mapped and residual"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MdjOriginalDocument {
    #[serde(with = "source_bytes")]
    bytes: Vec<u8>,
    fingerprint: MdjProjectionFingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MdjProjectionFingerprint {
    algorithm: MdjFingerprintAlgorithm,
    projection_version: u32,
    digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MdjFingerprintAlgorithm {
    CanonicalJsonSha256V1,
}

impl MdjProjectionFingerprint {
    pub fn from_export_projection(
        projection_version: u32,
        export_projection: &Value,
    ) -> Result<Self, serde_json::Error> {
        let bytes = serde_json::to_vec(&canonical_value(export_projection))?;
        Ok(Self {
            algorithm: MdjFingerprintAlgorithm::CanonicalJsonSha256V1,
            projection_version,
            digest: Sha256::digest(bytes).into(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MdjObjectResidual {
    pub type_name: String,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub mapped_fields: BTreeSet<String>,
    // Mapped fields are read only from the authoritative contract on export.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub residual: BTreeMap<String, Value>,
}

fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted: BTreeMap<_, _> = object.iter().collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| (key.clone(), canonical_value(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_value).collect()),
        _ => value.clone(),
    }
}

mod source_bytes {
    use super::*;

    pub fn serialize<S: serde::Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}
