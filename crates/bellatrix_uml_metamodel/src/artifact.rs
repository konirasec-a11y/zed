// Native UML/Ecore declaration from javapak/ruml-wasm 5c90d7399acbce4c37847b272759ed85d677ba92; MIT.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::visibility_kind::VisibilityKind;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
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

    pub file_name: Option<String>,

    pub nested_artifact: Vec<String>,

    pub manifestation: Vec<String>,

    pub owned_operation: Vec<String>,

    pub owned_attribute: Vec<String>,
}
