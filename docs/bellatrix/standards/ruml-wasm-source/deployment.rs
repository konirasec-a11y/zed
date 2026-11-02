// Deployment - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::deployment_specification::DeploymentSpecification;
use crate::deployment_target::DeploymentTarget;


const TYPE_NAME: &str = "Deployment";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct Deployment {
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
    pub owning_template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub supplier: Vec<String>,
    #[wasm_bindgen(skip)]
    pub client: Vec<String>,
    #[wasm_bindgen(skip)]
    pub deployed_artifact: Vec<String>,
    #[wasm_bindgen(skip)]
    pub configuration: Vec<String>,
    #[wasm_bindgen(skip)]
    pub location: String,
}

#[wasm_bindgen]
impl Deployment {
    /// Creates a new Deployment instance
    #[wasm_bindgen]
    pub fn create(location: String) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            owning_template_parameter: None,
            template_parameter: None,
            supplier: Vec::new(),
            client: Vec::new(),
            deployed_artifact: Vec::new(),
            configuration: Vec::new(),
            location,
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
        let instance: Deployment = serde_wasm_bindgen::from_value(value)
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
    pub fn add_deployed_artifact(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.deployed_artifact.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.deployed_artifact.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_deployed_artifact(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.deployed_artifact.len();
        instance.deployed_artifact.retain(|id| id != &ref_id);
        let removed = instance.deployed_artifact.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_deployed_artifact(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.deployed_artifact.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.deployed_artifact.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn add_configuration(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.configuration.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.configuration.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<DeploymentSpecification>(&ref_id) {
            target.deployment = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "DeploymentSpecification", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_configuration(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.configuration.len();
        instance.configuration.retain(|id| id != &ref_id);
        let removed = instance.configuration.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<DeploymentSpecification>(&ref_id) {
            target.deployment = None;
            let _ = registry::insert(ref_id.clone(), "DeploymentSpecification", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_configuration(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.configuration.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.configuration {
        // Update opposite reference
        if let Some(mut target) = registry::get::<DeploymentSpecification>(&ref_id) {
            target.deployment = None;
            let _ = registry::insert(ref_id.clone(), "DeploymentSpecification", &target);
        }
        }
        
        instance.configuration.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_location(instance_id: String, ref_id: String) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let old_ref_id = instance.location.clone();
            if let Some(mut old_target) = registry::get::<DeploymentTarget>(&old_ref_id) {
                old_target.deployment.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "DeploymentTarget", &old_target);
            }
        
        instance.location = ref_id.clone();
        
            if let Some(mut new_target) = registry::get::<DeploymentTarget>(&ref_id) {
                if !new_target.deployment.contains(&instance_id) {
                    new_target.deployment.push(instance_id.clone());
                }
                let _ = registry::insert(ref_id.clone(), "DeploymentTarget", &new_target);
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
        "Deployment".to_string()
    }


}
