use super::{Annotation, Diagram, FileContent, Implementation, Project};
use crate::ruml::Registry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The only persistent UML state is the supplied typed registry. Presentation
/// metadata references its identities and cannot override contract fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelingSection {
    pub registry: Registry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_model: Option<String>,
    #[serde(default, skip_serializing_if = "super::AuthoringKnowledge::is_empty")]
    pub authoring_knowledge: super::AuthoringKnowledge,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mdj_provenance: Option<super::MdjProvenance>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::protocol::model_snapshot::deserialize_hierarchy"
    )]
    pub hierarchy: BTreeMap<String, crate::HierarchyPlacement>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "super::composition::deserialize_composition"
    )]
    pub composition: BTreeMap<String, crate::CompositionPlacement>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub presentation: BTreeMap<String, SubjectPresentation>,
    #[serde(default)]
    pub diagrams: Vec<Diagram>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubjectPresentation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Decorative notation only; this does not apply a UML Stereotype.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype_label: Option<StereotypeLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implementations: Vec<Implementation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<FileContent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum StereotypeLabel {
    Empty,
    Text(String),
    Reference(String),
}

impl ModelingSection {
    pub fn to_model(self, id: impl Into<String>, name: impl Into<String>) -> Project {
        Project {
            schema_version: 2,
            id: id.into(),
            name: name.into(),
            model: self,
        }
    }
}
