// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::classifier::Classifier;
use crate::generalization_set::GeneralizationSet;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Generalization {
    /// Unique identifier for this instance

    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub is_substitutable: Option<String>,

    pub general: String,

    pub generalization_set: Vec<String>,

    pub specific: String,
}
