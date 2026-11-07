use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub api_version: u32,
    pub emission: Emission,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Emission {
    pub files: Vec<Artifact>,
    pub manifests: Vec<Manifest>,
    pub units: Vec<Unit>,
    pub skeleton: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TargetElement {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub properties: BTreeMap<String, Value>,
    pub annotations: Vec<Source>,
    pub implementations: Vec<Source>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub language: String,
    pub source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub element: TargetElement,
    pub path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Unit {
    pub element: TargetElement,
    pub children: Vec<TargetElement>,
    pub requires: Vec<Value>,
    pub detached: bool,
    pub path: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Artifact {
    pub path: String,
    pub contents: Option<Vec<u8>>,
    pub origin: String,
}

#[derive(Serialize)]
pub(super) struct Diagnostic {
    pub code: String,
    pub message: String,
    pub element: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Rendered {
    pub api_version: u32,
    pub artifacts: Vec<Artifact>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Rendered {
    // This only collects renderer output. The integration handler reconstructs
    // the kernel Plan, which remains the path and collision publication gate.
    pub fn file(&mut self, path: String, contents: Vec<u8>, origin: &str) {
        self.artifacts.push(Artifact {
            path,
            contents: Some(contents),
            origin: origin.to_owned(),
        });
    }

    pub fn warn(&mut self, code: &str, message: &str, element: Option<&str>) {
        self.diagnostics.push(Diagnostic {
            code: code.to_owned(),
            message: message.to_owned(),
            element: element.map(str::to_owned),
        });
    }
}
