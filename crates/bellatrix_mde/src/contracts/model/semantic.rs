use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StereotypeValue<'a> {
    Missing,
    Null,
    Text(&'a str),
    Reference(&'a str),
    Unsupported(&'a Value),
}

impl<'a> StereotypeValue<'a> {
    pub fn read(value: Option<&'a Value>) -> Self {
        match value {
            None => Self::Missing,
            Some(Value::Null) => Self::Null,
            Some(Value::String(text)) => Self::Text(text),
            Some(value @ Value::Object(object)) if object.len() == 1 => {
                match object.get("$ref").and_then(Value::as_str) {
                    Some(target_id) => Self::Reference(target_id),
                    None => Self::Unsupported(value),
                }
            }
            Some(value) => Self::Unsupported(value),
        }
    }
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
