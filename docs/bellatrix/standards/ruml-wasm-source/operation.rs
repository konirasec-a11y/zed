// Operation - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::call_concurrency_kind::CallConcurrencyKind;
use crate::interface::Interface;
use crate::class::Class;
use crate::data_type::DataType;


const TYPE_NAME: &str = "Operation";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct Operation {
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
    #[wasm_bindgen(skip)]
    pub owning_template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_binding: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_template_signature: Option<String>,
    #[wasm_bindgen(skip)]
    pub interface: Option<String>,
    #[wasm_bindgen(skip)]
    pub class: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_query: String,
    #[wasm_bindgen(skip)]
    pub precondition: Vec<String>,
    #[wasm_bindgen(skip)]
    pub postcondition: Vec<String>,
    #[wasm_bindgen(skip)]
    pub redefined_operation: Vec<String>,
    #[wasm_bindgen(skip)]
    pub datatype: Option<String>,
    #[wasm_bindgen(skip)]
    pub body_condition: Option<String>,
}

#[wasm_bindgen]
impl Operation {
    /// Creates a new Operation instance
    #[wasm_bindgen]
    pub fn create(is_leaf: String, is_static: String, is_abstract: String, concurrency: CallConcurrencyKind, is_query: String) -> String {
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
            owning_template_parameter: None,
            template_parameter: None,
            template_binding: Vec::new(),
            owned_template_signature: None,
            interface: None,
            class: None,
            is_query,
            precondition: Vec::new(),
            postcondition: Vec::new(),
            redefined_operation: Vec::new(),
            datatype: None,
            body_condition: None,
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
        let instance: Operation = serde_wasm_bindgen::from_value(value)
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
pub fn set_is_query(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_query = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}





    #[wasm_bindgen]
    pub fn set_interface(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.interface {
            if let Some(mut old_target) = registry::get::<Interface>(&old_ref_id) {
                old_target.owned_operation.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Interface", &old_target);
            }
        }
        
        instance.interface = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<Interface>(&new_ref_id) {
                if !new_target.owned_operation.contains(&instance_id) {
                    new_target.owned_operation.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "Interface", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_interface(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.interface.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.interface {
            if let Some(mut old_target) = registry::get::<Interface>(&old_ref_id) {
                old_target.owned_operation.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Interface", &old_target);
            }
        }
        
        instance.interface = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn set_class(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.class {
            if let Some(mut old_target) = registry::get::<Class>(&old_ref_id) {
                old_target.owned_operation.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Class", &old_target);
            }
        }
        
        instance.class = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<Class>(&new_ref_id) {
                if !new_target.owned_operation.contains(&instance_id) {
                    new_target.owned_operation.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "Class", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_class(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.class.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.class {
            if let Some(mut old_target) = registry::get::<Class>(&old_ref_id) {
                old_target.owned_operation.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Class", &old_target);
            }
        }
        
        instance.class = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn add_precondition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.precondition.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.precondition.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_precondition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.precondition.len();
        instance.precondition.retain(|id| id != &ref_id);
        let removed = instance.precondition.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_precondition(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.precondition.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.precondition.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_postcondition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.postcondition.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.postcondition.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_postcondition(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.postcondition.len();
        instance.postcondition.retain(|id| id != &ref_id);
        let removed = instance.postcondition.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_postcondition(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.postcondition.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.postcondition.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_redefined_operation(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.redefined_operation.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.redefined_operation.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_redefined_operation(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.redefined_operation.len();
        instance.redefined_operation.retain(|id| id != &ref_id);
        let removed = instance.redefined_operation.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_redefined_operation(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.redefined_operation.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.redefined_operation.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_datatype(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.datatype {
            if let Some(mut old_target) = registry::get::<DataType>(&old_ref_id) {
                old_target.owned_operation.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "DataType", &old_target);
            }
        }
        
        instance.datatype = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<DataType>(&new_ref_id) {
                if !new_target.owned_operation.contains(&instance_id) {
                    new_target.owned_operation.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "DataType", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_datatype(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.datatype.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.datatype {
            if let Some(mut old_target) = registry::get::<DataType>(&old_ref_id) {
                old_target.owned_operation.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "DataType", &old_target);
            }
        }
        
        instance.datatype = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn set_body_condition(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        instance.body_condition = ref_id.clone();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_body_condition(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.body_condition.is_none() {
            return Ok(false);
        }
        
        instance.body_condition = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
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
        "Operation".to_string()
    }


}
