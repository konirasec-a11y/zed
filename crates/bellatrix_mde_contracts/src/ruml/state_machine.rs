// StateMachine - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::pseudostate::Pseudostate;
use crate::ruml::region::Region;
use crate::ruml::state::State;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "StateMachine";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct StateMachine {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub element_import: Vec<String>,

    pub package_import: Vec<String>,

    pub owned_rule: Vec<String>,

    pub is_leaf: String,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub template_binding: Vec<String>,

    pub owned_template_signature: Option<String>,

    pub is_abstract: String,

    pub generalization: Vec<String>,

    pub powertype_extent: Vec<String>,

    pub redefined_classifier: Vec<String>,

    pub substitution: Vec<String>,

    pub representation: Option<String>,

    pub collaboration_use: Vec<String>,

    pub owned_use_case: Vec<String>,

    pub use_case: Vec<String>,

    pub owned_attribute: Vec<String>,

    pub owned_connector: Vec<String>,

    pub owned_behavior: Vec<String>,

    pub classifier_behavior: Option<String>,

    pub interface_realization: Vec<String>,

    pub owned_trigger: Vec<String>,

    pub nested_classifier: Vec<String>,

    pub owned_operation: Vec<String>,

    pub is_active: String,

    pub owned_reception: Vec<String>,

    pub is_reentrant: String,

    pub redefined_behavior: Vec<String>,

    pub owned_parameter: Vec<String>,

    pub precondition: Vec<String>,

    pub postcondition: Vec<String>,

    pub owned_parameter_set: Vec<String>,

    pub specification: Option<String>,

    pub region: Vec<String>,

    pub submachine_state: Vec<String>,

    pub connection_point: Vec<String>,

    pub extended_state_machine: Vec<String>,
}

impl StateMachine {
    /// Creates a new StateMachine instance

    pub fn create(
        registry: &mut Registry,
        is_leaf: String,
        is_abstract: String,
        is_active: String,
        is_reentrant: String,
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
            element_import: Vec::new(),
            package_import: Vec::new(),
            owned_rule: Vec::new(),
            is_leaf,
            owning_template_parameter: None,
            template_parameter: None,
            template_binding: Vec::new(),
            owned_template_signature: None,
            is_abstract,
            generalization: Vec::new(),
            powertype_extent: Vec::new(),
            redefined_classifier: Vec::new(),
            substitution: Vec::new(),
            representation: None,
            collaboration_use: Vec::new(),
            owned_use_case: Vec::new(),
            use_case: Vec::new(),
            owned_attribute: Vec::new(),
            owned_connector: Vec::new(),
            owned_behavior: Vec::new(),
            classifier_behavior: None,
            interface_realization: Vec::new(),
            owned_trigger: Vec::new(),
            nested_classifier: Vec::new(),
            owned_operation: Vec::new(),
            is_active,
            owned_reception: Vec::new(),
            is_reentrant,
            redefined_behavior: Vec::new(),
            owned_parameter: Vec::new(),
            precondition: Vec::new(),
            postcondition: Vec::new(),
            owned_parameter_set: Vec::new(),
            specification: None,
            region: Vec::new(),
            submachine_state: Vec::new(),
            connection_point: Vec::new(),
            extended_state_machine: Vec::new(),
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
        let instance: StateMachine =
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

    pub fn set_is_abstract(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_abstract = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_active(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_active = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_reentrant(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_reentrant = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn add_region(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.region.contains(&ref_id) {
            return Ok(false);
        }

        instance.region.push(ref_id.clone());

        // Update opposite reference
        if let Some(mut target) = registry.get::<Region>(&ref_id) {
            target.state_machine = Some(instance_id.clone());
            registry.insert(ref_id.clone(), "Region", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_region(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.region.len();
        instance.region.retain(|id| id != &ref_id);
        let removed = instance.region.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        // Update opposite reference
        if let Some(mut target) = registry.get::<Region>(&ref_id) {
            target.state_machine = None;
            registry.insert(ref_id.clone(), "Region", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_region(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.region.len();

        if count == 0 {
            return Ok(0);
        }

        for ref_id in &instance.region {
            // Update opposite reference
            if let Some(mut target) = registry.get::<Region>(&ref_id) {
                target.state_machine = None;
                registry.insert(ref_id.clone(), "Region", &target)?;
            }
        }

        instance.region.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_submachine_state(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.submachine_state.contains(&ref_id) {
            return Ok(false);
        }

        instance.submachine_state.push(ref_id.clone());

        // Update opposite reference
        if let Some(mut target) = registry.get::<State>(&ref_id) {
            target.submachine = Some(instance_id.clone());
            registry.insert(ref_id.clone(), "State", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_submachine_state(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.submachine_state.len();
        instance.submachine_state.retain(|id| id != &ref_id);
        let removed = instance.submachine_state.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        // Update opposite reference
        if let Some(mut target) = registry.get::<State>(&ref_id) {
            target.submachine = None;
            registry.insert(ref_id.clone(), "State", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_submachine_state(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.submachine_state.len();

        if count == 0 {
            return Ok(0);
        }

        for ref_id in &instance.submachine_state {
            // Update opposite reference
            if let Some(mut target) = registry.get::<State>(&ref_id) {
                target.submachine = None;
                registry.insert(ref_id.clone(), "State", &target)?;
            }
        }

        instance.submachine_state.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_connection_point(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.connection_point.contains(&ref_id) {
            return Ok(false);
        }

        instance.connection_point.push(ref_id.clone());

        // Update opposite reference
        if let Some(mut target) = registry.get::<Pseudostate>(&ref_id) {
            target.state_machine = Some(instance_id.clone());
            registry.insert(ref_id.clone(), "Pseudostate", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_connection_point(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.connection_point.len();
        instance.connection_point.retain(|id| id != &ref_id);
        let removed = instance.connection_point.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        // Update opposite reference
        if let Some(mut target) = registry.get::<Pseudostate>(&ref_id) {
            target.state_machine = None;
            registry.insert(ref_id.clone(), "Pseudostate", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_connection_point(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.connection_point.len();

        if count == 0 {
            return Ok(0);
        }

        for ref_id in &instance.connection_point {
            // Update opposite reference
            if let Some(mut target) = registry.get::<Pseudostate>(&ref_id) {
                target.state_machine = None;
                registry.insert(ref_id.clone(), "Pseudostate", &target)?;
            }
        }

        instance.connection_point.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_extended_state_machine(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.extended_state_machine.contains(&ref_id) {
            return Ok(false);
        }

        instance.extended_state_machine.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_extended_state_machine(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.extended_state_machine.len();
        instance.extended_state_machine.retain(|id| id != &ref_id);
        let removed = instance.extended_state_machine.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_extended_state_machine(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.extended_state_machine.len();

        if count == 0 {
            return Ok(0);
        }

        instance.extended_state_machine.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
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
        "StateMachine".to_string()
    }
}
