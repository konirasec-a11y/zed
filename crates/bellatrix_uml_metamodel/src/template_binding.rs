// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::template_parameter_substitution::TemplateParameterSubstitution;
use crate::templateable_element::TemplateableElement;
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
