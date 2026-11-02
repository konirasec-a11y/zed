//! Native declarations retained from javapak/ruml-wasm at
//! 5c90d7399acbce4c37847b272759ed85d677ba92 (upstream declares MIT).
//! These were the non-duplicate deployment declarations in the former metamodel
//! directory. Their original field types are preserved, not certified as a
//! complete or normalized UML 2.5.1 implementation.
use serde::{Deserialize, Serialize};
use super::common_structure::VisibilityKind;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
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
    pub file_name: Option<String>,
    pub nested_artifact: Vec<String>,
    pub manifestation: Vec<String>,
    pub owned_operation: Vec<String>,
    pub owned_attribute: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentSpecification {
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
    pub file_name: Option<String>,
    pub nested_artifact: Vec<String>,
    pub manifestation: Vec<String>,
    pub owned_operation: Vec<String>,
    pub owned_attribute: Vec<String>,
    pub deployment_location: Option<String>,
    pub execution_location: Option<String>,
    pub deployment: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
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
    pub deployed_artifact: Vec<String>,
    pub configuration: Vec<String>,
    pub location: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentTarget {
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
    pub deployment: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployedArtifact {
    pub id: String,
    pub e_annotations: Vec<String>,
    pub owned_comment: Vec<String>,
    pub name: Option<String>,
    pub visibility: Option<VisibilityKind>,
    pub client_dependency: Vec<String>,
    pub name_expression: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifestation {
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
    pub utilized_element: String,
}
