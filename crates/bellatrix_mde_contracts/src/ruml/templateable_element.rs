// TemplateableElement - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::template_binding::TemplateBinding;
use crate::ruml::template_signature::TemplateSignature;

const TYPE_NAME: &str = "TemplateableElement";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct TemplateableElement {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub template_binding: Vec<String>,

    pub owned_template_signature: Option<String>,
}

impl TemplateableElement {
    /// Creates a new TemplateableElement instance

    pub fn create(registry: &mut Registry) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            template_binding: Vec::new(),
            owned_template_signature: None,
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
        let instance: TemplateableElement =
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

    pub fn add_template_binding(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.template_binding.contains(&ref_id) {
                return Ok(false);
            }

            instance.template_binding.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<TemplateBinding>(&ref_id) {
                target.bound_element = instance_id.clone();
                registry.insert(ref_id.clone(), "TemplateBinding", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_template_binding(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.template_binding.len();
            instance.template_binding.retain(|id| id != &ref_id);
            let removed = instance.template_binding.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<TemplateBinding>(&ref_id) {
                registry.insert(ref_id.clone(), "TemplateBinding", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_template_binding(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.template_binding.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.template_binding {
                // Update opposite reference
                if let Some(target) = registry.get::<TemplateBinding>(&ref_id) {
                    registry.insert(ref_id.clone(), "TemplateBinding", &target)?;
                }
            }

            instance.template_binding.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn set_owned_template_signature(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if let Some(old_ref_id) = &instance.owned_template_signature {
                if let Some(old_target) = registry.get::<TemplateSignature>(&old_ref_id) {
                    registry.insert(old_ref_id.clone(), "TemplateSignature", &old_target)?;
                }
            }

            instance.owned_template_signature = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<TemplateSignature>(&new_ref_id) {
                    new_target.template = instance_id.clone();
                    registry.insert(new_ref_id.clone(), "TemplateSignature", &new_target)?;
                }
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(())
        })
    }

    pub fn clear_owned_template_signature(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.owned_template_signature.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.owned_template_signature {
                if let Some(old_target) = registry.get::<TemplateSignature>(&old_ref_id) {
                    registry.insert(old_ref_id.clone(), "TemplateSignature", &old_target)?;
                }
            }

            instance.owned_template_signature = None;

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
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
        "TemplateableElement".to_string()
    }
}
