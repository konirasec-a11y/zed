// TemplateBinding - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::template_parameter_substitution::TemplateParameterSubstitution;
use crate::templateable_element::TemplateableElement;


const TYPE_NAME: &str = "TemplateBinding";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct TemplateBinding {
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
    pub parameter_substitution: Vec<String>,
    #[wasm_bindgen(skip)]
    pub bound_element: String,
}

#[wasm_bindgen]
impl TemplateBinding {
    /// Creates a new TemplateBinding instance
    #[wasm_bindgen]
    pub fn create(signature: String, bound_element: String) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            signature,
            parameter_substitution: Vec::new(),
            bound_element,
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
        let instance: TemplateBinding = serde_wasm_bindgen::from_value(value)
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
        
        instance.signature = ref_id.clone();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn add_parameter_substitution(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.parameter_substitution.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.parameter_substitution.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<TemplateParameterSubstitution>(&ref_id) {
            target.template_binding = instance_id.clone();
            let _ = registry::insert(ref_id.clone(), "TemplateParameterSubstitution", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_parameter_substitution(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.parameter_substitution.len();
        instance.parameter_substitution.retain(|id| id != &ref_id);
        let removed = instance.parameter_substitution.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(target) = registry::get::<TemplateParameterSubstitution>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "TemplateParameterSubstitution", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_parameter_substitution(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.parameter_substitution.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.parameter_substitution {
        // Update opposite reference
        if let Some(target) = registry::get::<TemplateParameterSubstitution>(&ref_id) {
            let _ = registry::insert(ref_id.clone(), "TemplateParameterSubstitution", &target);
        }
        }
        
        instance.parameter_substitution.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_bound_element(instance_id: String, ref_id: String) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let old_ref_id = instance.bound_element.clone();
            if let Some(mut old_target) = registry::get::<TemplateableElement>(&old_ref_id) {
                old_target.template_binding.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "TemplateableElement", &old_target);
            }
        
        instance.bound_element = ref_id.clone();
        
            if let Some(mut new_target) = registry::get::<TemplateableElement>(&ref_id) {
                if !new_target.template_binding.contains(&instance_id) {
                    new_target.template_binding.push(instance_id.clone());
                }
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
        "TemplateBinding".to_string()
    }


}
