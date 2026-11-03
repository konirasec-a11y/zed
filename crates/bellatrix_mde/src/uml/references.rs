use super::{
    root::{ElementRef, Model},
    schema::{self, PropertyDescriptor, UnsupportedContract},
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// A borrowed lookup for one immutable graph snapshot. Duplicate identities fail closed.
pub struct ReferenceIndex<'a> {
    model: Model<'a>,
    elements: HashMap<&'a str, Option<ElementRef<'a>>>,
}

pub fn reference_id(value: &Value) -> Result<Option<&str>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let object = value
        .as_object()
        .ok_or("expected a local Element reference object or null")?;
    if object.len() != 1 {
        return Err(
            "expected exactly one $ref field; external references are not supported".into(),
        );
    }
    object
        .get("$ref")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(Some)
        .ok_or_else(|| "expected a nonempty $ref Element identity".into())
}

impl<'a> ReferenceIndex<'a> {
    pub fn new(model: Model<'a>) -> Self {
        let mut elements = HashMap::new();
        for element in model.elements() {
            elements
                .entry(element.id())
                .and_modify(|entry| *entry = None)
                .or_insert(Some(element));
        }
        Self { model, elements }
    }

    pub fn model(&self) -> Model<'a> {
        self.model
    }

    /// Explicit `owns` targets in persisted edge order; excludes ownership derived
    /// from specialized relationships, so this is not UML Element::ownedElement.
    pub fn explicit_owned_elements(&self, owner_id: &str) -> Result<Vec<ElementRef<'a>>, String> {
        self.element(owner_id)?;
        let mut seen = HashSet::new();
        self.model
            .ownership_links()
            .filter(|relation| relation.from == owner_id)
            .map(|relation| {
                if !seen.insert(relation.to.as_str()) {
                    return Err("duplicate composite ownership edge".into());
                }
                let element = self.element(&relation.to)?;
                self.model.owner(element)?;
                Ok(element)
            })
            .collect()
    }

    pub fn element(&self, id: &str) -> Result<ElementRef<'a>, String> {
        match self.elements.get(id) {
            Some(Some(element)) => Ok(*element),
            Some(None) => Err(format!("ambiguous Element identity {id}")),
            None => Err(format!("unknown Element reference {id}")),
        }
    }

    pub fn validate(
        &self,
        property: &PropertyDescriptor,
        value: &Value,
    ) -> Result<Option<ElementRef<'a>>, String> {
        if !matches!(property.upper, "0" | "1") {
            return Err(UnsupportedContract::Collection.to_string());
        }
        let Some(id) = reference_id(value)? else {
            return if property.lower == "0" {
                Ok(None)
            } else {
                Err("a reference is required by the property's lower bound".into())
            };
        };
        if property.upper == "0" {
            return Err("the property's upper bound is zero".into());
        }
        let required = property
            .property_type
            .target
            .and_then(schema::classifier)
            .ok_or_else(|| UnsupportedContract::UnresolvedType.to_string())?;
        let target = self.element(id)?;
        let conforms = target
            .metaclass()
            .and_then(schema::uml_classifier)
            .is_some_and(|actual| schema::conforms_to(actual.id, required.id) == Ok(true));
        if !conforms {
            return Err(format!("{id} must conform to UML {}", required.name));
        }
        Ok(Some(target))
    }

    pub fn candidates(&self, property: &PropertyDescriptor) -> Vec<ElementRef<'a>> {
        let Some(required) = property.property_type.target.and_then(schema::classifier) else {
            return Vec::new();
        };
        if property.upper != "1" {
            return Vec::new();
        }
        self.model
            .elements()
            .filter(|element| {
                self.element(element.id()).is_ok()
                    && element
                        .metaclass()
                        .and_then(schema::uml_classifier)
                        .is_some_and(|actual| {
                            schema::conforms_to(actual.id, required.id) == Ok(true)
                        })
            })
            .collect()
    }

    pub fn validate_typed_elements(&self) -> Vec<StructuralDiagnostic> {
        self.model
            .elements()
            .filter_map(|element| {
                let property = schema::uml_property(element.metaclass()?, "type").ok()??;
                if property.is_derived {
                    return None;
                }
                let value = element.properties().get("type").unwrap_or(&Value::Null);
                // Historical declarations are retained as text, never resolved by matching a name.
                if value.is_string() {
                    return None;
                }
                let error = self.validate(property, value).err()?;
                Some(StructuralDiagnostic {
                    severity: StructuralSeverity::Error,
                    code: "UML_TYPED_ELEMENT_TYPE",
                    subject_id: element.id().into(),
                    message: format!("{}::type: {error}", element.metaclass()?),
                })
            })
            .collect()
    }
}
