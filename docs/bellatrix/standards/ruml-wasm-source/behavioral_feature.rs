// BehavioralFeature - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::behavior::Behavior;
use crate::call_concurrency_kind::CallConcurrencyKind;


const TYPE_NAME: &str = "BehavioralFeature";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct BehavioralFeature {
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
    pub is_static: String,
    #[wasm_bindgen(skip)]
    pub owned_parameter: Vec<String>,
    #[wasm_bindgen(skip)]
    pub is_abstract: String,
    #[wasm_bindgen(skip)]
    pub method: Vec<String>,
    #[wasm_bindgen(skip)]
    pub concurrency: CallConcurrencyKind,
    #[wasm_bindgen(skip)]
    pub raised_exception: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_parameter_set: Vec<String>,
}

#[wasm_bindgen]
impl BehavioralFeature {
    /// Creates a new BehavioralFeature instance
    #[wasm_bindgen]
    pub fn create(is_leaf: String, is_static: String, is_abstract: String, concurrency: CallConcurrencyKind) -> String {
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
            is_static,
            owned_parameter: Vec::new(),
            is_abstract,
            method: Vec::new(),
            concurrency,
            raised_exception: Vec::new(),
            owned_parameter_set: Vec::new(),
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
        let instance: BehavioralFeature = serde_wasm_bindgen::from_value(value)
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
pub fn set_is_static(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_static = value;
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
pub fn set_concurrency(instance_id: String, value: CallConcurrencyKind) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.concurrency = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}





    #[wasm_bindgen]
    pub fn add_owned_parameter(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.owned_parameter.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.owned_parameter.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_owned_parameter(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.owned_parameter.len();
        instance.owned_parameter.retain(|id| id != &ref_id);
        let removed = instance.owned_parameter.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_owned_parameter(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.owned_parameter.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.owned_parameter.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_method(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.method.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.method.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<Behavior>(&ref_id) {
            target.specification = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "Behavior", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_method(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.method.len();
        instance.method.retain(|id| id != &ref_id);
        let removed = instance.method.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<Behavior>(&ref_id) {
            target.specification = None;
            let _ = registry::insert(ref_id.clone(), "Behavior", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_method(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.method.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.method {
        // Update opposite reference
        if let Some(mut target) = registry::get::<Behavior>(&ref_id) {
            target.specification = None;
            let _ = registry::insert(ref_id.clone(), "Behavior", &target);
        }
        }
        
        instance.method.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_raised_exception(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.raised_exception.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.raised_exception.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_raised_exception(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.raised_exception.len();
        instance.raised_exception.retain(|id| id != &ref_id);
        let removed = instance.raised_exception.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_raised_exception(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.raised_exception.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.raised_exception.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_owned_parameter_set(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.owned_parameter_set.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.owned_parameter_set.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_owned_parameter_set(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.owned_parameter_set.len();
        instance.owned_parameter_set.retain(|id| id != &ref_id);
        let removed = instance.owned_parameter_set.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_owned_parameter_set(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.owned_parameter_set.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.owned_parameter_set.clear();
        
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
        "BehavioralFeature".to_string()
    }


}
