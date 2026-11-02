// ActivityPartition - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::activity_node::ActivityNode;
use crate::activity_edge::ActivityEdge;


const TYPE_NAME: &str = "ActivityPartition";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct ActivityPartition {
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
    pub in_activity: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_dimension: String,
    #[wasm_bindgen(skip)]
    pub is_external: String,
    #[wasm_bindgen(skip)]
    pub node: Vec<String>,
    #[wasm_bindgen(skip)]
    pub subpartition: Vec<String>,
    #[wasm_bindgen(skip)]
    pub super_partition: Option<String>,
    #[wasm_bindgen(skip)]
    pub represents: Option<String>,
    #[wasm_bindgen(skip)]
    pub edge: Vec<String>,
}

#[wasm_bindgen]
impl ActivityPartition {
    /// Creates a new ActivityPartition instance
    #[wasm_bindgen]
    pub fn create(is_dimension: String, is_external: String) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            in_activity: None,
            is_dimension,
            is_external,
            node: Vec::new(),
            subpartition: Vec::new(),
            super_partition: None,
            represents: None,
            edge: Vec::new(),
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
        let instance: ActivityPartition = serde_wasm_bindgen::from_value(value)
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
pub fn set_is_dimension(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_dimension = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_external(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_external = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
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
            if !target.in_partition.contains(&instance_id) {
                target.in_partition.push(instance_id.clone());
            }
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
            target.in_partition.retain(|id| id != &instance_id);
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
            target.in_partition.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "ActivityNode", &target);
        }
        }
        
        instance.node.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_subpartition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.subpartition.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.subpartition.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityPartition>(&ref_id) {
            target.super_partition = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "ActivityPartition", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_subpartition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.subpartition.len();
        instance.subpartition.retain(|id| id != &ref_id);
        let removed = instance.subpartition.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityPartition>(&ref_id) {
            target.super_partition = None;
            let _ = registry::insert(ref_id.clone(), "ActivityPartition", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_subpartition(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.subpartition.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.subpartition {
        // Update opposite reference
        if let Some(mut target) = registry::get::<ActivityPartition>(&ref_id) {
            target.super_partition = None;
            let _ = registry::insert(ref_id.clone(), "ActivityPartition", &target);
        }
        }
        
        instance.subpartition.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_super_partition(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.super_partition {
            if let Some(mut old_target) = registry::get::<ActivityPartition>(&old_ref_id) {
                old_target.subpartition.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "ActivityPartition", &old_target);
            }
        }
        
        instance.super_partition = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<ActivityPartition>(&new_ref_id) {
                if !new_target.subpartition.contains(&instance_id) {
                    new_target.subpartition.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "ActivityPartition", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_super_partition(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.super_partition.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.super_partition {
            if let Some(mut old_target) = registry::get::<ActivityPartition>(&old_ref_id) {
                old_target.subpartition.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "ActivityPartition", &old_target);
            }
        }
        
        instance.super_partition = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn set_represents(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        instance.represents = ref_id.clone();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_represents(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.represents.is_none() {
            return Ok(false);
        }
        
        instance.represents = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
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
            if !target.in_partition.contains(&instance_id) {
                target.in_partition.push(instance_id.clone());
            }
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
            target.in_partition.retain(|id| id != &instance_id);
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
            target.in_partition.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "ActivityEdge", &target);
        }
        }
        
        instance.edge.clear();
        
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
        "ActivityPartition".to_string()
    }


}
