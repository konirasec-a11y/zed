use super::{
    root::{ElementRef, Model},
    schema::{self, ClassifierDescriptor, PropertyDescriptor, UnsupportedContract},
    types::{Integer, MultiplicityRange},
};
use crate::StructuralDiagnostic;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// A borrowed lookup for one immutable graph snapshot. Duplicate identities fail closed.
pub struct ReferenceIndex<'a> {
    model: Model<'a>,
    elements: HashMap<&'a str, Option<ElementRef<'a>>>,
    association_memberships: HashMap<&'a str, Vec<&'a str>>,
}

/// Recognize the canonical external-reference envelope without resolving its URI.
/// URI syntax is left to a future document resolver, including relative references.
pub(crate) fn external_reference_href(value: &Value) -> Result<Option<&str>, String> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    let Some(href) = object.get("href") else {
        return Ok(None);
    };
    if object.len() != 1 {
        return Err("external reference requires exactly one nonempty href".into());
    }
    href.as_str()
        .filter(|href| !href.is_empty())
        .map(Some)
        .ok_or_else(|| "external reference requires exactly one nonempty href".into())
}

pub fn reference_id(value: &Value) -> Result<Option<&str>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let object = value
        .as_object()
        .ok_or("expected a local Element reference object or null")?;
    if object.contains_key("href") {
        return Err(
            "external Element references are not supported; expected a local $ref identity".into(),
        );
    }
    if object.len() != 1 {
        return Err("expected exactly one $ref field for a local Element reference".into());
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
        let mut association_memberships = HashMap::<&str, Vec<&str>>::new();
        for element in model.elements() {
            elements
                .entry(element.id())
                .and_modify(|entry| *entry = None)
                .or_insert(Some(element));
            if element.conforms_to("Association")
                && let Some(values) = element
                    .properties()
                    .get("memberEnd")
                    .and_then(Value::as_array)
            {
                for id in values
                    .iter()
                    .filter_map(|value| reference_id(value).ok().flatten())
                {
                    association_memberships
                        .entry(id)
                        .or_default()
                        .push(element.id());
                }
            }
        }
        Self {
            model,
            elements,
            association_memberships,
        }
    }

    pub fn model(&self) -> Model<'a> {
        self.model
    }

    /// Snapshot-derived candidates, not an independently mutable opposite list.
    /// Callers still validate the candidate's canonical memberEnd collection.
    pub fn association_membership_candidates(
        &self,
        property_id: &str,
    ) -> Result<Vec<ElementRef<'a>>, String> {
        self.association_memberships
            .get(property_id)
            .into_iter()
            .flatten()
            .map(|id| self.element(id))
            .collect()
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
        self.validate_target(Self::required_type(property)?, id)
            .map(Some)
    }

    pub fn validate_many(
        &self,
        property: &PropertyDescriptor,
        value: &Value,
    ) -> Result<Vec<ElementRef<'a>>, String> {
        let values = value
            .as_array()
            .ok_or("expected an array of local Element references")?;
        let multiplicity = format!("{}..{}", property.lower, property.upper)
            .parse::<MultiplicityRange>()
            .map_err(|error| format!("invalid {} multiplicity: {error}", property.name))?;
        let cardinality = values
            .len()
            .to_string()
            .parse::<Integer>()
            .map_err(|error| error.to_string())?;
        if !multiplicity.includes_cardinality(&cardinality) {
            return Err(format!(
                "{} requires {}..{} references; received {cardinality}",
                property.name, property.lower, property.upper
            ));
        }
        let required = Self::required_type(property)?;
        let mut seen = HashSet::new();
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let validate = || {
                    let id =
                        reference_id(value)?.ok_or("reference collections cannot contain null")?;
                    self.validate_target(required, id)
                };
                let target =
                    validate().map_err(|error| format!("{}[{index}]: {error}", property.name))?;
                if property.is_unique && !seen.insert(target.id()) {
                    return Err(format!(
                        "{}[{index}]: duplicate Element reference {}; unique values are required",
                        property.name,
                        target.id()
                    ));
                }
                Ok(target)
            })
            .collect()
    }

    /// A valid witness proves membership even when another source is unresolved.
    /// Absence requires every candidate to resolve; no opposite list is persisted.
    pub(crate) fn has_reference_source(
        &self,
        property: &PropertyDescriptor,
        target_id: &str,
    ) -> Result<bool, String> {
        if property.is_derived || property.aggregation == schema::Aggregation::Composite {
            return Err(
                "derived and composite references require a canonical graph projection".into(),
            );
        }
        self.validate_target(Self::required_type(property)?, target_id)?;
        let owner = schema::classifier(property.owner).ok_or("unresolved reference owner metaclass")?;
        let mut unresolved = None;
        for source in self
            .model
            .elements()
            .filter(|element| element.conforms_to(owner.name))
        {
            let contains = || -> Result<bool, String> {
                self.element(source.id())?;
                if matches!(property.upper, "0" | "1") {
                    return self
                        .validate(
                            property,
                            source.properties().get(property.name).unwrap_or(&Value::Null),
                        )
                        .map(|target| target.is_some_and(|target| target.id() == target_id));
                }
                let empty = Value::Array(Vec::new());
                let value = source.properties().get(property.name).unwrap_or(&empty);
                match self.validate_many(property, value) {
                    Ok(targets) => Ok(targets.iter().any(|target| target.id() == target_id)),
                    Err(error) => {
                        // A malformed neighbor or duplicate does not erase a
                        // typed member. The property validator reports that fault.
                        let member = PropertyDescriptor {
                            lower: "1",
                            upper: "1",
                            ..*property
                        };
                        if value.as_array().is_some_and(|values| {
                            values.iter().any(|value| {
                                matches!(self.validate(&member, value), Ok(Some(target))
                                    if target.id() == target_id)
                            })
                        }) {
                            Ok(true)
                        } else {
                            Err(error)
                        }
                    }
                }
            };
            match contains() {
                Ok(true) => return Ok(true),
                Ok(false) => {}
                Err(error) => {
                    unresolved.get_or_insert_with(|| {
                        format!("{}::{}: {error}", source.id(), property.name)
                    });
                }
            }
        }
        match unresolved {
            Some(error) => Err(error),
            None => Ok(false),
        }
    }

    fn required_type(
        property: &PropertyDescriptor,
    ) -> Result<&'static ClassifierDescriptor, String> {
        if property.property_type.resolution == schema::ReferenceResolution::ExternalDocument {
            return Err(
                "external property Types are not supported by local Element references".into(),
            );
        }
        property
            .property_type
            .target
            .and_then(schema::classifier)
            .ok_or_else(|| UnsupportedContract::UnresolvedType.to_string())
    }

    fn validate_target(
        &self,
        required: &ClassifierDescriptor,
        id: &str,
    ) -> Result<ElementRef<'a>, String> {
        let target = self.element(id)?;
        let conforms = target
            .metaclass()
            .and_then(schema::uml_classifier)
            .is_some_and(|actual| schema::conforms_to(actual.id, required.id) == Ok(true));
        if !conforms {
            return Err(format!("{id} must conform to UML {}", required.name));
        }
        Ok(target)
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
        super::property_contracts::validate(self)
    }
}
