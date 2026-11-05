// EParameter - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

const TYPE_NAME: &str = "EParameter";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct EParameter {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub name: Option<String>,

    pub ordered: Option<bool>,

    pub unique: Option<bool>,

    pub lower_bound: Option<i32>,

    pub upper_bound: Option<i32>,

    pub e_type: Option<String>,

    pub e_generic_type: Option<String>,
}

impl EParameter {
    /// Creates a new EParameter instance

    pub fn create(registry: &mut Registry) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            name: None,
            ordered: None,
            unique: None,
            lower_bound: None,
            upper_bound: None,
            e_type: None,
            e_generic_type: None,
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
        let instance: EParameter =
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

    pub fn set_name(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.name = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_name(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.name = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_ordered(
        registry: &mut Registry,
        instance_id: String,
        value: bool,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.ordered = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_ordered(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.ordered = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_unique(
        registry: &mut Registry,
        instance_id: String,
        value: bool,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.unique = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_unique(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.unique = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_lower_bound(
        registry: &mut Registry,
        instance_id: String,
        value: i32,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.lower_bound = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_lower_bound(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.lower_bound = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_upper_bound(
        registry: &mut Registry,
        instance_id: String,
        value: i32,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.upper_bound = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_upper_bound(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.upper_bound = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    /// Returns whether this type can be created standalone (not nested)
    pub fn can_exist_standalone() -> bool {
        true
    }

    /// Returns whether this type requires a container
    pub fn requires_container() -> bool {
        false
    }

    /// Returns the type name
    pub fn type_name() -> String {
        "EParameter".to_string()
    }
}
