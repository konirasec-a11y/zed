//! Native UML 2.5 declarations: behavioral/interactions.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use crate::uml::structural::common_structure::VisibilityKind;

// Original module: message.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub message_sort: MessageSort,
    pub receive_event: Option<String>,
    pub send_event: Option<String>,
    pub connector: Option<String>,
    pub interaction: String,
    pub argument: Vec<String>,
}

// Original module: message_end.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEnd {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub message: Option<String>,
}

// Original module: message_event.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEvent {
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
}

// Original module: message_occurrence_specification.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageOccurrenceSpecification {
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
    pub to_before: Vec<String>,
    pub event: String,
    pub to_after: Vec<String>,
    pub message: Option<String>,
}

// Original module: message_kind.rs
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    Complete,
    Lost,
    Found,
    Unknown,
}

// Original module: message_sort.rs
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageSort {
    Synchcall,
    Asynchcall,
    Asynchsignal,
    Createmessage,
    Deletemessage,
    Reply,
}

// Original module: interaction.rs
#[derive(Clone, Serialize, Deserialize)]
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

// Original module: interaction_fragment.rs
#[derive(Clone, Serialize, Deserialize)]
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

// Original module: lifeline.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lifeline {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub represents: Option<String>,
    pub interaction: String,
    pub selector: Option<String>,
    pub decomposed_as: Option<String>,
    pub covered_by: Vec<String>,
}
