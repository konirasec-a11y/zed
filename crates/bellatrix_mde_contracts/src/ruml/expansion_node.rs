// ExpansionNode - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::expansion_region::ExpansionRegion;
use crate::ruml::object_node_ordering_kind::ObjectNodeOrderingKind;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "ExpansionNode";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ExpansionNode {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub is_leaf: String,

    pub in_structured_node: Option<String>,

    pub activity: Option<String>,

    pub outgoing: Vec<String>,

    pub incoming: Vec<String>,

    pub in_partition: Vec<String>,

    pub in_interruptible_region: Vec<String>,

    pub redefined_node: Vec<String>,

    pub type_: Option<String>,

    pub ordering: ObjectNodeOrderingKind,

    pub is_control_type: String,

    pub upper_bound: String,

    pub in_state: Vec<String>,

    pub selection: Option<String>,

    pub region_as_output: Option<String>,

    pub region_as_input: Option<String>,
}

impl ExpansionNode {
    /// Creates a new ExpansionNode instance

    pub fn create(
        registry: &mut Registry,
        is_leaf: String,
        ordering: ObjectNodeOrderingKind,
        is_control_type: String,
        upper_bound: String,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            is_leaf,
            in_structured_node: None,
            activity: None,
            outgoing: Vec::new(),
            incoming: Vec::new(),
            in_partition: Vec::new(),
            in_interruptible_region: Vec::new(),
            redefined_node: Vec::new(),
            type_: None,
            ordering,
            is_control_type,
            upper_bound,
            in_state: Vec::new(),
            selection: None,
            region_as_output: None,
            region_as_input: None,
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
        let instance: ExpansionNode =
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

    pub fn set_is_leaf(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_leaf = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_ordering(
        registry: &mut Registry,
        instance_id: String,
        value: ObjectNodeOrderingKind,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.ordering = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_control_type(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_control_type = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_region_as_output(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if let Some(old_ref_id) = &instance.region_as_output {
            if let Some(mut old_target) = registry.get::<ExpansionRegion>(&old_ref_id) {
                old_target.output_element.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "ExpansionRegion", &old_target)?;
            }
        }

        instance.region_as_output = ref_id.clone();

        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry.get::<ExpansionRegion>(&new_ref_id) {
                if !new_target.output_element.contains(&instance_id) {
                    new_target.output_element.push(instance_id.clone());
                }
                registry.insert(new_ref_id.clone(), "ExpansionRegion", &new_target)?;
            }
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_region_as_output(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.region_as_output.is_none() {
            return Ok(false);
        }

        if let Some(old_ref_id) = &instance.region_as_output {
            if let Some(mut old_target) = registry.get::<ExpansionRegion>(&old_ref_id) {
                old_target.output_element.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "ExpansionRegion", &old_target)?;
            }
        }

        instance.region_as_output = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_region_as_input(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if let Some(old_ref_id) = &instance.region_as_input {
            if let Some(mut old_target) = registry.get::<ExpansionRegion>(&old_ref_id) {
                old_target.input_element.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "ExpansionRegion", &old_target)?;
            }
        }

        instance.region_as_input = ref_id.clone();

        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry.get::<ExpansionRegion>(&new_ref_id) {
                if !new_target.input_element.contains(&instance_id) {
                    new_target.input_element.push(instance_id.clone());
                }
                registry.insert(new_ref_id.clone(), "ExpansionRegion", &new_target)?;
            }
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_region_as_input(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.region_as_input.is_none() {
            return Ok(false);
        }

        if let Some(old_ref_id) = &instance.region_as_input {
            if let Some(mut old_target) = registry.get::<ExpansionRegion>(&old_ref_id) {
                old_target.input_element.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "ExpansionRegion", &old_target)?;
            }
        }

        instance.region_as_input = None;

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
        "ExpansionNode".to_string()
    }
}
