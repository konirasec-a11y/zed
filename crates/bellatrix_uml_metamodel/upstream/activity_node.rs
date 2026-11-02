// ActivityNode - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::structured_activity_node::StructuredActivityNode;
use crate::activity::Activity;
use crate::activity_edge::ActivityEdge;
use crate::activity_partition::ActivityPartition;
use crate::interruptible_activity_region::InterruptibleActivityRegion;


const TYPE_NAME: &str = "ActivityNode";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct ActivityNode {
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
    pub is_leaf: String,
    #[wasm_bindgen(skip)]
    pub in_structured_node: Option<String>,
    #[wasm_bindgen(skip)]
    pub activity: Option<String>,
    #[wasm_bindgen(skip)]
    pub outgoing: Vec<String>,
    #[wasm_bindgen(skip)]
    pub incoming: Vec<String>,
    #[wasm_bindgen(skip)]
    pub in_partition: Vec<String>,
    #[wasm_bindgen(skip)]
    pub in_interruptible_region: Vec<String>,
    #[wasm_bindgen(skip)]
    pub redefined_node: Vec<String>,
}

#[wasm_bindgen]
impl ActivityNode {
    /// Creates a new ActivityNode instance
    #[wasm_bindgen]
    pub fn create(is_leaf: String) -> String {
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
        let instance: ActivityNode = serde_wasm_bindgen::from_value(value)
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
    pub fn set_in_structured_node(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.in_structured_node {
            if let Some(mut old_target) = registry::get::<StructuredActivityNode>(&old_ref_id) {
                old_target.node.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "StructuredActivityNode", &old_target);
            }
        }
        
        instance.in_structured_node = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<StructuredActivityNode>(&new_ref_id) {
                if !new_target.node.contains(&instance_id) {
                    new_target.node.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "StructuredActivityNode", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_in_structured_node(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.in_structured_node.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.in_structured_node {
            if let Some(mut old_target) = registry::get::<StructuredActivityNode>(&old_ref_id) {
                old_target.node.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "StructuredActivityNode", &old_target);
            }
        }
        
        instance.in_structured_node = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn set_activity(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.activity {
            if let Some(mut old_target) = registry::get::<Activity>(&old_ref_id) {
                old_target.node.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Activity", &old_target);
            }
        }
        
        instance.activity = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<Activity>(&new_ref_id) {
                if !new_target.node.contains(&instance_id) {
                    new_target.node.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "Activity", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_activity(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.activity.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.activity {
            if let Some(mut old_target) = registry::get::<Activity>(&old_ref_id) {
                old_target.node.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Activity", &old_target);
            }
        }
        
        instance.activity = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn add_outgoing(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.outgoing.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.outgoing.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityEdge>(&ref_id) {
            target.source = instance_id.clone();
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_outgoing(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.outgoing.len();
        instance.outgoing.retain(|id| id != &ref_id);
        let removed = instance.outgoing.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(target) = registry::get::<ActivityEdge>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_outgoing(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.outgoing.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.outgoing {
        // Update opposite reference
        if let Some(target) = registry::get::<ActivityEdge>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        }
        
        instance.outgoing.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_incoming(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.incoming.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.incoming.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityEdge>(&ref_id) {
            target.target = instance_id.clone();
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_incoming(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.incoming.len();
        instance.incoming.retain(|id| id != &ref_id);
        let removed = instance.incoming.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(target) = registry::get::<ActivityEdge>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_incoming(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.incoming.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.incoming {
        // Update opposite reference
        if let Some(target) = registry::get::<ActivityEdge>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        }
        
        instance.incoming.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_in_partition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.in_partition.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.in_partition.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityPartition>(&ref_id) {
            if !target.node.contains(&instance_id) {
                target.node.push(instance_id.clone());
            }
            let _ = registry::insert(ref_id.clone(), "ActivityPartition", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_in_partition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.in_partition.len();
        instance.in_partition.retain(|id| id != &ref_id);
        let removed = instance.in_partition.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityPartition>(&ref_id) {
            target.node.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "ActivityPartition", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_in_partition(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.in_partition.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.in_partition {
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityPartition>(&ref_id) {
            target.node.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "ActivityPartition", &target);
        }
        }
        
        instance.in_partition.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_in_interruptible_region(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.in_interruptible_region.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.in_interruptible_region.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<InterruptibleActivityRegion>(&ref_id) {
            if !target.node.contains(&instance_id) {
                target.node.push(instance_id.clone());
            }
            let _ = registry::insert(ref_id.clone(), "InterruptibleActivityRegion", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_in_interruptible_region(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.in_interruptible_region.len();
        instance.in_interruptible_region.retain(|id| id != &ref_id);
        let removed = instance.in_interruptible_region.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<InterruptibleActivityRegion>(&ref_id) {
            target.node.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "InterruptibleActivityRegion", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_in_interruptible_region(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.in_interruptible_region.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.in_interruptible_region {
        // Update opposite reference
        if let Some(mut target) = registry::get::<InterruptibleActivityRegion>(&ref_id) {
            target.node.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "InterruptibleActivityRegion", &target);
        }
        }
        
        instance.in_interruptible_region.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_redefined_node(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.redefined_node.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.redefined_node.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_redefined_node(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.redefined_node.len();
        instance.redefined_node.retain(|id| id != &ref_id);
        let removed = instance.redefined_node.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_redefined_node(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.redefined_node.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.redefined_node.clear();
        
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
        "ActivityNode".to_string()
    }


}
