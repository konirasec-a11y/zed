//! Native UML types: structural/values; see docs/bellatrix/standards/uml/RUML_IMPORT_MANIFEST.json.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use crate::uml::structural::common_structure::VisibilityKind;

// Upstream: value_specification.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueSpecification {
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
    pub type_: Option<String>,
}

// Upstream: opaque_expression.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpaqueExpression {
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
    pub type_: Option<String>,
    pub body: Vec<String>,
    pub language: Vec<String>,
    pub behavior: Option<String>,
}

// Upstream: string_expression.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringExpression {
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
    pub type_: Option<String>,
    pub symbol: Option<String>,
    pub operand: Vec<String>,
    pub template_binding: Vec<String>,
    pub owned_template_signature: Option<String>,
    pub sub_expression: Vec<String>,
    pub owning_expression: Option<String>,
}

// Upstream: expression.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expression {
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
    pub type_: Option<String>,
    pub symbol: Option<String>,
    pub operand: Vec<String>,
}

// Upstream: literal_specification.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralSpecification {
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
    pub type_: Option<String>,
}

// Upstream: literal_integer.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralInteger {
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
    pub type_: Option<String>,
    pub value: String,
}

// Upstream: literal_string.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralString {
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
    pub type_: Option<String>,
    pub value: Option<String>,
}

// Upstream: literal_boolean.rs
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralBoolean {
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
    pub type_: Option<String>,
    pub value: String,
}
