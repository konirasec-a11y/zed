use base64::{Engine as _, engine::general_purpose::STANDARD};
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
pub struct Annotation {
    pub language: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Implementation {
    pub language: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "encoding", content = "data", rename_all = "snake_case")]
pub enum FileContent {
    Text(String),
    Base64(String),
}

impl FileContent {
    pub fn bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        match self {
            Self::Text(text) => Ok(text.as_bytes().to_vec()),
            Self::Base64(value) => STANDARD.decode(value),
        }
    }
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
