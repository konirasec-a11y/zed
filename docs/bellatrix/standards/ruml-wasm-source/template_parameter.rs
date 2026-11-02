// TemplateParameter - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::template_signature::TemplateSignature;
use crate::parameterable_element::ParameterableElement;


const TYPE_NAME: &str = "TemplateParameter";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct TemplateParameter {
    /// Unique identifier for this instance
    #[wasm_bindgen(skip)]
    pub id: String,
    #[wasm_bindgen(skip)]
    pub e_annotations: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_comment: Vec<String>,
    #[wasm_bindgen(skip)]
    pub signature: String,
    #[wasm_bindgen(skip)]
    pub parametered_element: String,
    #[wasm_bindgen(skip)]
    pub owned_parametered_element: Option<String>,
    #[wasm_bindgen(skip)]
    pub default: Option<String>,
    #[wasm_bindgen(skip)]
    pub owned_default: Option<String>,
}

#[wasm_bindgen]
impl TemplateParameter {
    /// Creates a new TemplateParameter instance
    #[wasm_bindgen]
    pub fn create(signature: String, parametered_element: String) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            signature,
            parametered_element,
            owned_parametered_element: None,
            default: None,
            owned_default: None,
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
        let instance: TemplateParameter = serde_wasm_bindgen::from_value(value)
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
    pub fn set_signature(instance_id: String, ref_id: String) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let old_ref_id = instance.signature.clone();
            if let Some(mut old_target) = registry::get::<TemplateSignature>(&old_ref_id) {
                old_target.owned_parameter.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "TemplateSignature", &old_target);
            }
        
        instance.signature = ref_id.clone();
        
            if let Some(mut new_target) = registry::get::<TemplateSignature>(&ref_id) {
                if !new_target.owned_parameter.contains(&instance_id) {
                    new_target.owned_parameter.push(instance_id.clone());
                }
                let _ = registry::insert(ref_id.clone(), "TemplateSignature", &new_target);
            }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_parametered_element(instance_id: String, ref_id: String) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let old_ref_id = instance.parametered_element.clone();
            if let Some(mut old_target) = registry::get::<ParameterableElement>(&old_ref_id) {
                old_target.template_parameter = None;
                let _ = registry::insert(old_ref_id.clone(), "ParameterableElement", &old_target);
            }
        
        instance.parametered_element = ref_id.clone();
        
            if let Some(mut new_target) = registry::get::<ParameterableElement>(&ref_id) {
                new_target.template_parameter = Some(instance_id.clone());
                let _ = registry::insert(ref_id.clone(), "ParameterableElement", &new_target);
            }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_owned_parametered_element(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.owned_parametered_element {
            if let Some(mut old_target) = registry::get::<ParameterableElement>(&old_ref_id) {
                old_target.owning_template_parameter = None;
                let _ = registry::insert(old_ref_id.clone(), "ParameterableElement", &old_target);
            }
        }
        
        instance.owned_parametered_element = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<ParameterableElement>(&new_ref_id) {
                new_target.owning_template_parameter = Some(instance_id.clone());
                let _ = registry::insert(new_ref_id.clone(), "ParameterableElement", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_owned_parametered_element(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.owned_parametered_element.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.owned_parametered_element {
            if let Some(mut old_target) = registry::get::<ParameterableElement>(&old_ref_id) {
                old_target.owning_template_parameter = None;
                let _ = registry::insert(old_ref_id.clone(), "ParameterableElement", &old_target);
            }
        }
        
        instance.owned_parametered_element = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn set_default(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        instance.default = ref_id.clone();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_default(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.default.is_none() {
            return Ok(false);
        }
        
        instance.default = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn set_owned_default(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        instance.owned_default = ref_id.clone();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_owned_default(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.owned_default.is_none() {
            return Ok(false);
        }
        
        instance.owned_default = None;
        
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
        "TemplateParameter".to_string()
    }


}
