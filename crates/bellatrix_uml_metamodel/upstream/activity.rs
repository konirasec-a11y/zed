// Activity - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::variable::Variable;
use crate::activity_node::ActivityNode;
use crate::activity_edge::ActivityEdge;
use crate::activity_group::ActivityGroup;


const TYPE_NAME: &str = "Activity";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct Activity {
    /// Unique identifier for this instance
    #[wasm_bindgen(skip)]
    pub id: String,
    #[wasm_bindgen(skip)]
    pub e_annotations: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_comment: Vec<String>,
    #[wasm_bindgen(skip)]
    pub name: Option<String>,
    #[wasm_bindgen(skip)]
    pub visibility: Option<VisibilityKind>,
    #[wasm_bindgen(skip)]
    pub client_dependency: Vec<String>,
    #[wasm_bindgen(skip)]
    pub name_expression: Option<String>,
    #[wasm_bindgen(skip)]
    pub element_import: Vec<String>,
    #[wasm_bindgen(skip)]
    pub package_import: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_rule: Vec<String>,
    #[wasm_bindgen(skip)]
    pub is_leaf: String,
    #[wasm_bindgen(skip)]
    pub owning_template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_binding: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_template_signature: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_abstract: String,
    #[wasm_bindgen(skip)]
    pub generalization: Vec<String>,
    #[wasm_bindgen(skip)]
    pub powertype_extent: Vec<String>,
    #[wasm_bindgen(skip)]
    pub redefined_classifier: Vec<String>,
    #[wasm_bindgen(skip)]
    pub substitution: Vec<String>,
    #[wasm_bindgen(skip)]
    pub representation: Option<String>,
    #[wasm_bindgen(skip)]
    pub collaboration_use: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_use_case: Vec<String>,
    #[wasm_bindgen(skip)]
    pub use_case: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_attribute: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_connector: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_behavior: Vec<String>,
    #[wasm_bindgen(skip)]
    pub classifier_behavior: Option<String>,
    #[wasm_bindgen(skip)]
    pub interface_realization: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_trigger: Vec<String>,
    #[wasm_bindgen(skip)]
    pub nested_classifier: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_operation: Vec<String>,
    #[wasm_bindgen(skip)]
    pub is_active: String,
    #[wasm_bindgen(skip)]
    pub owned_reception: Vec<String>,
    #[wasm_bindgen(skip)]
    pub is_reentrant: String,
    #[wasm_bindgen(skip)]
    pub redefined_behavior: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_parameter: Vec<String>,
    #[wasm_bindgen(skip)]
    pub precondition: Vec<String>,
    #[wasm_bindgen(skip)]
    pub postcondition: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_parameter_set: Vec<String>,
    #[wasm_bindgen(skip)]
    pub specification: Option<String>,
    #[wasm_bindgen(skip)]
    pub variable: Vec<String>,
    #[wasm_bindgen(skip)]
    pub node: Vec<String>,
    #[wasm_bindgen(skip)]
    pub is_read_only: String,
    #[wasm_bindgen(skip)]
    pub edge: Vec<String>,
    #[wasm_bindgen(skip)]
    pub partition: Vec<String>,
    #[wasm_bindgen(skip)]
    pub is_single_execution: String,
    #[wasm_bindgen(skip)]
    pub group: Vec<String>,
}

#[wasm_bindgen]
impl Activity {
    /// Creates a new Activity instance
    #[wasm_bindgen]
    pub fn create(is_leaf: String, is_abstract: String, is_active: String, is_reentrant: String, is_read_only: String, is_single_execution: String) -> String {
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
            variable: Vec::new(),
            node: Vec::new(),
            is_read_only,
            edge: Vec::new(),
            partition: Vec::new(),
            is_single_execution,
            group: Vec::new(),
        };
        
        registry::insert(id.clone(), TYPE_NAME, &instance)
            .expect("Failed to insert into registry");
        
