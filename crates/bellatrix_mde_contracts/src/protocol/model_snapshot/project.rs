use super::{Diagram, Element, HierarchyPlacement, Relation};
use serde::{Deserialize, Serialize};

fn current_schema() -> u32 {
    2
}

/// Persistent bellatrix-mde-model: no GPUI, Cargo, Cordis or filesystem state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    #[serde(default = "current_schema")]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_model: Option<String>,
    #[serde(default, skip_serializing_if = "super::AuthoringKnowledge::is_empty")]
    pub authoring_knowledge: super::AuthoringKnowledge,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mdj_provenance: Option<super::MdjProvenance>,
    #[serde(
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty",
        deserialize_with = "deserialize_hierarchy"
    )]
    pub hierarchy: std::collections::BTreeMap<String, crate::HierarchyPlacement>,
    #[serde(
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty",
        deserialize_with = "crate::model::composition::deserialize_composition"
    )]
    pub composition: std::collections::BTreeMap<String, crate::CompositionPlacement>,
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

    pub fn validate_hierarchy(&self) -> Result<(), String> {
        use std::collections::{BTreeMap, HashSet};
        let diagrams: BTreeMap<_, _> = self
            .diagrams
            .iter()
            .map(|diagram| (diagram.id.as_str(), diagram))
            .collect();
        let mut containers = BTreeMap::new();
        for (subject_id, placement) in &self.hierarchy {
            if subject_id.trim().is_empty() {
                return Err("Hierarchy subject identity must not be empty".into());
            }
            if let Some(parent) = &placement.parent_id {
                if diagrams.contains_key(parent.as_str()) || self.hierarchy.contains_key(parent) {
                    containers.insert(subject_id.as_str(), parent.as_str());
                }
            }
            if let Some(occurrence) = &placement.occurrence {
                // Pending semantic references are persisted for later kernel diagnostics.
                // Only the occurrence graph itself must remain safe to traverse here.
                let anchor = diagrams
                    .get(occurrence.diagram_id.as_str())
                    .and_then(|diagram| {
                        diagram
                            .all_views()
                            .find(|view| view.id == occurrence.view_id)
                    })
                    .and_then(|view| view.subject.as_deref());
                let mut members = HashSet::new();
                if let Some(anchor) = anchor {
                    members.insert(anchor);
                }
                for member in &occurrence.member_path {
                    if !members.insert(member.as_str()) {
                        return Err("View occurrence member path contains a cycle".into());
                    }
                }
                containers.insert(subject_id.as_str(), occurrence.diagram_id.as_str());
            }
        }
        for subject_id in self.hierarchy.keys() {
            let mut visited = HashSet::new();
            let mut cursor = Some(subject_id.as_str());
            while let Some(current) = cursor {
                if !visited.insert(current) {
                    return Err(format!(
                        "Hierarchy contains a diagram occurrence cycle at {current}"
                    ));
                }
                cursor = containers.get(current).copied();
            }
        }
        Ok(())
    }
}

pub(crate) fn deserialize_hierarchy<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, HierarchyPlacement>, D::Error> {
    struct HierarchyVisitor;
    impl<'de> serde::de::Visitor<'de> for HierarchyVisitor {
        type Value = std::collections::BTreeMap<String, HierarchyPlacement>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a hierarchy map with unique subject identities")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut hierarchy = std::collections::BTreeMap::new();
            while let Some((subject, placement)) = map.next_entry::<String, HierarchyPlacement>()? {
                if hierarchy.insert(subject.clone(), placement).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate hierarchy subject: {subject}"
                    )));
                }
            }
            Ok(hierarchy)
        }
    }
    deserializer.deserialize_map(HierarchyVisitor)
}
