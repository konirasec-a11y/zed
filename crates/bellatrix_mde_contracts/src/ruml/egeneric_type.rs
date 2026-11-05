// EGenericType - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

const TYPE_NAME: &str = "EGenericType";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct EGenericType {
    /// Unique identifier for this instance
    pub id: String,

    pub e_upper_bound: Option<String>,

    pub e_type_arguments: Vec<String>,

    pub e_lower_bound: Option<String>,

    pub e_type_parameter: Option<String>,

    pub e_classifier: Option<String>,
}

impl EGenericType {
    /// Creates a new EGenericType instance

    pub fn create(registry: &mut Registry) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_upper_bound: None,
            e_type_arguments: Vec::new(),
            e_lower_bound: None,
            e_type_parameter: None,
            e_classifier: None,
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
        let instance: EGenericType =
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

    pub fn set_e_upper_bound(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.e_upper_bound = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_e_upper_bound(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.e_upper_bound.is_none() {
            return Ok(false);
        }

        instance.e_upper_bound = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn add_e_type_arguments(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.e_type_arguments.contains(&ref_id) {
            return Ok(false);
        }

        instance.e_type_arguments.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_e_type_arguments(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.e_type_arguments.len();
        instance.e_type_arguments.retain(|id| id != &ref_id);
        let removed = instance.e_type_arguments.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_e_type_arguments(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.e_type_arguments.len();

        if count == 0 {
            return Ok(0);
        }

        instance.e_type_arguments.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn set_e_lower_bound(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.e_lower_bound = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_e_lower_bound(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.e_lower_bound.is_none() {
            return Ok(false);
        }

        instance.e_lower_bound = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_e_type_parameter(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.e_type_parameter = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_e_type_parameter(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.e_type_parameter.is_none() {
            return Ok(false);
        }

        instance.e_type_parameter = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_e_classifier(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.e_classifier = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_e_classifier(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.e_classifier.is_none() {
            return Ok(false);
        }

        instance.e_classifier = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
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
        "EGenericType".to_string()
    }
}
