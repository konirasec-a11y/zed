use crate::{Diagram, Element, Relation};
use serde::{Deserialize, Serialize};

fn current_schema() -> u32 {
    1
}

/// Persistent bellatrix-mde-model: no GPUI, Cargo, Cordis or filesystem state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "current_schema")]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_model: Option<String>,
    /// Explicit explorer placement. Missing keys follow semantic projection;
    /// a present null parent places the identity at the project root.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub explorer_parents: std::collections::BTreeMap<String, Option<String>>,
    #[serde(default)]
    pub elements: Vec<Element>,
    #[serde(default)]
    pub relations: Vec<Relation>,
    #[serde(default)]
    pub diagrams: Vec<Diagram>,
}

impl Project {
    pub fn element(&self, id: &str) -> Option<&Element> {
        self.elements.iter().find(|element| element.id == id)
    }
}
