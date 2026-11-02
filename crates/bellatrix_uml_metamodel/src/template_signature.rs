// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::template_parameter::TemplateParameter;
use crate::templateable_element::TemplateableElement;
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
