// UseCase - Generated UML Class
// Uses unified registry with type filtering

use crate::contracts::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::contracts::ruml::classifier::Classifier;
use crate::contracts::ruml::extend::Extend;
use crate::contracts::ruml::extension_point::ExtensionPoint;
use crate::contracts::ruml::include::Include;
use crate::contracts::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "UseCase";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct UseCase {
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

    pub owned_behavior: Vec<String>,

    pub classifier_behavior: Option<String>,

    pub interface_realization: Vec<String>,

    pub owned_trigger: Vec<String>,

    pub include: Vec<String>,

    pub extend: Vec<String>,

    pub extension_point: Vec<String>,

    pub subject: Vec<String>,
}

impl UseCase {
    /// Creates a new UseCase instance

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
            owned_behavior: Vec::new(),
            classifier_behavior: None,
            interface_realization: Vec::new(),
            owned_trigger: Vec::new(),
            include: Vec::new(),
            extend: Vec::new(),
            extension_point: Vec::new(),
            subject: Vec::new(),
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
        let instance: UseCase =
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

    pub fn add_include(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.include.contains(&ref_id) {
                return Ok(false);
            }

            instance.include.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Include>(&ref_id) {
                target.including_case = instance_id.clone();
                registry.insert(ref_id.clone(), "Include", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_include(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let initial_len = instance.include.len();
            instance.include.retain(|id| id != &ref_id);
            let removed = instance.include.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<Include>(&ref_id) {
                registry.insert(ref_id.clone(), "Include", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_include(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let count = instance.include.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.include {
                // Update opposite reference
                if let Some(target) = registry.get::<Include>(&ref_id) {
                    registry.insert(ref_id.clone(), "Include", &target)?;
                }
            }

            instance.include.clear();

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(count)
        })
    }

    pub fn add_extend(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.extend.contains(&ref_id) {
                return Ok(false);
            }

            instance.extend.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Extend>(&ref_id) {
                target.extension = instance_id.clone();
                registry.insert(ref_id.clone(), "Extend", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_extend(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let initial_len = instance.extend.len();
            instance.extend.retain(|id| id != &ref_id);
            let removed = instance.extend.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<Extend>(&ref_id) {
                registry.insert(ref_id.clone(), "Extend", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_extend(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let count = instance.extend.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.extend {
                // Update opposite reference
                if let Some(target) = registry.get::<Extend>(&ref_id) {
                    registry.insert(ref_id.clone(), "Extend", &target)?;
                }
            }

            instance.extend.clear();

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(count)
        })
    }

    pub fn add_extension_point(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.extension_point.contains(&ref_id) {
                return Ok(false);
            }

            instance.extension_point.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<ExtensionPoint>(&ref_id) {
                target.use_case = instance_id.clone();
                registry.insert(ref_id.clone(), "ExtensionPoint", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_extension_point(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let initial_len = instance.extension_point.len();
            instance.extension_point.retain(|id| id != &ref_id);
            let removed = instance.extension_point.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<ExtensionPoint>(&ref_id) {
                registry.insert(ref_id.clone(), "ExtensionPoint", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_extension_point(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let count = instance.extension_point.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.extension_point {
                // Update opposite reference
                if let Some(target) = registry.get::<ExtensionPoint>(&ref_id) {
                    registry.insert(ref_id.clone(), "ExtensionPoint", &target)?;
                }
            }

            instance.extension_point.clear();

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(count)
        })
    }

    pub fn add_subject(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.subject.contains(&ref_id) {
                return Ok(false);
            }

            instance.subject.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Classifier>(&ref_id) {
                if !target.use_case.contains(&instance_id) {
                    target.use_case.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "Classifier", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_subject(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let initial_len = instance.subject.len();
            instance.subject.retain(|id| id != &ref_id);
            let removed = instance.subject.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<Classifier>(&ref_id) {
                target.use_case.retain(|id| id != &instance_id);
                registry.insert(ref_id.clone(), "Classifier", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_subject(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let count = instance.subject.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.subject {
                // Update opposite reference
                if let Some(mut target) = registry.get::<Classifier>(&ref_id) {
                    target.use_case.retain(|id| id != &instance_id);
                    registry.insert(ref_id.clone(), "Classifier", &target)?;
                }
            }

            instance.subject.clear();

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

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
        "UseCase".to_string()
    }
}
