use crate::{Diagram, Element, Project, Relation};
use serde::{Deserialize, Serialize};

/// Domain payload inside a platform ProjectDocument, without duplicate identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelingSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_model: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub explorer_parents: std::collections::BTreeMap<String, Option<String>>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub explorer_diagram_occurrences: std::collections::BTreeMap<String, crate::ExplorerDiagramOccurrence>,
    pub elements: Vec<Element>,
    pub relations: Vec<Relation>,
    pub diagrams: Vec<Diagram>,
}

impl ModelingSection {
    pub fn from_model(model: &Project) -> Self {
        Self {
            root_model: model.root_model.clone(),
            explorer_parents: model.explorer_parents.clone(),
            explorer_diagram_occurrences: model.explorer_diagram_occurrences.clone(),
            elements: model.elements.clone(),
            relations: model.relations.clone(),
            diagrams: model.diagrams.clone(),
        }
    }

    pub fn to_model(&self, id: &str, name: &str) -> Project {
        Project {
            schema_version: 1,
            id: id.into(),
            name: name.into(),
            root_model: self.root_model.clone(),
            explorer_parents: self.explorer_parents.clone(),
            explorer_diagram_occurrences: self.explorer_diagram_occurrences.clone(),
            elements: self.elements.clone(),
            relations: self.relations.clone(),
            diagrams: self.diagrams.clone(),
        }
    }
}
