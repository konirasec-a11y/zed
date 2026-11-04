use serde::Deserializer;
use serde::de::{MapAccess, Visitor};
use serde_json::{Map, Value, value::RawValue};
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MdjError {
    #[error("invalid MDJ JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid MDJ at {path}: {message}")]
    Structure { path: String, message: String },
}

fn invalid(path: &str, message: impl Into<String>) -> MdjError {
    MdjError::Structure {
        path: display_path(path).to_owned(),
        message: message.into(),
    }
}

fn display_path(path: &str) -> &str {
    if path.is_empty() { "/" } else { path }
}

fn property_path(path: &str, property: &str) -> String {
    format!("{path}/{}", property.replace('~', "~0").replace('/', "~1"))
}

/// Reads the native tree without converting it to the existing flattened Project.
/// Unknown types and properties are retained, including all view and subview data.
/// JSON whitespace is not retained; numbers retain their JSON precision.
pub fn decode(bytes: &[u8]) -> Result<Value, MdjError> {
    let raw: Box<RawValue> = serde_json::from_slice(bytes)?;
    let document = read_value(&raw, "", 0)?;
    Index::new(&document)?;
    Ok(document)
}

fn read_value(raw: &RawValue, path: &str, depth: usize) -> Result<Value, MdjError> {
    check_depth(path, depth)?;
    match raw.get().trim_start().as_bytes().first() {
        Some(b'{') => {
            // Value's usual deserializer replaces duplicate keys silently. Read raw
            // members first so an ambiguous document cannot lose data on ingestion.
            let mut deserializer = serde_json::Deserializer::from_str(raw.get());
            let members = deserializer.deserialize_map(ObjectMembers)?;
            let mut object = Map::new();
            for (key, value) in members {
                let child_path = property_path(path, &key);
                if object.contains_key(&key) {
                    return Err(invalid(&child_path, "duplicate JSON property"));
                }
                object.insert(key, read_value(&value, &child_path, depth + 1)?);
            }
            Ok(Value::Object(object))
        }
        Some(b'[') => {
            let values: Vec<Box<RawValue>> = serde_json::from_str(raw.get())?;
            values
                .iter()
                .enumerate()
                .map(|(position, value)| {
                    read_value(value, &format!("{path}/{position}"), depth + 1)
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        _ => Ok(serde_json::from_str(raw.get())?),
    }
}

struct ObjectMembers;

impl<'de> Visitor<'de> for ObjectMembers {
    type Value = Vec<(String, Box<RawValue>)>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an MDJ object")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut members = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            members.push((key, map.next_value::<Box<RawValue>>()?));
        }
        Ok(members)
    }
}

#[derive(Clone, Debug)]
pub struct Object<'document> {
    pub path: String,
    pub id: &'document str,
    pub type_name: &'document str,
    pub properties: &'document Map<String, Value>,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct Reference<'document> {
    pub path: String,
    pub target_id: &'document str,
}

/// A read-only, disposable index into the caller's native JSON tree. No object,
/// property or view is copied into a second persisted representation.
#[derive(Debug)]
pub struct Index<'document> {
    objects: BTreeMap<&'document str, Object<'document>>,
    references: Vec<Reference<'document>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'document> Index<'document> {
    pub fn new(document: &'document Value) -> Result<Self, MdjError> {
        let root = document
            .as_object()
            .ok_or_else(|| invalid("", "expected a Project object"))?;
        if root.get("_type").and_then(Value::as_str) != Some("Project") {
            return Err(invalid("/_type", "expected Project"));
        }
        required_string(root, "_id", "")?;
        let mut index = Self {
            objects: BTreeMap::new(),
            references: Vec::new(),
            diagnostics: Vec::new(),
        };
        index.visit(document, "", 0)?;
        Ok(index)
    }

    pub fn object(&self, id: &str) -> Option<&Object<'document>> {
        self.objects.get(id)
    }

    pub fn objects(&self) -> impl Iterator<Item = &Object<'document>> {
        self.objects.values()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Missing targets are reported, never removed or assigned new identities.
    pub fn unresolved_references(&self) -> impl Iterator<Item = &Reference<'document>> {
        self.references
            .iter()
            .filter(|reference| !self.objects.contains_key(reference.target_id))
    }

    fn visit(&mut self, value: &'document Value, path: &str, depth: usize) -> Result<(), MdjError> {
        check_depth(path, depth)?;
        match value {
            Value::Object(properties) => {
                // Extension payloads can use these keys without representing MDJ
                // objects. Required identities are checked at composition boundaries.
                if let (Some(id), Some(type_name)) = (
                    nonempty_string(properties, "_id"),
                    nonempty_string(properties, "_type"),
                ) {
                    if let Some(previous) = self.objects.get(id) {
                        return Err(invalid(
                            &property_path(path, "_id"),
                            format!(
                                "duplicate ID {id:?}; first defined at {}",
                                display_path(&previous.path)
                            ),
                        ));
                    }
                    self.objects.insert(
                        id,
                        Object {
                            path: path.to_owned(),
                            id,
                            type_name,
                            properties,
                        },
                    );
                    for property in ["ownedElements", "ownedViews", "subViews"] {
                        if let Some(children) = properties.get(property) {
                            let child_path = property_path(path, property);
                            let children = children.as_array().ok_or_else(|| {
                                invalid(&child_path, "expected an array of objects")
                            })?;
                            for (position, child) in children.iter().enumerate() {
                                let child_path = format!("{child_path}/{position}");
                                let child = child.as_object().ok_or_else(|| {
                                    invalid(&child_path, "expected an object with _id and _type")
                                })?;
                                required_string(child, "_id", &child_path)?;
                                required_string(child, "_type", &child_path)?;
                            }
                        }
                    }
                }
                if properties.contains_key("$ref") {
                    let reference_path = property_path(path, "$ref");
                    if let Some(target_id) = nonempty_string(properties, "$ref") {
                        self.references.push(Reference {
                            path: reference_path,
                            target_id,
                        });
                    } else {
                        // A plugin may use $ref for unrelated data. Keep that data
                        // and surface the ambiguity rather than rejecting its payload.
                        self.diagnostics.push(Diagnostic {
                            path: reference_path,
                            message: "not indexed as a reference: expected a nonempty string"
                                .to_owned(),
                        });
                    }
                }
                for (property, child) in properties {
                    self.visit(child, &property_path(path, property), depth + 1)?;
                }
            }
            Value::Array(values) => {
                for (position, value) in values.iter().enumerate() {
                    self.visit(value, &format!("{path}/{position}"), depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn required_string<'document>(
    properties: &'document Map<String, Value>,
    property: &str,
    path: &str,
) -> Result<&'document str, MdjError> {
    nonempty_string(properties, property)
        .ok_or_else(|| invalid(&property_path(path, property), "expected a nonempty string"))
}

fn nonempty_string<'document>(
    properties: &'document Map<String, Value>,
    property: &str,
) -> Option<&'document str> {
    properties
        .get(property)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn check_depth(path: &str, depth: usize) -> Result<(), MdjError> {
    // The public index also accepts caller-created Values, which have not passed
    // through serde_json's parser recursion limit.
    if depth > 128 {
        return Err(invalid(path, "nesting exceeds the supported depth of 128"));
    }
    Ok(())
}
