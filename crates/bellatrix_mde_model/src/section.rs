use crate::{Diagram, Element, Project, Relation};
use serde::{Deserialize, Serialize};

/// Domain payload inside a platform ProjectDocument, without duplicate identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelingSection {
    pub elements: Vec<Element>,
    pub relations: Vec<Relation>,
    pub diagrams: Vec<Diagram>,
}

impl ModelingSection {
    pub fn from_model(model: &Project) -> Self {
        Self {
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
            elements: self.elements.clone(),
            relations: self.relations.clone(),
            diagrams: self.diagrams.clone(),
        }
    }
}
