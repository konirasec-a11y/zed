use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Authored knowledge gaps are not properties of the OMG metamodel and are not
/// source provenance. They remain semantic inputs after the source is removed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringKnowledge {
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "deserialize_associations"
    )]
    pub associations: BTreeMap<String, AssociationKnowledge>,
}

impl AuthoringKnowledge {
    pub fn is_empty(&self) -> bool {
        self.associations.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssociationKnowledge {
    /// Association-owned Property identities whose navigation is not decided.
    /// A Vec retains duplicate declarations for kernel validation to reject.
    pub navigation_unverified: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum NavigationKnowledge {
    Known(bool),
    Unverified,
}

fn deserialize_associations<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, AssociationKnowledge>, D::Error> {
    struct AssociationsVisitor;
    impl<'de> serde::de::Visitor<'de> for AssociationsVisitor {
        type Value = BTreeMap<String, AssociationKnowledge>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("authoring knowledge with unique Association identities")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut associations = BTreeMap::new();
            while let Some((id, knowledge)) = map.next_entry::<String, AssociationKnowledge>()? {
                if associations.insert(id.clone(), knowledge).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate Association knowledge identity: {id}"
                    )));
                }
            }
            Ok(associations)
        }
    }
    deserializer.deserialize_map(AssociationsVisitor)
}
