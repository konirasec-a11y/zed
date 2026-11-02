// Property - Generated UML Class
// Uses unified registry with type filtering

use uuid::Uuid;
use crate::registry;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen;
use tsify::Tsify;
use crate::visibility_kind::VisibilityKind;
use crate::data_type::DataType;
use crate::aggregation_kind::AggregationKind;
use crate::association::Association;


const TYPE_NAME: &str = "Property";

#[derive(Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct Property {
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
    pub is_leaf: String,
    #[wasm_bindgen(skip)]
    pub is_static: String,
    #[wasm_bindgen(skip)]
    pub type_: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_ordered: String,
    #[wasm_bindgen(skip)]
    pub is_unique: String,
    #[wasm_bindgen(skip)]
    pub upper_value: Option<String>,
    #[wasm_bindgen(skip)]
    pub lower_value: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_read_only: String,
    #[wasm_bindgen(skip)]
    pub owning_template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub template_parameter: Option<String>,
    #[wasm_bindgen(skip)]
    pub deployment: Vec<String>,
    #[wasm_bindgen(skip)]
    pub template_binding: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owned_template_signature: Option<String>,
    #[wasm_bindgen(skip)]
    pub datatype: Option<String>,
    #[wasm_bindgen(skip)]
    pub is_derived: String,
    #[wasm_bindgen(skip)]
    pub is_derived_union: String,
    #[wasm_bindgen(skip)]
    pub aggregation: AggregationKind,
    #[wasm_bindgen(skip)]
    pub redefined_property: Vec<String>,
    #[wasm_bindgen(skip)]
    pub owning_association: Option<String>,
    #[wasm_bindgen(skip)]
    pub default_value: Option<String>,
    #[wasm_bindgen(skip)]
    pub subsetted_property: Vec<String>,
    #[wasm_bindgen(skip)]
    pub association: Option<String>,
    #[wasm_bindgen(skip)]
    pub qualifier: Vec<String>,
    #[wasm_bindgen(skip)]
    pub association_end: Option<String>,
}

