// LoopNode - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "LoopNode";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct LoopNode {
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

    pub handler: Vec<String>,

    pub local_precondition: Vec<String>,

    pub local_postcondition: Vec<String>,

    pub element_import: Vec<String>,

    pub package_import: Vec<String>,

    pub owned_rule: Vec<String>,

    pub in_activity: Option<String>,

    pub variable: Vec<String>,

    pub edge: Vec<String>,

    pub must_isolate: String,

    pub node: Vec<String>,

    pub is_tested_first: String,

    pub body_part: Vec<String>,

    pub setup_part: Vec<String>,

    pub decider: String,

    pub test: Vec<String>,

    pub result: Vec<String>,

    pub loop_variable: Vec<String>,

    pub body_output: Vec<String>,

    pub loop_variable_input: Vec<String>,
}

impl LoopNode {
    /// Creates a new LoopNode instance

    pub fn create(
        registry: &mut Registry,
        is_leaf: String,
        must_isolate: String,
        is_tested_first: String,
        decider: String,
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
            handler: Vec::new(),
            local_precondition: Vec::new(),
            local_postcondition: Vec::new(),
            element_import: Vec::new(),
            package_import: Vec::new(),
            owned_rule: Vec::new(),
            in_activity: None,
            variable: Vec::new(),
            edge: Vec::new(),
            must_isolate,
            node: Vec::new(),
            is_tested_first,
            body_part: Vec::new(),
            setup_part: Vec::new(),
            decider,
            test: Vec::new(),
            result: Vec::new(),
            loop_variable: Vec::new(),
            body_output: Vec::new(),
            loop_variable_input: Vec::new(),
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
        let instance: LoopNode =
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

    pub fn set_must_isolate(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.must_isolate = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_tested_first(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_tested_first = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn add_body_part(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.body_part.contains(&ref_id) {
            return Ok(false);
        }

        instance.body_part.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_body_part(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.body_part.len();
        instance.body_part.retain(|id| id != &ref_id);
        let removed = instance.body_part.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_body_part(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.body_part.len();

        if count == 0 {
            return Ok(0);
        }

        instance.body_part.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_setup_part(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.setup_part.contains(&ref_id) {
            return Ok(false);
        }

        instance.setup_part.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_setup_part(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.setup_part.len();
        instance.setup_part.retain(|id| id != &ref_id);
        let removed = instance.setup_part.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_setup_part(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.setup_part.len();

        if count == 0 {
            return Ok(0);
        }

        instance.setup_part.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn set_decider(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.decider = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn add_test(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.test.contains(&ref_id) {
            return Ok(false);
        }

        instance.test.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_test(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.test.len();
        instance.test.retain(|id| id != &ref_id);
        let removed = instance.test.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_test(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.test.len();

        if count == 0 {
            return Ok(0);
        }

        instance.test.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_result(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.result.contains(&ref_id) {
            return Ok(false);
        }

        instance.result.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_result(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.result.len();
        instance.result.retain(|id| id != &ref_id);
        let removed = instance.result.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_result(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.result.len();

        if count == 0 {
            return Ok(0);
        }

        instance.result.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_loop_variable(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.loop_variable.contains(&ref_id) {
            return Ok(false);
        }

        instance.loop_variable.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_loop_variable(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.loop_variable.len();
        instance.loop_variable.retain(|id| id != &ref_id);
        let removed = instance.loop_variable.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_loop_variable(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.loop_variable.len();

        if count == 0 {
            return Ok(0);
        }

        instance.loop_variable.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_body_output(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.body_output.contains(&ref_id) {
            return Ok(false);
        }

        instance.body_output.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_body_output(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.body_output.len();
        instance.body_output.retain(|id| id != &ref_id);
        let removed = instance.body_output.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_body_output(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.body_output.len();

        if count == 0 {
            return Ok(0);
        }

        instance.body_output.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_loop_variable_input(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.loop_variable_input.contains(&ref_id) {
            return Ok(false);
        }

        instance.loop_variable_input.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_loop_variable_input(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.loop_variable_input.len();
        instance.loop_variable_input.retain(|id| id != &ref_id);
        let removed = instance.loop_variable_input.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_loop_variable_input(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.loop_variable_input.len();

        if count == 0 {
            return Ok(0);
        }

        instance.loop_variable_input.clear();

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
        "LoopNode".to_string()
    }
}
