// EnumerationLiteral - Generated UML Class
// Uses unified registry with type filtering

use crate::contracts::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::contracts::ruml::enumeration::Enumeration;
use crate::contracts::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "EnumerationLiteral";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct EnumerationLiteral {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub deployment: Vec<String>,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub classifier: Vec<String>,

    pub slot: Vec<String>,

    pub specification: Option<String>,

    pub enumeration: Option<String>,
}

impl EnumerationLiteral {
    /// Creates a new EnumerationLiteral instance

    pub fn create(registry: &mut Registry) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            deployment: Vec::new(),
            owning_template_parameter: None,
            template_parameter: None,
            classifier: Vec::new(),
            slot: Vec::new(),
            specification: None,
            enumeration: None,
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
        let instance: EnumerationLiteral =
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

    pub fn set_enumeration(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if let Some(old_ref_id) = &instance.enumeration {
                if let Some(mut old_target) = registry.get::<Enumeration>(&old_ref_id) {
                    old_target.owned_literal.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "Enumeration", &old_target)?;
                }
            }

            instance.enumeration = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<Enumeration>(&new_ref_id) {
                    if !new_target.owned_literal.contains(&instance_id) {
                        new_target.owned_literal.push(instance_id.clone());
                    }
                    registry.insert(new_ref_id.clone(), "Enumeration", &new_target)?;
                }
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(())
        })
    }

    pub fn clear_enumeration(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.enumeration.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.enumeration {
                if let Some(mut old_target) = registry.get::<Enumeration>(&old_ref_id) {
                    old_target.owned_literal.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "Enumeration", &old_target)?;
                }
            }

            instance.enumeration = None;

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

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
        "EnumerationLiteral".to_string()
    }
}
