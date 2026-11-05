// InteractionConstraint - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "InteractionConstraint";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct InteractionConstraint {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub constrained_element: Vec<String>,

    pub specification: String,

    pub context: Option<String>,

    pub minint: Option<String>,

    pub maxint: Option<String>,
}

impl InteractionConstraint {
    /// Creates a new InteractionConstraint instance

    pub fn create(registry: &mut Registry, specification: String) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            owning_template_parameter: None,
            template_parameter: None,
            constrained_element: Vec::new(),
            specification,
            context: None,
            minint: None,
            maxint: None,
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
        let instance: InteractionConstraint =
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

    pub fn set_visibility(
        registry: &mut Registry,
        instance_id: String,
        value: VisibilityKind,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.visibility = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_visibility(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.visibility = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_minint(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.minint = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_minint(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.minint.is_none() {
            return Ok(false);
        }

        instance.minint = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_maxint(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.maxint = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_maxint(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.maxint.is_none() {
            return Ok(false);
        }

        instance.maxint = None;

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
        "InteractionConstraint".to_string()
    }
}
