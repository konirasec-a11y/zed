// ActivityNode - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::activity::Activity;
use crate::ruml::activity_edge::ActivityEdge;
use crate::ruml::activity_partition::ActivityPartition;
use crate::ruml::interruptible_activity_region::InterruptibleActivityRegion;
use crate::ruml::structured_activity_node::StructuredActivityNode;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "ActivityNode";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ActivityNode {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub is_leaf: String,

    pub in_structured_node: Option<String>,

    pub activity: Option<String>,

    pub outgoing: Vec<String>,

    pub incoming: Vec<String>,

    pub in_partition: Vec<String>,

    pub in_interruptible_region: Vec<String>,

    pub redefined_node: Vec<String>,
}

impl ActivityNode {
    /// Creates a new ActivityNode instance

    pub fn create(registry: &mut Registry, is_leaf: String) -> Result<String, String> {
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
            in_structured_node: None,
            activity: None,
            outgoing: Vec::new(),
            incoming: Vec::new(),
            in_partition: Vec::new(),
            in_interruptible_region: Vec::new(),
            redefined_node: Vec::new(),
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
        let instance: ActivityNode =
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

    pub fn set_in_structured_node(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if let Some(old_ref_id) = &instance.in_structured_node {
                if let Some(mut old_target) = registry.get::<StructuredActivityNode>(&old_ref_id) {
                    old_target.node.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "StructuredActivityNode", &old_target)?;
                }
            }

            instance.in_structured_node = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<StructuredActivityNode>(&new_ref_id) {
                    if !new_target.node.contains(&instance_id) {
                        new_target.node.push(instance_id.clone());
                    }
                    registry.insert(new_ref_id.clone(), "StructuredActivityNode", &new_target)?;
                }
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(())
        })
    }

    pub fn clear_in_structured_node(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.in_structured_node.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.in_structured_node {
                if let Some(mut old_target) = registry.get::<StructuredActivityNode>(&old_ref_id) {
                    old_target.node.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "StructuredActivityNode", &old_target)?;
                }
            }

            instance.in_structured_node = None;

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn set_activity(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if let Some(old_ref_id) = &instance.activity {
                if let Some(mut old_target) = registry.get::<Activity>(&old_ref_id) {
                    old_target.node.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "Activity", &old_target)?;
                }
            }

            instance.activity = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<Activity>(&new_ref_id) {
                    if !new_target.node.contains(&instance_id) {
                        new_target.node.push(instance_id.clone());
                    }
                    registry.insert(new_ref_id.clone(), "Activity", &new_target)?;
                }
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(())
        })
    }

    pub fn clear_activity(registry: &mut Registry, instance_id: String) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.activity.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.activity {
                if let Some(mut old_target) = registry.get::<Activity>(&old_ref_id) {
                    old_target.node.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "Activity", &old_target)?;
                }
            }

            instance.activity = None;

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn add_outgoing(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.outgoing.contains(&ref_id) {
                return Ok(false);
            }

            instance.outgoing.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
                target.source = instance_id.clone();
                registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_outgoing(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.outgoing.len();
            instance.outgoing.retain(|id| id != &ref_id);
            let removed = instance.outgoing.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<ActivityEdge>(&ref_id) {
                registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_outgoing(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.outgoing.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.outgoing {
                // Update opposite reference
                if let Some(target) = registry.get::<ActivityEdge>(&ref_id) {
                    registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
                }
            }

            instance.outgoing.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_incoming(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.incoming.contains(&ref_id) {
                return Ok(false);
            }

            instance.incoming.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityEdge>(&ref_id) {
                target.target = instance_id.clone();
                registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_incoming(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.incoming.len();
            instance.incoming.retain(|id| id != &ref_id);
            let removed = instance.incoming.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<ActivityEdge>(&ref_id) {
                registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_incoming(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.incoming.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.incoming {
                // Update opposite reference
                if let Some(target) = registry.get::<ActivityEdge>(&ref_id) {
                    registry.insert(ref_id.clone(), "ActivityEdge", &target)?;
                }
            }

            instance.incoming.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_in_partition(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.in_partition.contains(&ref_id) {
                return Ok(false);
            }

            instance.in_partition.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityPartition>(&ref_id) {
                if !target.node.contains(&instance_id) {
                    target.node.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "ActivityPartition", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_in_partition(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.in_partition.len();
            instance.in_partition.retain(|id| id != &ref_id);
            let removed = instance.in_partition.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<ActivityPartition>(&ref_id) {
                target.node.retain(|id| id != &instance_id);
                registry.insert(ref_id.clone(), "ActivityPartition", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_in_partition(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.in_partition.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.in_partition {
                // Update opposite reference
                if let Some(mut target) = registry.get::<ActivityPartition>(&ref_id) {
                    target.node.retain(|id| id != &instance_id);
                    registry.insert(ref_id.clone(), "ActivityPartition", &target)?;
                }
            }

            instance.in_partition.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_in_interruptible_region(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.in_interruptible_region.contains(&ref_id) {
                return Ok(false);
            }

            instance.in_interruptible_region.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<InterruptibleActivityRegion>(&ref_id) {
                if !target.node.contains(&instance_id) {
                    target.node.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "InterruptibleActivityRegion", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_in_interruptible_region(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.in_interruptible_region.len();
            instance.in_interruptible_region.retain(|id| id != &ref_id);
            let removed = instance.in_interruptible_region.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<InterruptibleActivityRegion>(&ref_id) {
                target.node.retain(|id| id != &instance_id);
                registry.insert(ref_id.clone(), "InterruptibleActivityRegion", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_in_interruptible_region(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.in_interruptible_region.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.in_interruptible_region {
                // Update opposite reference
                if let Some(mut target) = registry.get::<InterruptibleActivityRegion>(&ref_id) {
                    target.node.retain(|id| id != &instance_id);
                    registry.insert(ref_id.clone(), "InterruptibleActivityRegion", &target)?;
                }
            }

            instance.in_interruptible_region.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_redefined_node(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.redefined_node.contains(&ref_id) {
            return Ok(false);
        }

        instance.redefined_node.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_redefined_node(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.redefined_node.len();
        instance.redefined_node.retain(|id| id != &ref_id);
        let removed = instance.redefined_node.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_redefined_node(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.redefined_node.len();

        if count == 0 {
            return Ok(0);
        }

        instance.redefined_node.clear();

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
        "ActivityNode".to_string()
    }
}
