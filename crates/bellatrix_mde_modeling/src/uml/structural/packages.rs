//! Native UML 2.5 declarations: structural/packages.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use crate::uml::structural::common_structure::VisibilityKind;

// Original module: package.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
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
    pub owning_template_parameter: Option<String>,
    pub template_parameter: Option<String>,
    pub template_binding: Vec<String>,
    pub owned_template_signature: Option<String>,
    pub package_merge: Vec<String>,
    pub packaged_element: Vec<String>,
    pub profile_application: Vec<String>,
}

// Original module: namespace.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Namespace {
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
}

// Original module: packageable_element.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageableElement {
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

// Original module: package_import.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageImport {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub visibility: VisibilityKind,
    pub imported_package: String,
    pub importing_namespace: String,
}

// Original module: element_import.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementImport {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub visibility: VisibilityKind,
    pub alias: Option<String>,
    pub imported_element: String,
    pub importing_namespace: String,
}

// Original module: package_merge.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMerge {
    /// Unique identifier for this instance
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub merged_package: String,
    pub receiving_package: String,
}

// Original module: profile.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
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
    pub owning_template_parameter: Option<String>,
    pub template_parameter: Option<String>,
    pub template_binding: Vec<String>,
    pub owned_template_signature: Option<String>,
    pub package_merge: Vec<String>,
    pub packaged_element: Vec<String>,
    pub profile_application: Vec<String>,
    pub metaclass_reference: Vec<String>,
    pub metamodel_reference: Vec<String>,
}

// Original module: stereotype.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stereotype {
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
    pub icon: Vec<String>,
}

// Original module: extension.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
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
