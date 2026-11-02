//! Native UML 2.5 declarations: behavioral/actions.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use crate::uml::behavioral::activities::ObjectNodeOrderingKind;
use crate::uml::structural::common_structure::VisibilityKind;

// Original module: action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
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
    pub handler: Vec<String>,
    pub local_precondition: Vec<String>,
    pub local_postcondition: Vec<String>,
}

// Original module: executable_node.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableNode {
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
    pub handler: Vec<String>,
}

// Original module: opaque_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpaqueAction {
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
    pub handler: Vec<String>,
    pub local_precondition: Vec<String>,
    pub local_postcondition: Vec<String>,
    pub body: Vec<String>,
    pub language: Vec<String>,
    pub input_value: Vec<String>,
    pub output_value: Vec<String>,
}

// Original module: action_input_pin.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionInputPin {
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
    pub type_: Option<String>,
    pub ordering: ObjectNodeOrderingKind,
    pub is_control_type: String,
    pub upper_bound: String,
    pub in_state: Vec<String>,
    pub selection: Option<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
    pub is_control: String,
    pub from_action: String,
}

// Original module: pin.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
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
    pub type_: Option<String>,
    pub ordering: ObjectNodeOrderingKind,
    pub is_control_type: String,
    pub upper_bound: String,
    pub in_state: Vec<String>,
    pub selection: Option<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
    pub is_control: String,
}

// Original module: input_pin.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputPin {
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
    pub type_: Option<String>,
    pub ordering: ObjectNodeOrderingKind,
    pub is_control_type: String,
    pub upper_bound: String,
    pub in_state: Vec<String>,
    pub selection: Option<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
    pub is_control: String,
}

// Original module: output_pin.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputPin {
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
    pub type_: Option<String>,
    pub ordering: ObjectNodeOrderingKind,
    pub is_control_type: String,
    pub upper_bound: String,
    pub in_state: Vec<String>,
    pub selection: Option<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
    pub is_control: String,
}

// Original module: value_pin.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuePin {
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
    pub type_: Option<String>,
    pub ordering: ObjectNodeOrderingKind,
    pub is_control_type: String,
    pub upper_bound: String,
    pub in_state: Vec<String>,
    pub selection: Option<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
    pub is_control: String,
    pub value: String,
}

// Original module: call_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallAction {
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
    pub handler: Vec<String>,
    pub local_precondition: Vec<String>,
    pub local_postcondition: Vec<String>,
    pub argument: Vec<String>,
    pub on_port: Option<String>,
    pub is_synchronous: String,
    pub result: Vec<String>,
}
