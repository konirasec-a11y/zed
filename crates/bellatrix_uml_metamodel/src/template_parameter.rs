// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::template_signature::TemplateSignature;
use crate::parameterable_element::ParameterableElement;
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
