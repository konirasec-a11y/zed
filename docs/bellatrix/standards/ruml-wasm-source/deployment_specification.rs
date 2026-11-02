// DeploymentSpecification - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::deployment::Deployment;


const TYPE_NAME: &str = "DeploymentSpecification";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct DeploymentSpecification {
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
    pub owning_template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_binding: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_template_signature: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_abstract: String,
    #[wasm_bindgen(skip)]
    pub generalization: Vec<String>,
    #[wasm_bindgen(skip)]
    pub powertype_extent: Vec<String>,
    #[wasm_bindgen(skip)]
    pub redefined_classifier: Vec<String>,
    #[wasm_bindgen(skip)]
    pub substitution: Vec<String>,
    #[wasm_bindgen(skip)]
    pub representation: Option<String>,
    #[wasm_bindgen(skip)]
    pub collaboration_use: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_use_case: Vec<String>,
    #[wasm_bindgen(skip)]
    pub use_case: Vec<String>,
    #[wasm_bindgen(skip)]
    pub file_name: Option<String>,
    #[wasm_bindgen(skip)]
    pub nested_artifact: Vec<String>,
    #[wasm_bindgen(skip)]
    pub manifestation: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_operation: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_attribute: Vec<String>,
    #[wasm_bindgen(skip)]
    pub deployment_location: Option<String>,
    #[wasm_bindgen(skip)]
    pub execution_location: Option<String>,
    #[wasm_bindgen(skip)]
    pub deployment: Option<String>,
}

#[wasm_bindgen]
impl DeploymentSpecification {
    /// Creates a new DeploymentSpecification instance
    #[wasm_bindgen]
    pub fn create(is_leaf: String, is_abstract: String) -> String {
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
            owning_template_parameter: None,
            template_parameter: None,
            template_binding: Vec::new(),
            owned_template_signature: None,
            is_abstract,
            generalization: Vec::new(),
            powertype_extent: Vec::new(),
            redefined_classifier: Vec::new(),
            substitution: Vec::new(),
            representation: None,
            collaboration_use: Vec::new(),
            owned_use_case: Vec::new(),
            use_case: Vec::new(),
            file_name: None,
            nested_artifact: Vec::new(),
            manifestation: Vec::new(),
            owned_operation: Vec::new(),
            owned_attribute: Vec::new(),
            deployment_location: None,
            execution_location: None,
            deployment: None,
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
        let instance: DeploymentSpecification = serde_wasm_bindgen::from_value(value)
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
pub fn set_is_abstract(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_abstract = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_file_name(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.file_name = Some(value);
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn clear_file_name(instance_id: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.file_name = None;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_deployment_location(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.deployment_location = Some(value);
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn clear_deployment_location(instance_id: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.deployment_location = None;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_execution_location(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.execution_location = Some(value);
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn clear_execution_location(instance_id: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.execution_location = None;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}





    #[wasm_bindgen]
    pub fn set_deployment(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.deployment {
            if let Some(mut old_target) = registry::get::<Deployment>(&old_ref_id) {
                old_target.configuration.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Deployment", &old_target);
            }
        }
        
        instance.deployment = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<Deployment>(&new_ref_id) {
                if !new_target.configuration.contains(&instance_id) {
                    new_target.configuration.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "Deployment", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_deployment(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.deployment.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.deployment {
            if let Some(mut old_target) = registry::get::<Deployment>(&old_ref_id) {
                old_target.configuration.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Deployment", &old_target);
            }
        }
        
        instance.deployment = None;
        
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
        "DeploymentSpecification".to_string()
    }


}
