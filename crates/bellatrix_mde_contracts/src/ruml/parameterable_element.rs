// ParameterableElement - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::template_parameter::TemplateParameter;

const TYPE_NAME: &str = "ParameterableElement";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ParameterableElement {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,
}

impl ParameterableElement {
    /// Creates a new ParameterableElement instance

    pub fn create(registry: &mut Registry) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            owning_template_parameter: None,
            template_parameter: None,
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
        let instance: ParameterableElement =
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

    pub fn set_owning_template_parameter(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if let Some(old_ref_id) = &instance.owning_template_parameter {
                if let Some(mut old_target) = registry.get::<TemplateParameter>(&old_ref_id) {
                    old_target.owned_parametered_element = None;
                    registry.insert(old_ref_id.clone(), "TemplateParameter", &old_target)?;
                }
            }

            instance.owning_template_parameter = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<TemplateParameter>(&new_ref_id) {
                    new_target.owned_parametered_element = Some(instance_id.clone());
                    registry.insert(new_ref_id.clone(), "TemplateParameter", &new_target)?;
                }
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(())
        })
    }

    pub fn clear_owning_template_parameter(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.owning_template_parameter.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.owning_template_parameter {
                if let Some(mut old_target) = registry.get::<TemplateParameter>(&old_ref_id) {
                    old_target.owned_parametered_element = None;
                    registry.insert(old_ref_id.clone(), "TemplateParameter", &old_target)?;
                }
            }

            instance.owning_template_parameter = None;

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn set_template_parameter(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if let Some(old_ref_id) = &instance.template_parameter {
                if let Some(old_target) = registry.get::<TemplateParameter>(&old_ref_id) {
                    registry.insert(old_ref_id.clone(), "TemplateParameter", &old_target)?;
                }
            }

            instance.template_parameter = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<TemplateParameter>(&new_ref_id) {
                    new_target.parametered_element = instance_id.clone();
                    registry.insert(new_ref_id.clone(), "TemplateParameter", &new_target)?;
                }
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(())
        })
    }

    pub fn clear_template_parameter(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.template_parameter.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.template_parameter {
                if let Some(old_target) = registry.get::<TemplateParameter>(&old_ref_id) {
                    registry.insert(old_ref_id.clone(), "TemplateParameter", &old_target)?;
                }
            }

            instance.template_parameter = None;

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
        "ParameterableElement".to_string()
    }
}
