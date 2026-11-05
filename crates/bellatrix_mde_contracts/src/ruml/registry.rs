use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::ruml::{Contract, ContractType};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub id: String,
    #[serde(flatten)]
    pub contract: Contract,
}

impl<'de> Deserialize<'de> for RegistryEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct StoredEntry {
            id: String,
            #[serde(flatten)]
            contract: Contract,
        }
        let stored = StoredEntry::deserialize(deserializer)?;
        let entry = Self {
            id: stored.id,
            contract: stored.contract,
        };
        entry.validate().map_err(serde::de::Error::custom)?;
        Ok(entry)
    }
}

impl RegistryEntry {
    pub fn new(contract: Contract) -> Self {
        Self {
            id: contract.id().to_owned(),
            contract,
        }
    }

    pub fn type_name(&self) -> &'static str {
        self.contract.type_name()
    }

    pub fn data(&self) -> Result<serde_json::Value, String> {
        let value = serde_json::to_value(&self.contract).map_err(|error| error.to_string())?;
        value
            .get("data")
            .cloned()
            .ok_or_else(|| "Missing contract data".to_owned())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Registry ID must not be empty".to_owned());
        }
        if self.id != self.contract.id() {
            return Err(format!(
                "Registry ID {} does not match contract ID {}",
                self.id,
                self.contract.id()
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    entries: BTreeMap<String, RegistryEntry>,
}

impl<'de> Deserialize<'de> for Registry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct StoredRegistry {
            #[serde(deserialize_with = "deserialize_entries")]
            entries: BTreeMap<String, RegistryEntry>,
        }
        let stored = StoredRegistry::deserialize(deserializer)?;
        let registry = Self {
            entries: stored.entries,
        };
        registry.validate().map_err(serde::de::Error::custom)?;
        Ok(registry)
    }
}

fn deserialize_entries<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, RegistryEntry>, D::Error> {
    struct EntriesVisitor;

    impl<'de> serde::de::Visitor<'de> for EntriesVisitor {
        type Value = BTreeMap<String, RegistryEntry>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a registry map with unique identity keys")
        }

        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut entries = BTreeMap::new();
            while let Some((id, entry)) = map.next_entry::<String, RegistryEntry>()? {
                if entries.contains_key(&id) {
                    return Err(serde::de::Error::custom(format!(
                        "Duplicate registry key: {id}"
                    )));
                }
                entries.insert(id, entry);
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_map(EntriesVisitor)
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &BTreeMap<String, RegistryEntry> {
        &self.entries
    }

    pub fn get_entry(&self, id: &str) -> Option<&RegistryEntry> {
        self.entries.get(id)
    }

    pub fn validate(&self) -> Result<(), String> {
        for (id, entry) in &self.entries {
            if id.trim().is_empty() {
                return Err("Registry key must not be empty".to_owned());
            }
            if id != &entry.id {
                return Err(format!(
                    "Registry key {id} does not match entry ID {}",
                    entry.id
                ));
            }
            entry.validate()?;
        }
        Ok(())
    }

    pub fn insert_entry(&mut self, entry: RegistryEntry) -> Result<(), String> {
        entry.validate()?;
        if let Some(previous) = self.entries.get(&entry.id) {
            if previous.type_name() != entry.type_name() {
                return Err(format!(
                    "Registry ID {} already has type {}",
                    entry.id,
                    previous.type_name()
                ));
            }
        }
        self.entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    pub fn insert_contract(&mut self, contract: Contract) -> Result<(), String> {
        self.insert_entry(RegistryEntry::new(contract))
    }

    pub fn insert<T: Serialize + ContractType>(
        &mut self,
        id: String,
        type_name: &str,
        instance: &T,
    ) -> Result<(), String> {
        if type_name != T::TYPE_NAME {
            return Err(format!(
                "Expected type {}, received {type_name}",
                T::TYPE_NAME
            ));
        }
        let data = serde_json::to_value(instance).map_err(|error| error.to_string())?;
        let contract = Contract::from_data(type_name, data)?;
        self.insert_entry(RegistryEntry { id, contract })
    }

    pub fn get<T: for<'de> Deserialize<'de> + ContractType>(&self, id: &str) -> Option<T> {
        let entry = self.entries.get(id)?;
        if entry.type_name() != T::TYPE_NAME {
            return None;
        }
        serde_json::from_value(entry.data().ok()?).ok()
    }

    pub fn get_as_string(&self, id: &str) -> Result<String, String> {
        let entry = self.entries.get(id).ok_or_else(|| "Not found".to_owned())?;
        serde_json::to_string(&entry.data()?).map_err(|error| error.to_string())
    }

    pub fn update_from_string(
        &mut self,
        id: String,
        type_name: &str,
        value: String,
    ) -> Result<(), String> {
        let data = serde_json::from_str(&value).map_err(|error| error.to_string())?;
        let contract = Contract::from_data(type_name, data)?;
        self.insert_entry(RegistryEntry { id, contract })
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.entries.remove(id).is_some()
    }

    pub fn exists(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    pub fn get_all_of_type(&self, type_name: &str) -> Result<Vec<serde_json::Value>, String> {
        self.entries
            .values()
            .filter(|entry| entry.type_name() == type_name)
            .map(RegistryEntry::data)
            .collect()
    }

    pub fn get_all_of_type_as_string(&self, type_name: &str) -> Result<String, String> {
        serde_json::to_string(&self.get_all_of_type(type_name)?).map_err(|error| error.to_string())
    }

    pub fn clear_type(&mut self, type_name: &str) -> usize {
        let previous_count = self.entries.len();
        self.entries
            .retain(|_, entry| entry.type_name() != type_name);
        previous_count - self.entries.len()
    }

    pub fn clear_all(&mut self) -> usize {
        let count = self.entries.len();
        self.entries.clear();
        count
    }

    pub fn get_all_types(&self) -> Vec<String> {
        let mut types: Vec<_> = self
            .entries
            .values()
            .map(|entry| entry.type_name().to_owned())
            .collect();
        types.sort();
        types.dedup();
        types
    }
}
