// Generalization - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::classifier::Classifier;
use crate::generalization_set::GeneralizationSet;


const TYPE_NAME: &str = "Generalization";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct Generalization {
    /// Unique identifier for this instance
    #[wasm_bindgen(skip)]
    pub id: String,
    #[wasm_bindgen(skip)]
    pub e_annotations: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_comment: Vec<String>,
    #[wasm_bindgen(skip)]
    pub is_substitutable: Option<String>,
    #[wasm_bindgen(skip)]
    pub general: String,
    #[wasm_bindgen(skip)]
    pub generalization_set: Vec<String>,
    #[wasm_bindgen(skip)]
    pub specific: String,
}

#[wasm_bindgen]
impl Generalization {
    /// Creates a new Generalization instance
    #[wasm_bindgen]
    pub fn create(general: String, specific: String) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            is_substitutable: None,
            general,
            generalization_set: Vec::new(),
            specific,
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
        let instance: Generalization = serde_wasm_bindgen::from_value(value)
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
pub fn set_is_substitutable(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_substitutable = Some(value);
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn clear_is_substitutable(instance_id: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_substitutable = None;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}





    #[wasm_bindgen]
    pub fn set_general(instance_id: String, ref_id: String) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        instance.general = ref_id.clone();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn add_generalization_set(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.generalization_set.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.generalization_set.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<GeneralizationSet>(&ref_id) {
            if !target.generalization.contains(&instance_id) {
                target.generalization.push(instance_id.clone());
            }
            let _ = registry::insert(ref_id.clone(), "GeneralizationSet", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_generalization_set(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.generalization_set.len();
        instance.generalization_set.retain(|id| id != &ref_id);
        let removed = instance.generalization_set.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<GeneralizationSet>(&ref_id) {
            target.generalization.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "GeneralizationSet", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_generalization_set(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.generalization_set.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.generalization_set {
        // Update opposite reference
        if let Some(mut target) = registry::get::<GeneralizationSet>(&ref_id) {
            target.generalization.retain(|id| id != &instance_id);
            let _ = registry::insert(ref_id.clone(), "GeneralizationSet", &target);
        }
        }
        
        instance.generalization_set.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_specific(instance_id: String, ref_id: String) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let old_ref_id = instance.specific.clone();
            if let Some(mut old_target) = registry::get::<Classifier>(&old_ref_id) {
                old_target.generalization.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Classifier", &old_target);
            }
        
        instance.specific = ref_id.clone();
        
            if let Some(mut new_target) = registry::get::<Classifier>(&ref_id) {
                if !new_target.generalization.contains(&instance_id) {
                    new_target.generalization.push(instance_id.clone());
                }
                let _ = registry::insert(ref_id.clone(), "Classifier", &new_target);
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
        "Generalization".to_string()
    }


}
