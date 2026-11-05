// InterruptibleActivityRegion - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::activity_edge::ActivityEdge;
use crate::ruml::activity_node::ActivityNode;

const TYPE_NAME: &str = "InterruptibleActivityRegion";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct InterruptibleActivityRegion {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub in_activity: Option<String>,

    pub node: Vec<String>,

    pub interrupting_edge: Vec<String>,
}

impl InterruptibleActivityRegion {
    /// Creates a new InterruptibleActivityRegion instance

    pub fn create(registry: &mut Registry) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            in_activity: None,
            node: Vec::new(),
            interrupting_edge: Vec::new(),
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
        let instance: InterruptibleActivityRegion =
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

    pub fn add_node(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.node.contains(&ref_id) {
                return Ok(false);
            }

            instance.node.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityNode>(&ref_id) {
                if !target.in_interruptible_region.contains(&instance_id) {
                    target.in_interruptible_region.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "ActivityNode", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_node(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
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
                target
                    .in_interruptible_region
                    .retain(|id| id != &instance_id);
                registry.insert(ref_id.clone(), "ActivityNode", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_node(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
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
                    target
                        .in_interruptible_region
                        .retain(|id| id != &instance_id);
                    registry.insert(ref_id.clone(), "ActivityNode", &target)?;
                }
            }

            instance.node.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_interrupting_edge(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.interrupting_edge.contains(&ref_id) {
                return Ok(false);
            }

            instance.interrupting_edge.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
                target.interrupts = Some(instance_id.clone());
                registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_interrupting_edge(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.interrupting_edge.len();
            instance.interrupting_edge.retain(|id| id != &ref_id);
            let removed = instance.interrupting_edge.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
                target.interrupts = None;
                registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_interrupting_edge(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.interrupting_edge.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.interrupting_edge {
                // Update opposite reference
                if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
                    target.interrupts = None;
                    registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
                }
            }

            instance.interrupting_edge.clear();

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
        "InterruptibleActivityRegion".to_string()
    }
}
