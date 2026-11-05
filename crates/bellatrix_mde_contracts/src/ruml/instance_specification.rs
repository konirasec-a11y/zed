// InstanceSpecification - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::slot::Slot;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "InstanceSpecification";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct InstanceSpecification {
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
}

impl InstanceSpecification {
    /// Creates a new InstanceSpecification instance

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
        let instance: InstanceSpecification =
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

    pub fn add_classifier(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.classifier.contains(&ref_id) {
            return Ok(false);
        }

        instance.classifier.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_classifier(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.classifier.len();
        instance.classifier.retain(|id| id != &ref_id);
        let removed = instance.classifier.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_classifier(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.classifier.len();

        if count == 0 {
            return Ok(0);
        }

        instance.classifier.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_slot(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.slot.contains(&ref_id) {
                return Ok(false);
            }

            instance.slot.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Slot>(&ref_id) {
                target.owning_instance = instance_id.clone();
                registry.insert(ref_id.clone(), "Slot", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_slot(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.slot.len();
            instance.slot.retain(|id| id != &ref_id);
            let removed = instance.slot.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<Slot>(&ref_id) {
                registry.insert(ref_id.clone(), "Slot", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_slot(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.slot.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.slot {
                // Update opposite reference
                if let Some(target) = registry.get::<Slot>(&ref_id) {
                    registry.insert(ref_id.clone(), "Slot", &target)?;
                }
            }

            instance.slot.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn set_specification(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.specification = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_specification(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.specification.is_none() {
            return Ok(false);
        }

        instance.specification = None;

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
        "InstanceSpecification".to_string()
    }
}
