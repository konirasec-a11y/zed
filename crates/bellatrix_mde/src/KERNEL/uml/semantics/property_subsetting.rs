//! Bounded UML 2.5.1 Property subsetting (§§9.9.17.7–8).
//! Original XMI contracts compose existing canonical ownership, Association,
//! Classifier and value queries; no separate subset graph or evaluator is stored.

use super::{
    associations,
    classifiers::{self, QueryError},
    feature_compatibility::{
        accept_any, compare_resolved_types, compare_type_conformance, require_all,
        resolve_feature_type, resolve_feature_upper_bound,
    },
    references::{ReferenceIndex, external_reference_href},
    root::{ElementRef, Model},
    schema,
};
use crate::{ModelingSection, StructuralDiagnostic, StructuralSeverity};
use serde_json::{Value, json};
use std::collections::HashSet;

fn require_property(element: ElementRef<'_>) -> Result<(), QueryError> {
    if element.conforms_to("Property") {
        Ok(())
    } else {
        Err(QueryError::invalid(
            element.id(),
            "subsetting requires a Property",
        ))
    }
}

pub fn subsetted_properties<'a>(
    references: &ReferenceIndex<'a>,
    property: ElementRef<'a>,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    let (local, external) = resolve_subsetted_properties(references, property)?;
    if external {
        return Err(unresolved_subset(property));
    }
    Ok(local)
}

fn unresolved_subset(property: ElementRef<'_>) -> QueryError {
    QueryError::unsupported(
        property.id(),
        "external subsetted Properties are not resolved",
    )
}

fn resolve_subsetted_properties<'a>(
    references: &ReferenceIndex<'a>,
    property: ElementRef<'a>,
) -> Result<(Vec<ElementRef<'a>>, bool), QueryError> {
    require_property(property)?;
    let Some(value) = property.properties().get("subsettedProperty") else {
        return Ok((Vec::new(), false));
    };
    let descriptor = schema::uml_property("Property", "subsettedProperty")
        .map_err(|error| QueryError::unsupported(property.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(property.id(), "missing subsetting contract"))?;
    let values = value.as_array().ok_or_else(|| {
        QueryError::invalid(
            property.id(),
            "subsettedProperty requires an array of references",
        )
    })?;
    let mut external = HashSet::new();
    let mut local = Vec::new();
    for value in values {
        if let Some(href) = external_reference_href(value)
            .map_err(|error| QueryError::invalid(property.id(), error))?
        {
            let inserted = external.insert(href);
            if descriptor.is_unique && !inserted {
                return Err(QueryError::invalid(
                    property.id(),
                    "subsettedProperty requires unique external references",
                ));
            }
        } else {
            local.push(value.clone());
        }
    }
    // This contract is 0..*: unresolved external entries cannot affect its
    // cardinality, but must not hide malformed, duplicate or mistyped local entries.
    let local = references
        .validate_many(descriptor, &Value::Array(local))
        .map_err(|error| QueryError::invalid(property.id(), error))?;
    Ok((local, !external.is_empty()))
}

/// Association end context takes precedence over attribute ownership, including
/// n-ary ends: the context is the set of Types at all the other ends.
pub fn subsetting_context<'a>(
    references: &ReferenceIndex<'a>,
    property: ElementRef<'a>,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    require_property(property)?;
    for key in [
        "association",
        "owningAssociation",
        "classifier",
        "subsettingContext",
    ] {
        if property
            .properties()
            .get(key)
            .is_some_and(|value| !value.is_null())
        {
            return Err(QueryError::unsupported(
                property.id(),
                format!("{key} requires its canonical graph projection"),
            ));
        }
    }
    if let Some(association) = associations::association_of(references, property.id())
        .map_err(|error| QueryError::invalid(property.id(), error))?
    {
        let mut seen = HashSet::new();
        return associations::member_ends(references, association.id())
            .map_err(|error| QueryError::invalid(property.id(), error))?
            .into_iter()
            .filter(|other| other.id != property.id())
            .map(|other| resolve_feature_type(references, ElementRef::Object(other)))
            .filter(|result| match result {
                Ok(element) => seen.insert(element.id()),
                Err(_) => true,
            })
            .collect();
    }
    let Some(owner) = references
        .model()
        .owner(property)
        .map_err(|error| QueryError::invalid(property.id(), error))?
    else {
        return Ok(Vec::new());
    };
    if owner.conforms_to("Association") {
        return Err(QueryError::unsupported(
            property.id(),
            "Association-owned Property requires canonical memberEnd membership",
        ));
    }
    if owner.conforms_to("Classifier") && classifiers::is_owned_attribute(owner, property)? {
        Ok(vec![owner])
    } else {
        Ok(Vec::new())
    }
}

