// ElementImport - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::namespace::Namespace;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "ElementImport";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ElementImport {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub visibility: VisibilityKind,

    pub alias: Option<String>,

    pub imported_element: String,

    pub importing_namespace: String,
}

impl ElementImport {
    /// Creates a new ElementImport instance

    pub fn create(
        registry: &mut Registry,
        visibility: VisibilityKind,
        imported_element: String,
        importing_namespace: String,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            visibility,
            alias: None,
            imported_element,
            importing_namespace,
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
        let instance: ElementImport =
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

    pub fn set_visibility(
        registry: &mut Registry,
        instance_id: String,
        value: VisibilityKind,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.visibility = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_alias(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.alias = Some(value);
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn clear_alias(registry: &mut Registry, instance_id: String) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.alias = None;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_imported_element(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.imported_element = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn set_importing_namespace(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let old_ref_id = instance.importing_namespace.clone();
            if let Some(mut old_target) = registry.get::<Namespace>(&old_ref_id) {
                old_target.element_import.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "Namespace", &old_target)?;
            }

            instance.importing_namespace = ref_id.clone();

            if let Some(mut new_target) = registry.get::<Namespace>(&ref_id) {
                if !new_target.element_import.contains(&instance_id) {
                    new_target.element_import.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "Namespace", &new_target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(())
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
        "ElementImport".to_string()
    }
}
