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

// Original module: invocation_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationAction {
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
}

// Original module: call_operation_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallOperationAction {
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
    pub operation: String,
    pub target: String,
}

// Original module: call_behavior_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallBehaviorAction {
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
    pub behavior: String,
}

// Original module: send_signal_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSignalAction {
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
    pub target: String,
    pub signal: String,
}

// Original module: create_object_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateObjectAction {
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
    pub classifier: String,
    pub result: String,
}

// Original module: destroy_object_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestroyObjectAction {
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
    pub is_destroy_links: String,
    pub is_destroy_owned_objects: String,
    pub target: String,
}

// Original module: test_identity_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestIdentityAction {
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
    pub first: String,
    pub second: String,
    pub result: String,
}

// Original module: read_self_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadSelfAction {
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
    pub result: String,
}

// Original module: structural_feature_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralFeatureAction {
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
    pub structural_feature: String,
    pub object: String,
}

// Original module: read_structural_feature_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadStructuralFeatureAction {
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
    pub structural_feature: String,
    pub object: String,
    pub result: String,
}

// Original module: write_structural_feature_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteStructuralFeatureAction {
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
    pub structural_feature: String,
    pub object: String,
    pub value: String,
}

// Original module: clear_structural_feature_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearStructuralFeatureAction {
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
    pub structural_feature: String,
    pub object: String,
}

// Original module: remove_structural_feature_value_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveStructuralFeatureValueAction {
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
    pub structural_feature: String,
    pub object: String,
    pub value: String,
    pub is_remove_duplicates: String,
    pub remove_at: Option<String>,
}

// Original module: add_structural_feature_value_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddStructuralFeatureValueAction {
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
    pub structural_feature: String,
    pub object: String,
    pub value: String,
    pub is_replace_all: String,
    pub insert_at: Option<String>,
}

// Original module: link_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkAction {
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
    pub end_data: Vec<String>,
    pub input_value: Vec<String>,
}

// Original module: link_end_data.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEndData {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub value: Option<String>,
    pub end: String,
    pub qualifier: Vec<String>,
}

// Original module: qualifier_value.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualifierValue {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub qualifier: String,
    pub value: String,
}

// Original module: read_link_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadLinkAction {
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
    pub end_data: Vec<String>,
    pub input_value: Vec<String>,
    pub result: String,
}

// Original module: link_end_creation_data.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEndCreationData {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub value: Option<String>,
    pub end: String,
    pub qualifier: Vec<String>,
    pub is_replace_all: String,
    pub insert_at: Option<String>,
}

// Original module: create_link_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLinkAction {
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
    pub end_data: Vec<String>,
    pub input_value: Vec<String>,
}

// Original module: write_link_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteLinkAction {
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
    pub end_data: Vec<String>,
    pub input_value: Vec<String>,
}

// Original module: destroy_link_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestroyLinkAction {
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
    pub end_data: Vec<String>,
    pub input_value: Vec<String>,
}

// Original module: link_end_destruction_data.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEndDestructionData {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub value: Option<String>,
    pub end: String,
    pub qualifier: Vec<String>,
    pub is_destroy_duplicates: String,
    pub destroy_at: Option<String>,
}

// Original module: clear_association_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearAssociationAction {
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
    pub object: String,
    pub association: String,
}

// Original module: broadcast_signal_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastSignalAction {
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
    pub signal: String,
}

// Original module: send_object_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendObjectAction {
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
    pub target: String,
    pub request: String,
}

// Original module: value_specification_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueSpecificationAction {
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
    pub value: String,
    pub result: String,
}

// Original module: variable_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableAction {
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
    pub variable: String,
}

// Original module: read_variable_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadVariableAction {
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
    pub variable: String,
    pub result: String,
}

// Original module: write_variable_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteVariableAction {
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
    pub variable: String,
    pub value: String,
}

// Original module: clear_variable_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearVariableAction {
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
    pub variable: String,
}

// Original module: add_variable_value_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddVariableValueAction {
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
    pub variable: String,
    pub value: String,
    pub is_replace_all: String,
    pub insert_at: Option<String>,
}

// Original module: remove_variable_value_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveVariableValueAction {
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
    pub variable: String,
    pub value: String,
    pub is_remove_duplicates: String,
    pub remove_at: Option<String>,
}

// Original module: raise_exception_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaiseExceptionAction {
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
    pub exception: String,
}

// Original module: read_extent_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadExtentAction {
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
    pub result: String,
    pub classifier: String,
}

// Original module: reclassify_object_action.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReclassifyObjectAction {
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
    pub is_replace_all: String,
    pub old_classifier: Vec<String>,
    pub new_classifier: Vec<String>,
    pub object: String,
}
