//! Native UML 2.5 declarations: behavioral/state_machines.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::uml::structural::common_structure::VisibilityKind;

// Original module: protocol_state_machine.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolStateMachine {
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
    pub region: Vec<String>,
    pub submachine_state: Vec<String>,
    pub connection_point: Vec<String>,
    pub extended_state_machine: Vec<String>,
    pub conformance: Vec<String>,
}

// Original module: state_machine.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateMachine {
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
    pub region: Vec<String>,
    pub submachine_state: Vec<String>,
    pub connection_point: Vec<String>,
    pub extended_state_machine: Vec<String>,
}

// Original module: region.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
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
    pub subvertex: Vec<String>,
    pub transition: Vec<String>,
    pub state: Option<String>,
    pub extended_region: Option<String>,
    pub state_machine: Option<String>,
}

// Original module: vertex.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub container: Option<String>,
}

// Original module: transition.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
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
    pub kind: TransitionKind,
    pub container: String,
    pub redefined_transition: Option<String>,
    pub guard: Option<String>,
    pub effect: Option<String>,
    pub trigger: Vec<String>,
    pub target: String,
    pub source: String,
}

// Original module: trigger.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trigger {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub event: String,
    pub port: Vec<String>,
}

// Original module: state.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
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
    pub container: Option<String>,
    pub submachine: Option<String>,
    pub connection: Vec<String>,
    pub connection_point: Vec<String>,
    pub redefined_state: Option<String>,
    pub state_invariant: Option<String>,
    pub entry: Option<String>,
    pub exit: Option<String>,
    pub do_activity: Option<String>,
    pub deferrable_trigger: Vec<String>,
    pub region: Vec<String>,
}

// Original module: connection_point_reference.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPointReference {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub container: Option<String>,
    pub entry: Vec<String>,
    pub exit: Vec<String>,
    pub state: Option<String>,
}

// Original module: pseudostate.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pseudostate {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub container: Option<String>,
    pub kind: PseudostateKind,
    pub state_machine: Option<String>,
    pub state: Option<String>,
}

// Original module: protocol_conformance.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolConformance {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub general_machine: String,
    pub specific_machine: String,
}

// Original module: final_state.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalState {
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
    pub container: Option<String>,
    pub submachine: Option<String>,
    pub connection: Vec<String>,
    pub connection_point: Vec<String>,
    pub redefined_state: Option<String>,
    pub state_invariant: Option<String>,
    pub entry: Option<String>,
    pub exit: Option<String>,
    pub do_activity: Option<String>,
    pub deferrable_trigger: Vec<String>,
    pub region: Vec<String>,
}

// Original module: protocol_transition.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolTransition {
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
    pub kind: TransitionKind,
    pub container: String,
    pub redefined_transition: Option<String>,
    pub guard: Option<String>,
    pub effect: Option<String>,
    pub trigger: Vec<String>,
    pub target: String,
    pub source: String,
    pub post_condition: Option<String>,
    pub pre_condition: Option<String>,
}

// Original module: pseudostate_kind.rs
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PseudostateKind {
    Initial,
    Deephistory,
    Shallowhistory,
    Join,
    Fork,
    Junction,
    Choice,
    Entrypoint,
    Exitpoint,
    Terminate,
}

// Original module: transition_kind.rs
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionKind {
    Internal,
    Local,
    External,
}
