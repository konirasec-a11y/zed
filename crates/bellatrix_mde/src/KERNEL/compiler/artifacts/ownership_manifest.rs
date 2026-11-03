use crate::Plan;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

pub(crate) const MARKER: &str = ".bellatrix-generated.json";

#[derive(Serialize, Deserialize)]
pub(crate) struct OwnershipManifest {
    format: String,
    version: u32,
    files: BTreeSet<String>,
}

fn key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

impl OwnershipManifest {
    pub(crate) fn is_supported(&self) -> bool {
        self.format == "bellatrix.generated" && self.version == 1
    }

    pub(crate) fn owns_file(&self, path: &Path) -> bool {
        self.files.contains(&key(path))
    }
}

pub(crate) fn encode(plan: &Plan) -> Result<Vec<u8>, serde_json::Error> {
    let files = plan
        .artifacts()
        .filter(|artifact| artifact.contents.is_some())
        .map(|artifact| key(&artifact.path))
        .collect();
    serde_json::to_vec_pretty(&OwnershipManifest {
        format: "bellatrix.generated".into(),
        version: 1,
        files,
    })
}

pub(crate) fn decode(bytes: &[u8]) -> Result<OwnershipManifest, serde_json::Error> {
    serde_json::from_slice(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip_preserves_normalized_file_ownership() -> Result<(), Box<dyn std::error::Error>> {
        let mut plan = Plan::new();
        plan.file("src/Main.rs", b"main".to_vec(), "test")?;
        plan.directory("empty", "test")?;
        let bytes = encode(&plan)?;
        let manifest = decode(&bytes)?;
        assert!(manifest.is_supported());
        assert!(manifest.owns_file(Path::new("SRC\\MAIN.RS")));
        assert!(!manifest.owns_file(Path::new("empty")));
        assert!(!manifest.owns_file(Path::new("untracked.txt")));
        assert_eq!(bytes, encode(&plan)?);
        Ok(())
    }

    #[test]
    fn unsupported_manifest_is_distinct_from_malformed_bytes() -> Result<(), serde_json::Error> {
        let manifest = decode(br#"{"format":"bellatrix.generated","version":2,"files":[]}"#)?;
        assert!(!manifest.is_supported());
        assert!(decode(b"invalid JSON").is_err());
        Ok(())
    }
}
