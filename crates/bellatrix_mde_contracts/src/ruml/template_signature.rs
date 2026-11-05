// TemplateSignature - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::template_parameter::TemplateParameter;
use crate::ruml::templateable_element::TemplateableElement;

const TYPE_NAME: &str = "TemplateSignature";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct TemplateSignature {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub parameter: Vec<String>,

    pub owned_parameter: Vec<String>,

    pub template: String,
}

impl TemplateSignature {
    /// Creates a new TemplateSignature instance

    pub fn create(registry: &mut Registry, template: String) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            parameter: Vec::new(),
            owned_parameter: Vec::new(),
            template,
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
        let instance: TemplateSignature =
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

    pub fn add_parameter(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.parameter.contains(&ref_id) {
            return Ok(false);
        }

        instance.parameter.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_parameter(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.parameter.len();
        instance.parameter.retain(|id| id != &ref_id);
        let removed = instance.parameter.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_parameter(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.parameter.len();

        if count == 0 {
            return Ok(0);
        }

        instance.parameter.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_owned_parameter(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.owned_parameter.contains(&ref_id) {
                return Ok(false);
            }

            instance.owned_parameter.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<TemplateParameter>(&ref_id) {
                target.signature = instance_id.clone();
                registry.insert(ref_id.clone(), "TemplateParameter", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_owned_parameter(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.owned_parameter.len();
            instance.owned_parameter.retain(|id| id != &ref_id);
            let removed = instance.owned_parameter.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<TemplateParameter>(&ref_id) {
                registry.insert(ref_id.clone(), "TemplateParameter", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_owned_parameter(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.owned_parameter.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.owned_parameter {
                // Update opposite reference
                if let Some(target) = registry.get::<TemplateParameter>(&ref_id) {
                    registry.insert(ref_id.clone(), "TemplateParameter", &target)?;
                }
            }

            instance.owned_parameter.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn set_template(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let old_ref_id = instance.template.clone();
            if let Some(mut old_target) = registry.get::<TemplateableElement>(&old_ref_id) {
                old_target.owned_template_signature = None;
                registry.insert(old_ref_id.clone(), "TemplateableElement", &old_target)?;
            }

            instance.template = ref_id.clone();

            if let Some(mut new_target) = registry.get::<TemplateableElement>(&ref_id) {
                new_target.owned_template_signature = Some(instance_id.clone());
                registry.insert(ref_id.clone(), "TemplateableElement", &new_target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(())
        })
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
        "TemplateSignature".to_string()
    }
}
