use crate::{Diagram, Element, Project, Relation};
use serde::{Deserialize, Serialize};

/// Domain payload inside a platform ProjectDocument, without duplicate identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelingSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mdj_provenance: Option<super::MdjProvenance>,
    #[serde(
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty",
        deserialize_with = "super::project::deserialize_hierarchy"
    )]
    pub hierarchy: std::collections::BTreeMap<String, crate::HierarchyPlacement>,
    pub elements: Vec<Element>,
    pub relations: Vec<Relation>,
    pub diagrams: Vec<Diagram>,
}

impl ModelingSection {
    pub fn from_model(model: &Project) -> Self {
        Self {
            root_model: model.root_model.clone(),
            mdj_provenance: model.mdj_provenance.clone(),
            hierarchy: model.hierarchy.clone(),
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
            mdj_provenance: self.mdj_provenance.clone(),
            hierarchy: self.hierarchy.clone(),
            elements: self.elements.clone(),
            relations: self.relations.clone(),
            diagrams: self.diagrams.clone(),
        }
    }
}
