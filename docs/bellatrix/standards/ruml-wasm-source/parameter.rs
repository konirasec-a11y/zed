// Parameter - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::parameter_set::ParameterSet;
use crate::parameter_direction_kind::ParameterDirectionKind;
use crate::parameter_effect_kind::ParameterEffectKind;


const TYPE_NAME: &str = "Parameter";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct Parameter {
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
    pub type_: Option<String>,
    #[wasm_bindgen(skip)]
    pub owning_template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_ordered: String,
    #[wasm_bindgen(skip)]
    pub is_unique: String,
    #[wasm_bindgen(skip)]
    pub upper_value: Option<String>,
    #[wasm_bindgen(skip)]
    pub lower_value: Option<String>,
    #[wasm_bindgen(skip)]
    pub parameter_set: Vec<String>,
    #[wasm_bindgen(skip)]
    pub direction: ParameterDirectionKind,
    #[wasm_bindgen(skip)]
    pub default_value: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_exception: String,
    #[wasm_bindgen(skip)]
    pub is_stream: String,
    #[wasm_bindgen(skip)]
    pub effect: Option<ParameterEffectKind>,
}

#[wasm_bindgen]
impl Parameter {
    /// Creates a new Parameter instance
    #[wasm_bindgen]
    pub fn create(is_ordered: String, is_unique: String, direction: ParameterDirectionKind, is_exception: String, is_stream: String) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            type_: None,
            owning_template_parameter: None,
            template_parameter: None,
            is_ordered,
            is_unique,
            upper_value: None,
            lower_value: None,
            parameter_set: Vec::new(),
            direction,
            default_value: None,
            is_exception,
            is_stream,
            effect: None,
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
        let instance: Parameter = serde_wasm_bindgen::from_value(value)
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
pub fn set_is_ordered(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_ordered = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_unique(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_unique = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_direction(instance_id: String, value: ParameterDirectionKind) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.direction = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_exception(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_exception = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_stream(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_stream = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_effect(instance_id: String, value: ParameterEffectKind) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.effect = Some(value);
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn clear_effect(instance_id: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.effect = None;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}





    #[wasm_bindgen]
    pub fn add_parameter_set(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.parameter_set.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.parameter_set.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ParameterSet>(&ref_id) {
            if !target.parameter.contains(&instance_id) {
                target.parameter.push(instance_id.clone());
            }
            let _ = registry::insert(ref_id.clone(), "ParameterSet", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_parameter_set(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.parameter_set.len();
        instance.parameter_set.retain(|id| id != &ref_id);
        let removed = instance.parameter_set.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<ParameterSet>(&ref_id) {
            target.parameter.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "ParameterSet", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_parameter_set(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.parameter_set.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.parameter_set {
        // Update opposite reference
        if let Some(mut target) = registry::get::<ParameterSet>(&ref_id) {
            target.parameter.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "ParameterSet", &target);
        }
        }
        
        instance.parameter_set.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_default_value(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        instance.default_value = ref_id.clone();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_default_value(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.default_value.is_none() {
            return Ok(false);
        }
        
        instance.default_value = None;
        
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
        "Parameter".to_string()
    }


}
