// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorEnd {
    /// Unique identifier for this instance

    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub is_ordered: String,

    pub is_unique: String,

    pub upper_value: Option<String>,

    pub lower_value: Option<String>,

    pub part_with_port: Option<String>,

    pub role: String,
}
