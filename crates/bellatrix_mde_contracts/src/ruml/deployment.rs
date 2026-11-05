// Deployment - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::deployment_specification::DeploymentSpecification;
use crate::ruml::deployment_target::DeploymentTarget;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "Deployment";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct Deployment {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub supplier: Vec<String>,

    pub client: Vec<String>,

    pub deployed_artifact: Vec<String>,

    pub configuration: Vec<String>,

    pub location: String,
}

impl Deployment {
    /// Creates a new Deployment instance

    pub fn create(registry: &mut Registry, location: String) -> Result<String, String> {
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
        let instance: Deployment =
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

    pub fn add_deployed_artifact(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.deployed_artifact.contains(&ref_id) {
            return Ok(false);
        }

        instance.deployed_artifact.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_deployed_artifact(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.deployed_artifact.len();
        instance.deployed_artifact.retain(|id| id != &ref_id);
        let removed = instance.deployed_artifact.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_deployed_artifact(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.deployed_artifact.len();

        if count == 0 {
            return Ok(0);
        }

        instance.deployed_artifact.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_configuration(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.configuration.contains(&ref_id) {
                return Ok(false);
            }

            instance.configuration.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<DeploymentSpecification>(&ref_id) {
                target.deployment = Some(instance_id.clone());
                registry.insert(ref_id.clone(), "DeploymentSpecification", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_configuration(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.configuration.len();
            instance.configuration.retain(|id| id != &ref_id);
            let removed = instance.configuration.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<DeploymentSpecification>(&ref_id) {
                target.deployment = None;
                registry.insert(ref_id.clone(), "DeploymentSpecification", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_configuration(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.configuration.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.configuration {
                // Update opposite reference
                if let Some(mut target) = registry.get::<DeploymentSpecification>(&ref_id) {
                    target.deployment = None;
                    registry.insert(ref_id.clone(), "DeploymentSpecification", &target)?;
                }
            }

            instance.configuration.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn set_location(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let old_ref_id = instance.location.clone();
            if let Some(mut old_target) = registry.get::<DeploymentTarget>(&old_ref_id) {
                old_target.deployment.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "DeploymentTarget", &old_target)?;
            }

            instance.location = ref_id.clone();

            if let Some(mut new_target) = registry.get::<DeploymentTarget>(&ref_id) {
                if !new_target.deployment.contains(&instance_id) {
                    new_target.deployment.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "DeploymentTarget", &new_target)?;
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
        "Deployment".to_string()
    }
}