#[wasm_bindgen]
impl Property {
    /// Creates a new Property instance
    #[wasm_bindgen]
    pub fn create(is_leaf: String, is_static: String, is_ordered: String, is_unique: String, is_read_only: String, is_derived: String, is_derived_union: String, aggregation: AggregationKind) -> String {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            is_leaf,
            is_static,
            type_: None,
            is_ordered,
            is_unique,
            upper_value: None,
            lower_value: None,
            is_read_only,
            owning_template_parameter: None,
            template_parameter: None,
            deployment: Vec::new(),
            template_binding: Vec::new(),
            owned_template_signature: None,
            datatype: None,
            is_derived,
            is_derived_union,
            aggregation,
            redefined_property: Vec::new(),
            owning_association: None,
            default_value: None,
            subsetted_property: Vec::new(),
            association: None,
            qualifier: Vec::new(),
            association_end: None,
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
        let instance: Property = serde_wasm_bindgen::from_value(value)
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
pub fn set_is_read_only(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_read_only = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_derived(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_derived = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_is_derived_union(instance_id: String, value: String) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.is_derived_union = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}

#[wasm_bindgen]
pub fn set_aggregation(instance_id: String, value: AggregationKind) -> Result<(), JsValue> {
    let mut instance: Self = registry::get(&instance_id)
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;
    instance.aggregation = value;
    registry::insert(instance_id, TYPE_NAME, &instance)?;
    Ok(())
}





    #[wasm_bindgen]
    pub fn set_datatype(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.datatype {
            if let Some(mut old_target) = registry::get::<DataType>(&old_ref_id) {
                old_target.owned_attribute.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "DataType", &old_target);
            }
        }
        
        instance.datatype = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<DataType>(&new_ref_id) {
                if !new_target.owned_attribute.contains(&instance_id) {
                    new_target.owned_attribute.push(instance_id.clone());
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
                old_target.owned_attribute.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "DataType", &old_target);
            }
        }
        
        instance.datatype = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn add_redefined_property(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.redefined_property.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.redefined_property.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_redefined_property(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.redefined_property.len();
        instance.redefined_property.retain(|id| id != &ref_id);
        let removed = instance.redefined_property.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_redefined_property(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.redefined_property.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.redefined_property.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_owning_association(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.owning_association {
            if let Some(mut old_target) = registry::get::<Association>(&old_ref_id) {
                old_target.owned_end.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Association", &old_target);
            }
        }
        
        instance.owning_association = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<Association>(&new_ref_id) {
                if !new_target.owned_end.contains(&instance_id) {
                    new_target.owned_end.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "Association", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_owning_association(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.owning_association.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.owning_association {
            if let Some(mut old_target) = registry::get::<Association>(&old_ref_id) {
                old_target.owned_end.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Association", &old_target);
            }
        }
        
        instance.owning_association = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
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

    #[wasm_bindgen]
    pub fn add_subsetted_property(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.subsetted_property.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.subsetted_property.push(ref_id.clone());
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_subsetted_property(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.subsetted_property.len();
        instance.subsetted_property.retain(|id| id != &ref_id);
        let removed = instance.subsetted_property.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_subsetted_property(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.subsetted_property.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        instance.subsetted_property.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_association(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.association {
            if let Some(mut old_target) = registry::get::<Association>(&old_ref_id) {
                old_target.member_end.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Association", &old_target);
            }
        }
        
        instance.association = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<Association>(&new_ref_id) {
                if !new_target.member_end.contains(&instance_id) {
                    new_target.member_end.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "Association", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_association(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.association.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.association {
            if let Some(mut old_target) = registry::get::<Association>(&old_ref_id) {
                old_target.member_end.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Association", &old_target);
            }
        }
        
        instance.association = None;
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn add_qualifier(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.qualifier.contains(&ref_id) {
            return Ok(false);
        }
        
        instance.qualifier.push(ref_id.clone());
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<Property>(&ref_id) {
            target.association_end = Some(instance_id.clone());
            let _ = registry::insert(ref_id.clone(), "Property", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn remove_qualifier(instance_id: String, ref_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let initial_len = instance.qualifier.len();
        instance.qualifier.retain(|id| id != &ref_id);
        let removed = instance.qualifier.len() < initial_len;
        
        if !removed {
            return Ok(false);
        }
        
        // Update opposite reference
        if let Some(mut target) = registry::get::<Property>(&ref_id) {
            target.association_end = None;
            let _ = registry::insert(ref_id.clone(), "Property", &target);
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(true)
    }

    #[wasm_bindgen]
    pub fn clear_qualifier(instance_id: String) -> Result<usize, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        let count = instance.qualifier.len();
        
        if count == 0 {
            return Ok(0);
        }
        
        for ref_id in &instance.qualifier {
        // Update opposite reference
        if let Some(mut target) = registry::get::<Property>(&ref_id) {
            target.association_end = None;
            let _ = registry::insert(ref_id.clone(), "Property", &target);
        }
        }
        
        instance.qualifier.clear();
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(count)
    }

    #[wasm_bindgen]
    pub fn set_association_end(instance_id: String, ref_id: Option<String>) -> Result<(), JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if let Some(old_ref_id) = &instance.association_end {
            if let Some(mut old_target) = registry::get::<Property>(&old_ref_id) {
                old_target.qualifier.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Property", &old_target);
            }
        }
        
        instance.association_end = ref_id.clone();
        
        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry::get::<Property>(&new_ref_id) {
                if !new_target.qualifier.contains(&instance_id) {
                    new_target.qualifier.push(instance_id.clone());
                }
                let _ = registry::insert(new_ref_id.clone(), "Property", &new_target);
            }
        }
        
        registry::insert(instance_id, TYPE_NAME, &instance)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn clear_association_end(instance_id: String) -> Result<bool, JsValue> {
        let mut instance: Self = registry::get(&instance_id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))?;
        
        if instance.association_end.is_none() {
            return Ok(false);
        }
        
        if let Some(old_ref_id) = &instance.association_end {
            if let Some(mut old_target) = registry::get::<Property>(&old_ref_id) {
                old_target.qualifier.retain(|id| id != &instance_id);
                let _ = registry::insert(old_ref_id.clone(), "Property", &old_target);
            }
        }
        
        instance.association_end = None;
        
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
        "Property".to_string()
    }


}
