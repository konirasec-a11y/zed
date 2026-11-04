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
