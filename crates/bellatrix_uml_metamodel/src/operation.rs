// Native UML/Ecore declaration from javapak/ruml-wasm 5c90d7399acbce4c37847b272759ed85d677ba92; MIT.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::visibility_kind::VisibilityKind;
use crate::call_concurrency_kind::CallConcurrencyKind;
use crate::interface::Interface;
use crate::class::Class;
use crate::data_type::DataType;
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
