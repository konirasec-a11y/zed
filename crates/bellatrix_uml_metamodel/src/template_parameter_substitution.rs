// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::template_binding::TemplateBinding;
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
