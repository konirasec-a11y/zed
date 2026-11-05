// State - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::connection_point_reference::ConnectionPointReference;
use crate::ruml::pseudostate::Pseudostate;
use crate::ruml::region::Region;
use crate::ruml::state_machine::StateMachine;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "State";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct State {
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

    pub container: Option<String>,

    pub submachine: Option<String>,

    pub connection: Vec<String>,

    pub connection_point: Vec<String>,

    pub redefined_state: Option<String>,

    pub state_invariant: Option<String>,

    pub entry: Option<String>,

    pub exit: Option<String>,

    pub do_activity: Option<String>,

    pub deferrable_trigger: Vec<String>,

    pub region: Vec<String>,
}

impl State {
    /// Creates a new State instance

    pub fn create(registry: &mut Registry, is_leaf: String) -> Result<String, String> {
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
            container: None,
            submachine: None,
            connection: Vec::new(),
            connection_point: Vec::new(),
            redefined_state: None,
            state_invariant: None,
            entry: None,
            exit: None,
            do_activity: None,
            deferrable_trigger: Vec::new(),
            region: Vec::new(),
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
        let instance: State =
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

    pub fn set_submachine(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if let Some(old_ref_id) = &instance.submachine {
            if let Some(mut old_target) = registry.get::<StateMachine>(&old_ref_id) {
                old_target.submachine_state.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "StateMachine", &old_target)?;
            }
        }

        instance.submachine = ref_id.clone();

        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry.get::<StateMachine>(&new_ref_id) {
                if !new_target.submachine_state.contains(&instance_id) {
                    new_target.submachine_state.push(instance_id.clone());
                }
                registry.insert(new_ref_id.clone(), "StateMachine", &new_target)?;
            }
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_submachine(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.submachine.is_none() {
            return Ok(false);
        }

        if let Some(old_ref_id) = &instance.submachine {
            if let Some(mut old_target) = registry.get::<StateMachine>(&old_ref_id) {
                old_target.submachine_state.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "StateMachine", &old_target)?;
            }
        }

        instance.submachine = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn add_connection(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.connection.contains(&ref_id) {
            return Ok(false);
        }

        instance.connection.push(ref_id.clone());

        // Update opposite reference
        if let Some(mut target) = registry.get::<ConnectionPointReference>(&ref_id) {
            target.state = Some(instance_id.clone());
            registry.insert(ref_id.clone(), "ConnectionPointReference", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_connection(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.connection.len();
        instance.connection.retain(|id| id != &ref_id);
        let removed = instance.connection.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        // Update opposite reference
        if let Some(mut target) = registry.get::<ConnectionPointReference>(&ref_id) {
            target.state = None;
            registry.insert(ref_id.clone(), "ConnectionPointReference", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_connection(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.connection.len();

        if count == 0 {
            return Ok(0);
        }

        for ref_id in &instance.connection {
            // Update opposite reference
            if let Some(mut target) = registry.get::<ConnectionPointReference>(&ref_id) {
                target.state = None;
                registry.insert(ref_id.clone(), "ConnectionPointReference", &target)?;
            }
        }

        instance.connection.clear();

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
            target.state = Some(instance_id.clone());
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
            target.state = None;
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
                target.state = None;
                registry.insert(ref_id.clone(), "Pseudostate", &target)?;
            }
        }

        instance.connection_point.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn set_redefined_state(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.redefined_state = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_redefined_state(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.redefined_state.is_none() {
            return Ok(false);
        }

        instance.redefined_state = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_state_invariant(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.state_invariant = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_state_invariant(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.state_invariant.is_none() {
            return Ok(false);
        }

        instance.state_invariant = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_entry(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.entry = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_entry(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.entry.is_none() {
            return Ok(false);
        }

        instance.entry = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_exit(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.exit = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_exit(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.exit.is_none() {
            return Ok(false);
        }

        instance.exit = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_do_activity(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.do_activity = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_do_activity(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.do_activity.is_none() {
            return Ok(false);
        }

        instance.do_activity = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn add_deferrable_trigger(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.deferrable_trigger.contains(&ref_id) {
            return Ok(false);
        }

        instance.deferrable_trigger.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_deferrable_trigger(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.deferrable_trigger.len();
        instance.deferrable_trigger.retain(|id| id != &ref_id);
        let removed = instance.deferrable_trigger.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_deferrable_trigger(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.deferrable_trigger.len();

        if count == 0 {
            return Ok(0);
        }

        instance.deferrable_trigger.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
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
            target.state = Some(instance_id.clone());
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
            target.state = None;
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
                target.state = None;
                registry.insert(ref_id.clone(), "Region", &target)?;
            }
        }

        instance.region.clear();

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
        "State".to_string()
    }
}
