//! Authored document composition, independent of UMLDI context and view occurrences.
use serde::{Deserialize, Serialize};

/// A mapped ownership edge is referenced, never copied into a second editable parent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompositionParent {
    Object { id: String },
    SemanticOwner { relation_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionPlacement {
    pub parent: CompositionParent,
    /// Escaped JSON pointer relative to the nearest identified parent, excluding
    /// the final array index. Scalar object slots have no order.
    pub slot: String,
    pub order: Option<usize>,
}

impl CompositionPlacement {
    pub fn parent_id<'a>(
        &'a self,
        subject: &str,
        relations: &'a [crate::contracts::protocol::model_snapshot::Relation],
    ) -> Option<&'a str> {
        match &self.parent {
            CompositionParent::Object { id } => Some(id),
            CompositionParent::SemanticOwner { relation_id } => relations
                .iter()
                .find(|relation| {
                    relation.id == *relation_id && relation.kind == "owns" && relation.to == subject
                })
                .map(|relation| relation.from.as_str()),
        }
    }
    pub fn relative_path(&self) -> String {
        match self.order {
            Some(order) => format!("{}/{order}", self.slot),
            None => self.slot.clone(),
        }
    }
}

/// Structural resolution only. No UML metaclass or ownership admissibility rules.
pub fn composition_paths(
    project: &crate::contracts::protocol::model_snapshot::Project,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    use std::collections::{BTreeMap, HashSet};
    let mut paths = BTreeMap::from([(project.id.clone(), String::new())]);
    let mut pending = project.composition.iter().collect::<Vec<_>>();
    let identities = project
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .chain(
            project
                .relations
                .iter()
                .map(|relation| relation.id.as_str()),
        )
        .chain(project.diagrams.iter().map(|diagram| diagram.id.as_str()))
        .collect::<HashSet<_>>();
    let mut occupied = HashSet::new();
    while !pending.is_empty() {
        let before = pending.len();
        let mut unresolved = Vec::new();
        for (id, placement) in pending {
            if !identities.contains(id.as_str()) {
                return Err(format!("Composition subject not found: {id}"));
            }
            if !placement.slot.starts_with('/') {
                return Err(format!("Invalid composition slot for {id}"));
            }
            let parent = placement
                .parent_id(id, &project.relations)
                .ok_or_else(|| format!("Composition owner reference not found: {id}"))?;
            if let Some(parent_path) = paths.get(parent) {
                let path = format!("{parent_path}{}", placement.relative_path());
                if !occupied.insert(path.clone()) {
                    return Err(format!("Duplicate composition slot: {path}"));
                }
                paths.insert(id.clone(), path);
            } else {
                unresolved.push((id, placement));
            }
        }
        if unresolved.len() == before {
            return Err("Composition contains an absent parent or cycle".into());
        }
        pending = unresolved;
    }
    Ok(paths)
}

/// Reject ambiguous keys instead of silently retaining only the last placement.
pub fn deserialize_composition<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, CompositionPlacement>, D::Error> {
    struct CompositionVisitor;
    impl<'de> serde::de::Visitor<'de> for CompositionVisitor {
        type Value = std::collections::BTreeMap<String, CompositionPlacement>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("composition with unique object identities")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut composition = std::collections::BTreeMap::new();
            while let Some((id, placement)) = map.next_entry::<String, CompositionPlacement>()? {
                if composition.insert(id.clone(), placement).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "Duplicate composition identity: {id}"
                    )));
                }
            }
            Ok(composition)
        }
    }
    deserializer.deserialize_map(CompositionVisitor)
}
