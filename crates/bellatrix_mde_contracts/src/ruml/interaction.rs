// Interaction - Generated UML Class
// Uses unified registry with type filtering

use crate::ruml::registry::Registry;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::ruml::interaction_fragment::InteractionFragment;
use crate::ruml::lifeline::Lifeline;
use crate::ruml::message::Message;
use crate::ruml::visibility_kind::VisibilityKind;

const TYPE_NAME: &str = "Interaction";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct Interaction {
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

    pub owned_connector: Vec<String>,

    pub owned_behavior: Vec<String>,

    pub classifier_behavior: Option<String>,

    pub interface_realization: Vec<String>,

    pub owned_trigger: Vec<String>,

    pub nested_classifier: Vec<String>,

    pub owned_operation: Vec<String>,

    pub is_active: String,

    pub owned_reception: Vec<String>,

    pub is_reentrant: String,

    pub redefined_behavior: Vec<String>,

    pub owned_parameter: Vec<String>,

    pub precondition: Vec<String>,

    pub postcondition: Vec<String>,

    pub owned_parameter_set: Vec<String>,

    pub specification: Option<String>,

    pub covered: Vec<String>,

    pub general_ordering: Vec<String>,

    pub enclosing_interaction: Option<String>,

    pub enclosing_operand: Option<String>,

    pub lifeline: Vec<String>,

    pub fragment: Vec<String>,

    pub action: Vec<String>,

    pub formal_gate: Vec<String>,

    pub message: Vec<String>,
}

impl Interaction {
    /// Creates a new Interaction instance

    pub fn create(
        registry: &mut Registry,
        is_leaf: String,
        is_abstract: String,
        is_active: String,
        is_reentrant: String,
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
            owned_connector: Vec::new(),
            owned_behavior: Vec::new(),
            classifier_behavior: None,
            interface_realization: Vec::new(),
            owned_trigger: Vec::new(),
            nested_classifier: Vec::new(),
            owned_operation: Vec::new(),
            is_active,
            owned_reception: Vec::new(),
            is_reentrant,
            redefined_behavior: Vec::new(),
            owned_parameter: Vec::new(),
            precondition: Vec::new(),
            postcondition: Vec::new(),
            owned_parameter_set: Vec::new(),
            specification: None,
            covered: Vec::new(),
            general_ordering: Vec::new(),
            enclosing_interaction: None,
            enclosing_operand: None,
            lifeline: Vec::new(),
            fragment: Vec::new(),
            action: Vec::new(),
            formal_gate: Vec::new(),
            message: Vec::new(),
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
        let instance: Interaction =
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

    pub fn set_is_active(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_active = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn set_is_reentrant(
        registry: &mut Registry,
        instance_id: String,
        value: String,
    ) -> Result<(), String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;
        instance.is_reentrant = value;
        registry.insert(instance_id, TYPE_NAME, &instance)?;
        Ok(())
    }

    pub fn add_lifeline(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.lifeline.contains(&ref_id) {
                return Ok(false);
            }

            instance.lifeline.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Lifeline>(&ref_id) {
                target.interaction = instance_id.clone();
                registry.insert(ref_id.clone(), "Lifeline", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_lifeline(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.lifeline.len();
            instance.lifeline.retain(|id| id != &ref_id);
            let removed = instance.lifeline.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<Lifeline>(&ref_id) {
                registry.insert(ref_id.clone(), "Lifeline", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_lifeline(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.lifeline.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.lifeline {
                // Update opposite reference
                if let Some(target) = registry.get::<Lifeline>(&ref_id) {
                    registry.insert(ref_id.clone(), "Lifeline", &target)?;
                }
            }

            instance.lifeline.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_fragment(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.fragment.contains(&ref_id) {
                return Ok(false);
            }

            instance.fragment.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<InteractionFragment>(&ref_id) {
                target.enclosing_interaction = Some(instance_id.clone());
                registry.insert(ref_id.clone(), "InteractionFragment", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_fragment(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.fragment.len();
            instance.fragment.retain(|id| id != &ref_id);
            let removed = instance.fragment.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(mut target) = registry.get::<InteractionFragment>(&ref_id) {
                target.enclosing_interaction = None;
                registry.insert(ref_id.clone(), "InteractionFragment", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_fragment(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.fragment.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.fragment {
                // Update opposite reference
                if let Some(mut target) = registry.get::<InteractionFragment>(&ref_id) {
                    target.enclosing_interaction = None;
                    registry.insert(ref_id.clone(), "InteractionFragment", &target)?;
                }
            }

            instance.fragment.clear();

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(count)
        })
    }

    pub fn add_action(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.action.contains(&ref_id) {
            return Ok(false);
        }

        instance.action.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_action(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.action.len();
        instance.action.retain(|id| id != &ref_id);
        let removed = instance.action.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_action(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.action.len();

        if count == 0 {
            return Ok(0);
        }

        instance.action.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_formal_gate(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        if instance.formal_gate.contains(&ref_id) {
            return Ok(false);
        }

        instance.formal_gate.push(ref_id.clone());

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn remove_formal_gate(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let initial_len = instance.formal_gate.len();
        instance.formal_gate.retain(|id| id != &ref_id);
        let removed = instance.formal_gate.len() < initial_len;

        if !removed {
            return Ok(false);
        }

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(true)
    }

    pub fn clear_formal_gate(
        registry: &mut Registry,
        instance_id: String,
    ) -> Result<usize, String> {
        let mut instance: Self = registry
            .get(&instance_id)
            .ok_or_else(|| String::from("Instance not found"))?;

        let count = instance.formal_gate.len();

        if count == 0 {
            return Ok(0);
        }

        instance.formal_gate.clear();

        registry.insert(instance_id, TYPE_NAME, &instance)?;

        Ok(count)
    }

    pub fn add_message(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            if instance.message.contains(&ref_id) {
                return Ok(false);
            }

            instance.message.push(ref_id.clone());

            // Update opposite reference
            if let Some(mut target) = registry.get::<Message>(&ref_id) {
                target.interaction = instance_id.clone();
                registry.insert(ref_id.clone(), "Message", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn remove_message(
        registry: &mut Registry,
        instance_id: String,
        ref_id: String,
    ) -> Result<bool, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let initial_len = instance.message.len();
            instance.message.retain(|id| id != &ref_id);
            let removed = instance.message.len() < initial_len;

            if !removed {
                return Ok(false);
            }

            // Update opposite reference
            if let Some(target) = registry.get::<Message>(&ref_id) {
                registry.insert(ref_id.clone(), "Message", &target)?;
            }

            registry.insert(instance_id, TYPE_NAME, &instance)?;

            Ok(true)
        })
    }

    pub fn clear_message(registry: &mut Registry, instance_id: String) -> Result<usize, String> {
        registry.transaction(|registry| {
            let mut instance: Self = registry
                .get(&instance_id)
                .ok_or_else(|| String::from("Instance not found"))?;

            let count = instance.message.len();

            if count == 0 {
                return Ok(0);
            }

            for ref_id in &instance.message {
                // Update opposite reference
                if let Some(target) = registry.get::<Message>(&ref_id) {
                    registry.insert(ref_id.clone(), "Message", &target)?;
                }
            }

            instance.message.clear();

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
        "Interaction".to_string()
    }
}
