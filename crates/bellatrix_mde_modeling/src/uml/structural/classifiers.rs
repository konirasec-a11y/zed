//! Native UML 2.5 declarations: structural/classifiers; see RUML_IMPORT_MANIFEST.json.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::uml::behavioral::common_behavior::CallConcurrencyKind;
use crate::uml::structural::common_structure::VisibilityKind;

// Original module: type_.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Type {
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

// Original module: typed_element.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedElement {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub type_: Option<String>,
}

// Original module: feature.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub is_leaf: String,
    pub is_static: String,
}

// Original module: structural_feature.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralFeature {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub is_leaf: String,
    pub is_static: String,
    pub type_: Option<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
    pub is_read_only: String,
}

// Original module: classifier.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Classifier {
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
}

// Original module: class.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Class {
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
}

// Original module: interface.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interface {
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
    pub owned_operation: Vec<String>,
    pub nested_classifier: Vec<String>,
    pub redefined_interface: Vec<String>,
    pub owned_reception: Vec<String>,
    pub protocol: Option<String>,
}

// Original module: reception.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reception {
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
    pub is_static: String,
    pub owned_parameter: Vec<String>,
    pub is_abstract: String,
    pub method: Vec<String>,
    pub concurrency: CallConcurrencyKind,
    pub raised_exception: Vec<String>,
    pub owned_parameter_set: Vec<String>,
    pub signal: Option<String>,
}

// Original module: signal.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
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
}

// Original module: generalization.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Generalization {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub is_substitutable: Option<String>,
    pub general: String,
    pub generalization_set: Vec<String>,
    pub specific: String,
}

// Original module: generalization_set.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralizationSet {
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
    pub is_covering: String,
    pub is_disjoint: String,
    pub powertype: Option<String>,
    pub generalization: Vec<String>,
}

// Original module: association.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Association {
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
    pub owned_end: Vec<String>,
    pub member_end: Vec<String>,
    pub is_derived: String,
    pub navigable_owned_end: Vec<String>,
}

// Original module: association_class.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssociationClass {
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
    pub owned_end: Vec<String>,
    pub member_end: Vec<String>,
    pub is_derived: String,
    pub navigable_owned_end: Vec<String>,
}

// Original module: aggregation_kind.rs
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationKind {
    None,
    Shared,
    Composite,
}

// Original module: connectable_element.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectableElement {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub type_: Option<String>,
    pub owning_template_parameter: Option<String>,
    pub template_parameter: Option<String>,
}

// Original module: connectable_element_template_parameter.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectableElementTemplateParameter {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub signature: String,
    pub parametered_element: String,
    pub owned_parametered_element: Option<String>,
    pub default: Option<String>,
    pub owned_default: Option<String>,
}

// Original module: connector.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connector {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub is_leaf: String,
    pub is_static: String,
    pub type_: Option<String>,
    pub redefined_connector: Vec<String>,
    pub end: Vec<String>,
    pub kind: Option<ConnectorKind>,
    pub contract: Vec<String>,
}

// Original module: connector_end.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorEnd {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
    pub part_with_port: Option<String>,
    pub role: String,
}
