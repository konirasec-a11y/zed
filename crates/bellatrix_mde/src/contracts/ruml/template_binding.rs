// TemplateBinding - Generated UML Class
// Uses unified registry with type filtering

use crate::contracts::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::contracts::ruml::template_parameter_substitution::TemplateParameterSubstitution;
use crate::contracts::ruml::templateable_element::TemplateableElement;

const TYPE_NAME: &str = "TemplateBinding";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct TemplateBinding {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub signature: String,

    pub parameter_substitution: Vec<String>,

    pub bound_element: String,
}

impl TemplateBinding {
    /// Creates a new TemplateBinding instance

    pub fn create(
        registry: &mut Registry,
        signature: String,
        bound_element: String,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            signature,
            parameter_substitution: Vec::new(),
            bound_element,
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
        let instance: TemplateBinding =
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

    pub fn set_signature(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.signature = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn add_parameter_substitution(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.parameter_substitution.contains(&ref_id) {
                return Ok(false);
            }

            instance.parameter_substitution.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<TemplateParameterSubstitution>(&ref_id) {
                target.template_binding = instance_id.clone();
                registry.insert(ref_id.clone(), "TemplateParameterSubstitution", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_parameter_substitution(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let initial_len = instance.parameter_substitution.len();
            instance.parameter_substitution.retain(|id| id != &ref_id);
            let removed = instance.parameter_substitution.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<TemplateParameterSubstitution>(&ref_id) {
                registry.insert(ref_id.clone(), "TemplateParameterSubstitution", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_parameter_substitution(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let count = instance.parameter_substitution.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.parameter_substitution {
                // Update opposite reference
                if let Some(target) = registry.get::<TemplateParameterSubstitution>(&ref_id) {
                    registry.insert(ref_id.clone(), "TemplateParameterSubstitution", &target)?;
                }
            }

            instance.parameter_substitution.clear();

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(count)
        })
    }

    pub fn set_bound_element(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let old_ref_id = instance.bound_element.clone();
            if let Some(mut old_target) = registry.get::<TemplateableElement>(&old_ref_id) {
                old_target.template_binding.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "TemplateableElement", &old_target)?;
            }

            instance.bound_element = ref_id.clone();

            if let Some(mut new_target) = registry.get::<TemplateableElement>(&ref_id) {
                if !new_target.template_binding.contains(&instance_id) {
                    new_target.template_binding.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "TemplateableElement", &new_target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

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
        "TemplateBinding".to_string()
    }
}
