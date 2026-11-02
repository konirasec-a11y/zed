// TemplateableElement - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::template_binding::TemplateBinding;
use crate::template_signature::TemplateSignature;


const TYPE_NAME: &str = "TemplateableElement";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct TemplateableElement {
    /// Unique identifier for this instance
    #[wasm_bindgen(skip)]
    pub id: String,
    #[wasm_bindgen(skip)]
    pub e_annotations: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_comment: Vec<String>,
    #[wasm_bindgen(skip)]
    pub template_binding: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_template_signature: Option<String>,
}

#[wasm_bindgen]
impl TemplateableElement {
    /// Creates a new TemplateableElement instance
    #[wasm_bindgen]
    pub fn create() -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            template_binding: Vec::new(),
            owned_template_signature: None,
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
        let instance: TemplateableElement = serde_wasm_bindgen::from_value(value)
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
    pub fn add_template_binding(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.template_binding.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.template_binding.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<TemplateBinding>(&ref_id) {
            target.bound_element = instance_id.clone();
            let _ = registry::insert(ref_id.clone(), "TemplateBinding", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_template_binding(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.template_binding.len();
        instance.template_binding.retain(|id| id != &ref_id);
        let removed = instance.template_binding.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(target) = registry::get::<TemplateBinding>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "TemplateBinding", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_template_binding(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.template_binding.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.template_binding {
        // Update opposite reference
        if let Some(target) = registry::get::<TemplateBinding>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "TemplateBinding", &target);
        }
        }
        
        instance.template_binding.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_owned_template_signature(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.owned_template_signature {
            if let Some(old_target) = registry::get::<TemplateSignature>(&old_ref_id) {
                let _ = registry::insert(old_ref_id.clone(), "TemplateSignature", &old_target);
            }
        }
        
        instance.owned_template_signature = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<TemplateSignature>(&new_ref_id) {
                new_target.template = instance_id.clone();
                let _ = registry::insert(new_ref_id.clone(), "TemplateSignature", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_owned_template_signature(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.owned_template_signature.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.owned_template_signature {
            if let Some(old_target) = registry::get::<TemplateSignature>(&old_ref_id) {
                let _ = registry::insert(old_ref_id.clone(), "TemplateSignature", &old_target);
            }
        }
        
        instance.owned_template_signature = None;
        
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
        "TemplateableElement".to_string()
    }


}
