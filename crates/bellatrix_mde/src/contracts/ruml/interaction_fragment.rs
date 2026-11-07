// InteractionFragment - Generated UML Class
// Uses unified registry with type filtering

use crate::contracts::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::contracts::ruml::interaction::Interaction;
use crate::contracts::ruml::interaction_operand::InteractionOperand;
use crate::contracts::ruml::lifeline::Lifeline;
use crate::contracts::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "InteractionFragment";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct InteractionFragment {
    /// Unique identifier for this instance
    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub covered: Vec<String>,

    pub general_ordering: Vec<String>,

    pub enclosing_interaction: Option<String>,

    pub enclosing_operand: Option<String>,
}

impl InteractionFragment {
    /// Creates a new InteractionFragment instance

    pub fn create(registry: &mut Registry) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let instance = Self {
            id: id.clone(),
            e_annotations: Vec::new(),
            owned_comment: Vec::new(),
            name: None,
            visibility: None,
            client_dependency: Vec::new(),
            name_expression: None,
            covered: Vec::new(),
            general_ordering: Vec::new(),
            enclosing_interaction: None,
            enclosing_operand: None,
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
        let instance: InteractionFragment =
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

    pub fn add_covered(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.covered.contains(&ref_id) {
                return Ok(false);
            }

            instance.covered.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Lifeline>(&ref_id) {
                if !target.covered_by.contains(&instance_id) {
                    target.covered_by.push(instance_id.clone());
                }
                registry.insert(ref_id.clone(), "Lifeline", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_covered(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let initial_len = instance.covered.len();
            instance.covered.retain(|id| id != &ref_id);
            let removed = instance.covered.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<Lifeline>(&ref_id) {
                target.covered_by.retain(|id| id != &instance_id);
                registry.insert(ref_id.clone(), "Lifeline", &target)?;
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_covered(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            let count = instance.covered.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.covered {
                // Update opposite reference
                if let Some(mut target) = registry.get::<Lifeline>(&ref_id) {
                    target.covered_by.retain(|id| id != &instance_id);
                    registry.insert(ref_id.clone(), "Lifeline", &target)?;
                }
            }

            instance.covered.clear();

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(count)
        })
    }

    pub fn add_general_ordering(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.general_ordering.contains(&ref_id) {
            return Ok(false);
        }

        instance.general_ordering.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_general_ordering(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.general_ordering.len();
        instance.general_ordering.retain(|id| id != &ref_id);
        let removed = instance.general_ordering.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_general_ordering(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.general_ordering.len();

        if count == 0 {
            return Ok(0);
        }

        instance.general_ordering.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn set_enclosing_interaction(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if let Some(old_ref_id) = &instance.enclosing_interaction {
                if let Some(mut old_target) = registry.get::<Interaction>(&old_ref_id) {
                    old_target.fragment.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "Interaction", &old_target)?;
                }
            }

            instance.enclosing_interaction = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<Interaction>(&new_ref_id) {
                    if !new_target.fragment.contains(&instance_id) {
                        new_target.fragment.push(instance_id.clone());
                    }
                    registry.insert(new_ref_id.clone(), "Interaction", &new_target)?;
                }
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(())
        })
    }

    pub fn clear_enclosing_interaction(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.enclosing_interaction.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.enclosing_interaction {
                if let Some(mut old_target) = registry.get::<Interaction>(&old_ref_id) {
                    old_target.fragment.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "Interaction", &old_target)?;
                }
            }

            instance.enclosing_interaction = None;

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
        })
    }

    pub fn set_enclosing_operand(
        registry: &mut Registry,
        instance_id: String,
        ref_id: Option<String>,
    ) -> Result<(), String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if let Some(old_ref_id) = &instance.enclosing_operand {
                if let Some(mut old_target) = registry.get::<InteractionOperand>(&old_ref_id) {
                    old_target.fragment.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "InteractionOperand", &old_target)?;
                }
            }

            instance.enclosing_operand = ref_id.clone();

            if let Some(new_ref_id) = &ref_id {
                if let Some(mut new_target) = registry.get::<InteractionOperand>(&new_ref_id) {
                    if !new_target.fragment.contains(&instance_id) {
                        new_target.fragment.push(instance_id.clone());
                    }
                    registry.insert(new_ref_id.clone(), "InteractionOperand", &new_target)?;
                }
            }

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(())
        })
    }

    pub fn clear_enclosing_operand(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;
            let original = instance.clone();

            if instance.enclosing_operand.is_none() {
                return Ok(false);
            }

            if let Some(old_ref_id) = &instance.enclosing_operand {
                if let Some(mut old_target) = registry.get::<InteractionOperand>(&old_ref_id) {
                    old_target.fragment.retain(|id| id != &instance_id);
                    registry.insert(old_ref_id.clone(), "InteractionOperand", &old_target)?;
                }
            }

            instance.enclosing_operand = None;

            registry.insert_changes(instance_id, TYPE_NAME, &original, &instance)?;

            Ok(true)
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
        "InteractionFragment".to_string()
    }
}
