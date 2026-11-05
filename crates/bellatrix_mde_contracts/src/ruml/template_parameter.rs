// TemplateParameter - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::parameterable_element::ParameterableElement;
use crate::ruml::template_signature::TemplateSignature;

const TYPE_NAME: &str = "TemplateParameter";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct TemplateParameter {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub signature: String,

    pub parametered_element: String,

    pub owned_parametered_element: Option<String>,

    pub default: Option<String>,

    pub owned_default: Option<String>,
}

impl TemplateParameter {
    /// Creates a new TemplateParameter instance

    pub fn create(
        registry: &mut Registry,
        signature: String,
        parametered_element: String,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            signature,
            parametered_element,
            owned_parametered_element: None,
            default: None,
            owned_default: None,
        };

        registry.insert(id.clone(), TYPE_NAME, &instance)?;

        Ok(id)
    }

    /// Gets a snapshot of this instance
    /// Note: Returns a snapshot. Modifications require calling update().

    pub fn get(registry: &Registry, id: String) -> Result<String, String> {
        registry.get_as_string(&id)
    }

    /// Updates the instance in the registry

    pub fn update(registry: &mut Registry, value: String) -> Result<(), String> {
        let instance: TemplateParameter =
            serde_json::from_str(&value).map_err(|_| String::from("Invalid data"))?;
        registry.update_from_string(
            instance.id.clone(),
            TYPE_NAME,
            serde_json::to_string(&instance).map_err(|_| String::from("Invalid data"))?,
        )
    }

    /// Deletes this instance from the registry

    pub fn delete(registry: &mut Registry, id: String) -> bool {
        registry.delete(&id)
    }

    /// Checks if an instance exists

    pub fn exists(registry: &Registry, id: String) -> bool {
        registry.exists(&id)
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

        let old_ref_id = instance.signature.clone();
        if let Some(mut old_target) = registry.get::<TemplateSignature>(&old_ref_id) {
            old_target.owned_parameter.retain(|id| id != &instance_id);
            registry.insert(old_ref_id.clone(), "TemplateSignature", &old_target)?;
        }

        instance.signature = ref_id.clone();

        if let Some(mut new_target) = registry.get::<TemplateSignature>(&ref_id) {
            if !new_target.owned_parameter.contains(&instance_id) {
                new_target.owned_parameter.push(instance_id.clone());
            }
            registry.insert(ref_id.clone(), "TemplateSignature", &new_target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn set_parametered_element(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let old_ref_id = instance.parametered_element.clone();
        if let Some(mut old_target) = registry.get::<ParameterableElement>(&old_ref_id) {
            old_target.template_parameter = None;
            registry.insert(old_ref_id.clone(), "ParameterableElement", &old_target)?;
        }

        instance.parametered_element = ref_id.clone();

        if let Some(mut new_target) = registry.get::<ParameterableElement>(&ref_id) {
            new_target.template_parameter = Some(instance_id.clone());
            registry.insert(ref_id.clone(), "ParameterableElement", &new_target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn set_owned_parametered_element(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if let Some(old_ref_id) = &instance.owned_parametered_element {
            if let Some(mut old_target) = registry.get::<ParameterableElement>(&old_ref_id) {
                old_target.owning_template_parameter = None;
                registry.insert(old_ref_id.clone(), "ParameterableElement", &old_target)?;
            }
        }

        instance.owned_parametered_element = ref_id.clone();

        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry.get::<ParameterableElement>(&new_ref_id) {
                new_target.owning_template_parameter = Some(instance_id.clone());
                registry.insert(new_ref_id.clone(), "ParameterableElement", &new_target)?;
            }
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_owned_parametered_element(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.owned_parametered_element.is_none() {
            return Ok(false);
        }

        if let Some(old_ref_id) = &instance.owned_parametered_element {
            if let Some(mut old_target) = registry.get::<ParameterableElement>(&old_ref_id) {
                old_target.owning_template_parameter = None;
                registry.insert(old_ref_id.clone(), "ParameterableElement", &old_target)?;
            }
        }

        instance.owned_parametered_element = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_default(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.default = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_default(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.default.is_none() {
            return Ok(false);
        }

        instance.default = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_owned_default(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.owned_default = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_owned_default(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.owned_default.is_none() {
            return Ok(false);
        }

        instance.owned_default = None;

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
        "TemplateParameter".to_string()
    }
}
