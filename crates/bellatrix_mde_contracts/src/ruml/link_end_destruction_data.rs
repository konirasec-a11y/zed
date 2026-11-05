// LinkEndDestructionData - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

const TYPE_NAME: &str = "LinkEndDestructionData";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct LinkEndDestructionData {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub value: Option<String>,

    pub end: String,

    pub qualifier: Vec<String>,

    pub is_destroy_duplicates: String,

    pub destroy_at: Option<String>,
}

impl LinkEndDestructionData {
    /// Creates a new LinkEndDestructionData instance

    pub fn create(
        registry: &mut Registry,
        end: String,
        is_destroy_duplicates: String,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            value: None,
            end,
            qualifier: Vec::new(),
            is_destroy_duplicates,
            destroy_at: None,
        };

        registry.insert(id.clone(), TYPE_NAME, &instance)?;

        Ok(id)
    }

    /// Gets a snapshot of this instance
    /// Note: Returns a snapshot. Modifications require calling update().

    pub fn get(registry: &Registry, id: String) -> Result<String, String> {
        registry.get_as_string_of_type(&id, TYPE_NAME)
    }

    /// Updates the instance in the registry

    pub fn update(registry: &mut Registry, value: String) -> Result<(), String> {
        let instance: LinkEndDestructionData =
            serde_json::from_str(&value).map_err(|_| String::from("Invalid data"))?;
        registry.update_from_string(
            instance.id.clone(),
            TYPE_NAME,
            serde_json::to_string(&instance).map_err(|_| String::from("Invalid data"))?,
        )
    }

    /// Deletes this instance from the registry

    pub fn delete(registry: &mut Registry, id: String) -> bool {
        registry.delete_of_type(&id, TYPE_NAME)
    }

    /// Checks if an instance exists

    pub fn exists(registry: &Registry, id: String) -> bool {
        registry.exists_of_type(&id, TYPE_NAME)
    }

    /// Gets all instances of this type

    pub fn get_all(registry: &Registry) -> Result<String, String> {
        registry.get_all_of_type_as_string(TYPE_NAME)
    }

    /// Clears all instances of this type

    pub fn clear_all(registry: &mut Registry) -> usize {
        registry.clear_type(TYPE_NAME)
    }

    pub fn set_is_destroy_duplicates(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_destroy_duplicates = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_destroy_at(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.destroy_at = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_destroy_at(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.destroy_at.is_none() {
            return Ok(false);
        }

        instance.destroy_at = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    /// Returns whether this type can be created standalone (not nested)
    pub fn can_exist_standalone() -> bool {
        false
    }

    /// Returns whether this type requires a container
    pub fn requires_container() -> bool {
        true
    }

    /// Returns the type name
    pub fn type_name() -> String {
        "LinkEndDestructionData".to_string()
    }
}
