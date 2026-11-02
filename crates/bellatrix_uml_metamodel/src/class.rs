// Original declaration from javapak/ruml-wasm (MIT), native Bellatrix adaptation.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::visibility_kind::VisibilityKind;
use crate::operation::Operation;
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