        id
    }

    /// Gets a snapshot of this instance
    /// Note: Returns a snapshot. Modifications require calling update().
    #[wasm_bindgen]
    pub fn get(id: String) -> Result<JsValue, JsValue> {
        registry::get_as_jsvalue(&id)
    }

    /// Updates the instance in the registry
    #[wasm_bindgen]
    pub fn update(value: JsValue) -> Result<(), JsValue> {
        let instance: Activity = serde_wasm_bindgen::from_value(value)
            .map_err(|_| JsValue::from_str("Invalid data"))?;
        registry::update_from_jsvalue(
            instance.id.clone(),
            TYPE_NAME,
            serde_wasm_bindgen::to_value(&instance)?
        )
    }

    /// Deletes this instance from the registry
    #[wasm_bindgen]
    pub fn delete(id: String) -> bool {
        registry::delete(&id)
    }

    /// Checks if an instance exists
    #[wasm_bindgen]
    pub fn exists(id: String) -> bool {
        registry::exists(&id)
    }

    /// Gets all instances of this type
    #[wasm_bindgen]
    pub fn get_all() -> Result<JsValue, JsValue> {
        registry::get_all_of_type_as_jsvalue(TYPE_NAME)
    }

    /// Clears all instances of this type
    #[wasm_bindgen]
    pub fn clear_all() -> usize {
        registry::clear_type(TYPE_NAME)
    }

#[wasm_bindgen]
pub fn set_name(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.name = Some(value);
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn clear_name(instance_id: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.name = None;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_visibility(instance_id: String, value: VisibilityKind) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.visibility = Some(value);
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn clear_visibility(instance_id: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.visibility = None;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_leaf(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_leaf = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_abstract(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_abstract = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_active(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_active = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_reentrant(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_reentrant = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_read_only(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_read_only = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_single_execution(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_single_execution = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}





    #[wasm_bindgen]
    pub fn add_variable(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.variable.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.variable.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<Variable>(&ref_id) {
            target.activity_scope = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "Variable", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_variable(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.variable.len();
        instance.variable.retain(|id| id != &ref_id);
        let removed = instance.variable.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<Variable>(&ref_id) {
            target.activity_scope = None;
            let _ = registry::insert(ref_id.clone(), "Variable", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_variable(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.variable.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.variable {
        // Update opposite reference
        if let Some(mut target) = registry::get::<Variable>(&ref_id) {
            target.activity_scope = None;
            let _ = registry::insert(ref_id.clone(), "Variable", &target);
        }
        }
        
        instance.variable.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_node(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.node.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.node.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityNode>(&ref_id) {
            target.activity = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "ActivityNode", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_node(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.node.len();
        instance.node.retain(|id| id != &ref_id);
        let removed = instance.node.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityNode>(&ref_id) {
            target.activity = None;
            let _ = registry::insert(ref_id.clone(), "ActivityNode", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_node(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.node.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.node {
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityNode>(&ref_id) {
            target.activity = None;
            let _ = registry::insert(ref_id.clone(), "ActivityNode", &target);
        }
        }
        
        instance.node.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_edge(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.edge.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.edge.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityEdge>(&ref_id) {
            target.activity = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_edge(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.edge.len();
        instance.edge.retain(|id| id != &ref_id);
        let removed = instance.edge.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityEdge>(&ref_id) {
            target.activity = None;
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_edge(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.edge.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.edge {
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityEdge>(&ref_id) {
            target.activity = None;
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        }
        
        instance.edge.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_partition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.partition.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.partition.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_partition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.partition.len();
        instance.partition.retain(|id| id != &ref_id);
        let removed = instance.partition.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_partition(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.partition.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.partition.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_group(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.group.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.group.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityGroup>(&ref_id) {
            target.in_activity = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "ActivityGroup", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_group(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.group.len();
        instance.group.retain(|id| id != &ref_id);
        let removed = instance.group.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityGroup>(&ref_id) {
            target.in_activity = None;
            let _ = registry::insert(ref_id.clone(), "ActivityGroup", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_group(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.group.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.group {
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityGroup>(&ref_id) {
            target.in_activity = None;
            let _ = registry::insert(ref_id.clone(), "ActivityGroup", &target);
        }
        }
        
        instance.group.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
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
        "Activity".to_string()
    }


}
