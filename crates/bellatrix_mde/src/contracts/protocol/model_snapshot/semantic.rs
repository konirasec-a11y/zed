use crate::contracts::model::{Annotation, FileContent, Implementation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    pub kind: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    #[serde(default)]
    pub implementations: Vec<Implementation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<FileContent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}
