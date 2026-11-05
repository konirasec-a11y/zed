// DataType - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::operation::Operation;
use crate::ruml::property::Property;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "DataType";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct DataType {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub element_import: Vec<String>,

    pub package_import: Vec<String>,

    pub owned_rule: Vec<String>,

    pub is_leaf: String,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub template_binding: Vec<String>,

    pub owned_template_signature: Option<String>,

    pub is_abstract: String,

    pub generalization: Vec<String>,

    pub powertype_extent: Vec<String>,

    pub redefined_classifier: Vec<String>,

    pub substitution: Vec<String>,

    pub representation: Option<String>,

    pub collaboration_use: Vec<String>,

    pub owned_use_case: Vec<String>,

    pub use_case: Vec<String>,

    pub owned_attribute: Vec<String>,

    pub owned_operation: Vec<String>,
}

impl DataType {
    /// Creates a new DataType instance

    pub fn create(
        registry: &mut Registry,
        is_leaf: String,
        is_abstract: String,
    ) -> Result<String, String> {
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
            owned_attribute: Vec::new(),
            owned_operation: Vec::new(),
        };

        registry.insert(id.clone(), TYPE_NAME, &instance)?;

        Ok(id)
    }

    /// Gets a snapshot of this instance
    /// Note: Returns a snapshot. Modifications require calling update().

    pub fn get(registry: &Registry, id: String) -> Result<String, String> {
        registry.get_as_string_of_type(&id, TYPE_NAME)
    }

    /// Updates the instance in the registry

    pub fn update(registry: &mut Registry, value: String) -> Result<(), String> {
        let instance: DataType =
            serde_json::from_str(&value).map_err(|_| String::from("Invalid data"))?;
        registry.update_from_string(
            instance.id.clone(),
            TYPE_NAME,
            serde_json::to_string(&instance).map_err(|_| String::from("Invalid data"))?,
        )
    }

    /// Deletes this instance from the registry

    pub fn delete(registry: &mut Registry, id: String) -> bool {
        registry.delete_of_type(&id, TYPE_NAME)
    }

    /// Checks if an instance exists

    pub fn exists(registry: &Registry, id: String) -> bool {
        registry.exists_of_type(&id, TYPE_NAME)
    }

    /// Gets all instances of this type

    pub fn get_all(registry: &Registry) -> Result<String, String> {
        registry.get_all_of_type_as_string(TYPE_NAME)
    }

    /// Clears all instances of this type

    pub fn clear_all(registry: &mut Registry) -> usize {
        registry.clear_type(TYPE_NAME)
    }

    pub fn set_name(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.name = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_name(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.name = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_visibility(
        registry: &mut Registry,
        instance_id: String,
        value: VisibilityKind,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.visibility = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_visibility(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.visibility = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_leaf(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_leaf = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_abstract(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_abstract = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn add_owned_attribute(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.owned_attribute.contains(&ref_id) {
                return Ok(false);
            }

            instance.owned_attribute.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Property>(&ref_id) {
                target.datatype = Some(instance_id.clone());
                registry.insert(ref_id.clone(), "Property", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_owned_attribute(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.owned_attribute.len();
            instance.owned_attribute.retain(|id| id != &ref_id);
            let removed = instance.owned_attribute.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<Property>(&ref_id) {
                target.datatype = None;
                registry.insert(ref_id.clone(), "Property", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_owned_attribute(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.owned_attribute.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.owned_attribute {
                // Update opposite reference
                if let Some(mut target) = registry.get::<Property>(&ref_id) {
                    target.datatype = None;
                    registry.insert(ref_id.clone(), "Property", &target)?;
                }
            }

            instance.owned_attribute.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_owned_operation(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.owned_operation.contains(&ref_id) {
                return Ok(false);
            }

            instance.owned_operation.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Operation>(&ref_id) {
                target.datatype = Some(instance_id.clone());
                registry.insert(ref_id.clone(), "Operation", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_owned_operation(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.owned_operation.len();
            instance.owned_operation.retain(|id| id != &ref_id);
            let removed = instance.owned_operation.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<Operation>(&ref_id) {
                target.datatype = None;
                registry.insert(ref_id.clone(), "Operation", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_owned_operation(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.owned_operation.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.owned_operation {
                // Update opposite reference
                if let Some(mut target) = registry.get::<Operation>(&ref_id) {
                    target.datatype = None;
                    registry.insert(ref_id.clone(), "Operation", &target)?;
                }
            }

            instance.owned_operation.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
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
        "DataType".to_string()
    }
}
