// TemplateSignature - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::template_parameter::TemplateParameter;
use crate::templateable_element::TemplateableElement;


const TYPE_NAME: &str = "TemplateSignature";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct TemplateSignature {
    /// Unique identifier for this instance
    #[wasm_bindgen(skip)]
    pub id: String,
    #[wasm_bindgen(skip)]
    pub e_annotations: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_comment: Vec<String>,
    #[wasm_bindgen(skip)]
    pub parameter: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_parameter: Vec<String>,
    #[wasm_bindgen(skip)]
    pub template: String,
}

#[wasm_bindgen]
impl TemplateSignature {
    /// Creates a new TemplateSignature instance
    #[wasm_bindgen]
    pub fn create(template: String) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            parameter: Vec::new(),
            owned_parameter: Vec::new(),
            template,
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
        let instance: TemplateSignature = serde_wasm_bindgen::from_value(value)
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
    pub fn add_parameter(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.parameter.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.parameter.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_parameter(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.parameter.len();
        instance.parameter.retain(|id| id != &ref_id);
        let removed = instance.parameter.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_parameter(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.parameter.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.parameter.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_owned_parameter(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.owned_parameter.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.owned_parameter.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<TemplateParameter>(&ref_id) {
            target.signature = instance_id.clone();
            let _ = registry::insert(ref_id.clone(), "TemplateParameter", &target);
        }
        
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
        
        // Update opposite reference
        if let Some(target) = registry::get::<TemplateParameter>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "TemplateParameter", &target);
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
        
        for ref_id in &instance.owned_parameter {
        // Update opposite reference
        if let Some(target) = registry::get::<TemplateParameter>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "TemplateParameter", &target);
        }
        }
        
        instance.owned_parameter.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_template(instance_id: String, ref_id: String) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let old_ref_id = instance.template.clone();
            if let Some(mut old_target) = registry::get::<TemplateableElement>(&old_ref_id) {
                old_target.owned_template_signature = None;
                let _ = registry::insert(old_ref_id.clone(), "TemplateableElement", &old_target);
            }
        
        instance.template = ref_id.clone();
        
            if let Some(mut new_target) = registry::get::<TemplateableElement>(&ref_id) {
                new_target.owned_template_signature = Some(instance_id.clone());
                let _ = registry::insert(ref_id.clone(), "TemplateableElement", &new_target);
            }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
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
        "TemplateSignature".to_string()
    }


}