fn compare_contexts(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let contexts = subsetting_context(references, specific)?;
    if contexts.is_empty() {
        return Ok(false);
    }
    let general_contexts = subsetting_context(references, general)?;
    require_all(contexts.into_iter().map(|context| {
        accept_any(
            general_contexts
                .iter()
                .map(|general| compare_resolved_types(references, context, *general)),
        )
    }))
}

/// Subsetting restricts the upper bound only. Unlike redefinition, it imposes
/// no containment condition on the lower bound and does not replace a feature.
pub fn compare_subsetting(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    require_property(specific)?;
    require_property(general)?;
    let upper = (|| {
        let specific = resolve_feature_upper_bound(references, specific)?;
        let general = resolve_feature_upper_bound(references, general)?;
        Ok(specific.fits_within(&general))
    })();
    require_all([
        Ok(specific.named().and_then(|named| named.name())
            != general.named().and_then(|named| named.name())),
        compare_type_conformance(references, specific, general),
        upper,
        compare_contexts(references, specific, general),
    ])
}

fn check_subsetting(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Result<(), QueryError> {
    let (local, external) = resolve_subsetted_properties(references, property)?;
    let results = local
        .into_iter()
        .map(|general| compare_subsetting(references, property, general))
        .chain(external.then(|| Err(unresolved_subset(property))));
    if require_all(results)? {
        return Ok(());
    }
    Err(QueryError::invalid(
        property.id(),
        "Property subsetting violates distinct names, Type conformance, upper-bound restriction or context conformance",
    ))
}

pub fn subsetting_diagnostic(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Option<StructuralDiagnostic> {
    if !property.conforms_to("Property") {
        return None;
    }
    let error = check_subsetting(references, property).err()?;
    let (severity, code, message) = match error {
        QueryError::Invalid { message, .. } => (
            StructuralSeverity::Error,
            "UML_PROPERTY_SUBSETTING",
            message,
        ),
        QueryError::Unsupported { message, .. } => (
            StructuralSeverity::Warning,
            "UML_PROPERTY_SUBSETTING_UNSUPPORTED",
            message,
        ),
    };
    Some(StructuralDiagnostic {
        severity,
        code,
        subject_id: property.id().into(),
        message,
    })
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    references
        .model()
        .elements()
        .filter_map(|element| subsetting_diagnostic(references, element))
        .collect()
}

pub fn set_subsetted_properties(
    section: &mut ModelingSection,
    property_id: &str,
    subsetted_ids: &[String],
) -> Result<(), String> {
    let mut candidate = section.clone();
    let property = candidate
        .elements
        .iter_mut()
        .find(|element| element.id == property_id)
        .ok_or("Property not found")?;
    require_property(ElementRef::Object(property)).map_err(|error| error.to_string())?;
    property.properties.insert(
        "subsettedProperty".into(),
        json!(
            subsetted_ids
                .iter()
                .map(|id| json!({"$ref": id}))
                .collect::<Vec<_>>()
        ),
    );
    let references = ReferenceIndex::new(Model::from_section(&candidate));
    if let Err(error @ QueryError::Invalid { .. }) =
        check_subsetting(&references, references.element(property_id)?)
    {
        return Err(error.to_string());
    }
    *section = candidate;
    Ok(())
}
