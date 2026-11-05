// ActivityPartition - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::activity_edge::ActivityEdge;
use crate::ruml::activity_node::ActivityNode;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "ActivityPartition";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ActivityPartition {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub in_activity: Option<String>,

    pub is_dimension: String,

    pub is_external: String,

    pub node: Vec<String>,

    pub subpartition: Vec<String>,

    pub super_partition: Option<String>,

    pub represents: Option<String>,

    pub edge: Vec<String>,
}

impl ActivityPartition {
    /// Creates a new ActivityPartition instance

    pub fn create(
        registry: &mut Registry,
        is_dimension: String,
        is_external: String,
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
            in_activity: None,
            is_dimension,
            is_external,
            node: Vec::new(),
            subpartition: Vec::new(),
            super_partition: None,
            represents: None,
            edge: Vec::new(),
        };

        registry.insert(id.clone(), TYPE_NAME, &instance)?;

        Ok(id)
    }

    /// Gets a snapshot of this instance
    /// Note: Returns a snapshot. Modifications require calling update().

    pub fn get(registry: &Registry, id: String) -> Result<String, String> {
        registry.get_as_string(&id)
    }

    /// Updates the instance in the registry

    pub fn update(registry: &mut Registry, value: String) -> Result<(), String> {
        let instance: ActivityPartition =
            serde_json::from_str(&value).map_err(|_| String::from("Invalid data"))?;
        registry.update_from_string(
            instance.id.clone(),
            TYPE_NAME,
            serde_json::to_string(&instance).map_err(|_| String::from("Invalid data"))?,
        )
    }

    /// Deletes this instance from the registry

    pub fn delete(registry: &mut Registry, id: String) -> bool {
        registry.delete(&id)
    }

    /// Checks if an instance exists

    pub fn exists(registry: &Registry, id: String) -> bool {
        registry.exists(&id)
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

    pub fn set_is_dimension(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_dimension = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_external(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_external = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn add_node(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.node.contains(&ref_id) {
            return Ok(false);
        }

        instance.node.push(ref_id.clone());

        // Update opposite reference
        if let Some(mut target) = registry.get::<ActivityNode>(&ref_id) {
            if !target.in_partition.contains(&instance_id) {
                target.in_partition.push(instance_id.clone());
            }
            registry.insert(ref_id.clone(), "ActivityNode", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_node(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.node.len();
        instance.node.retain(|id| id != &ref_id);
        let removed = instance.node.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        // Update opposite reference
        if let Some(mut target) = registry.get::<ActivityNode>(&ref_id) {
            target.in_partition.retain(|id| id != &instance_id);
            registry.insert(ref_id.clone(), "ActivityNode", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_node(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.node.len();

        if count == 0 {
            return Ok(0);
        }

        for ref_id in &instance.node {
            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityNode>(&ref_id) {
                target.in_partition.retain(|id| id != &instance_id);
                registry.insert(ref_id.clone(), "ActivityNode", &target)?;
            }
        }

        instance.node.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_subpartition(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.subpartition.contains(&ref_id) {
            return Ok(false);
        }

        instance.subpartition.push(ref_id.clone());

        // Update opposite reference
        if let Some(mut target) = registry.get::<ActivityPartition>(&ref_id) {
            target.super_partition = Some(instance_id.clone());
            registry.insert(ref_id.clone(), "ActivityPartition", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_subpartition(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.subpartition.len();
        instance.subpartition.retain(|id| id != &ref_id);
        let removed = instance.subpartition.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        // Update opposite reference
        if let Some(mut target) = registry.get::<ActivityPartition>(&ref_id) {
            target.super_partition = None;
            registry.insert(ref_id.clone(), "ActivityPartition", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_subpartition(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.subpartition.len();

        if count == 0 {
            return Ok(0);
        }

        for ref_id in &instance.subpartition {
            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityPartition>(&ref_id) {
                target.super_partition = None;
                registry.insert(ref_id.clone(), "ActivityPartition", &target)?;
            }
        }

        instance.subpartition.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn set_super_partition(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if let Some(old_ref_id) = &instance.super_partition {
            if let Some(mut old_target) = registry.get::<ActivityPartition>(&old_ref_id) {
                old_target.subpartition.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "ActivityPartition", &old_target)?;
            }
        }

        instance.super_partition = ref_id.clone();

        if let Some(new_ref_id) = &ref_id {
            if let Some(mut new_target) = registry.get::<ActivityPartition>(&new_ref_id) {
                if !new_target.subpartition.contains(&instance_id) {
                    new_target.subpartition.push(instance_id.clone());
                }
                registry.insert(new_ref_id.clone(), "ActivityPartition", &new_target)?;
            }
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_super_partition(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.super_partition.is_none() {
            return Ok(false);
        }

        if let Some(old_ref_id) = &instance.super_partition {
            if let Some(mut old_target) = registry.get::<ActivityPartition>(&old_ref_id) {
                old_target.subpartition.retain(|id| id != &instance_id);
                registry.insert(old_ref_id.clone(), "ActivityPartition", &old_target)?;
            }
        }

        instance.super_partition = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn set_represents(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        instance.represents = ref_id.clone();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(())
    }

    pub fn clear_represents(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.represents.is_none() {
            return Ok(false);
        }

        instance.represents = None;

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn add_edge(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.edge.contains(&ref_id) {
            return Ok(false);
        }

        instance.edge.push(ref_id.clone());

        // Update opposite reference
        if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
            if !target.in_partition.contains(&instance_id) {
                target.in_partition.push(instance_id.clone());
            }
            registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_edge(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.edge.len();
        instance.edge.retain(|id| id != &ref_id);
        let removed = instance.edge.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        // Update opposite reference
        if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
            target.in_partition.retain(|id| id != &instance_id);
            registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_edge(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.edge.len();

        if count == 0 {
            return Ok(0);
        }

        for ref_id in &instance.edge {
            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
                target.in_partition.retain(|id| id != &instance_id);
                registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
            }
        }

        instance.edge.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
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
        "ActivityPartition".to_string()
    }
}
