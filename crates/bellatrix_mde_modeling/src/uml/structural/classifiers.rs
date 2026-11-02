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

// Original module: connector_kind.rs
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectorKind {
    Assembly,
    Delegation,
}

// Original module: encapsulated_classifier.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncapsulatedClassifier {
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
}

// Original module: structured_classifier.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredClassifier {
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
}

// Original module: property.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
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
    pub owning_template_parameter: Option<String>,
    pub template_parameter: Option<String>,
    pub deployment: Vec<String>,
    pub template_binding: Vec<String>,
    pub owned_template_signature: Option<String>,
    pub datatype: Option<String>,
    pub is_derived: String,
    pub is_derived_union: String,
    pub aggregation: AggregationKind,
    pub redefined_property: Vec<String>,
    pub owning_association: Option<String>,
    pub default_value: Option<String>,
    pub subsetted_property: Vec<String>,
    pub association: Option<String>,
    pub qualifier: Vec<String>,
    pub association_end: Option<String>,
}

// Original module: multiplicity_element.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiplicityElement {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub is_ordered: String,
    pub is_unique: String,
    pub upper_value: Option<String>,
    pub lower_value: Option<String>,
}

// Original module: operation.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
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
    pub owning_template_parameter: Option<String>,
    pub template_parameter: Option<String>,
    pub template_binding: Vec<String>,
    pub owned_template_signature: Option<String>,
    pub interface: Option<String>,
    pub class: Option<String>,
    pub is_query: String,
    pub precondition: Vec<String>,
    pub postcondition: Vec<String>,
    pub redefined_operation: Vec<String>,
    pub datatype: Option<String>,
    pub body_condition: Option<String>,
}

// Original module: templateable_element.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateableElement {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub template_binding: Vec<String>,
    pub owned_template_signature: Option<String>,
}

// Original module: template_binding.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBinding {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub signature: String,
    pub parameter_substitution: Vec<String>,
    pub bound_element: String,
}

// Original module: template_signature.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSignature {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub parameter: Vec<String>,
    pub owned_parameter: Vec<String>,
    pub template: String,
}

// Original module: template_parameter.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateParameter {
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

// Original module: template_parameter_substitution.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateParameterSubstitution {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub formal: String,
    pub actual: Vec<String>,
    pub owned_actual: Vec<String>,
    pub template_binding: String,
}

// Original module: classifier_template_parameter.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifierTemplateParameter {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub signature: String,
    pub parametered_element: String,
    pub owned_parametered_element: Option<String>,
    pub default: Option<String>,
    pub owned_default: Option<String>,
    pub allow_substitutable: String,
    pub default_classifier: Option<String>,
    pub constraining_classifier: Option<String>,
}

// Original module: redefinable_template_signature.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedefinableTemplateSignature {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub is_leaf: String,
    pub parameter: Vec<String>,
    pub owned_parameter: Vec<String>,
    pub template: String,
    pub extended_signature: Vec<String>,
}

// Original module: operation_template_parameter.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationTemplateParameter {
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

// Original module: substitution.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Substitution {
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
    pub supplier: Vec<String>,
    pub client: Vec<String>,
    pub mapping: Option<String>,
    pub contract: String,
    pub substituting_classifier: String,
}

// Original module: realization.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Realization {
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
    pub supplier: Vec<String>,
    pub client: Vec<String>,
    pub mapping: Option<String>,
}

// Original module: abstraction.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Abstraction {
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
    pub supplier: Vec<String>,
    pub client: Vec<String>,
    pub mapping: Option<String>,
}

// Original module: interface_realization.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceRealization {
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
    pub supplier: Vec<String>,
    pub client: Vec<String>,
    pub mapping: Option<String>,
    pub contract: String,
    pub implementing_classifier: String,
}
