//! Bounded UML 2.5.1 Property subsetting (§§9.9.17.7–8).
//! Original XMI contracts compose existing canonical ownership, Association,
//! Classifier and value queries; no separate subset graph or evaluator is stored.

use super::{
    classifiers::{self, QueryError},
    feature_compatibility::{
        accept_any, compare_resolved_types, compare_type_conformance, require_all,
        resolve_feature_type, resolve_feature_upper_bound,
    },
    references::{ReferenceIndex, reference_id},
    root::{ElementRef, Model},
    schema,
};
use crate::{ModelingSection, StructuralDiagnostic, StructuralSeverity};
use serde_json::json;
use std::collections::HashSet;

fn require_property(element: ElementRef<'_>) -> Result<(), QueryError> {
    if element.conforms_to("Property") {
        Ok(())
    } else if element.metaclass().is_none() {
        Err(QueryError::unsupported(
            element.id(),
            "Property metaclass is unresolved",
        ))
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
    let (local, unresolved) = resolve_subsetted_properties(references, property)?;
    if unresolved {
        return Err(unresolved_subset(property));
    }
    Ok(local)
}

fn unresolved_subset(property: ElementRef<'_>) -> QueryError {
    QueryError::unsupported(
        property.id(),
        "subsetted Property references or metaclasses are unresolved",
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
    let unresolved =
        match super::property_contracts::check_value(references, property, descriptor, value) {
            super::property_contracts::PropertyOutcome::Satisfied => false,
            super::property_contracts::PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(property.id(), format!("subsettedProperty requires unique Property identities: {message}")));
            }
            super::property_contracts::PropertyOutcome::Unverified(_) => true,
        };
    // Retain known Properties beside unresolved members so local violations
    // still decide the conjunction, without presenting this as a complete list.
    let local = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| reference_id(value).ok().flatten())
        .filter_map(|id| references.element(id).ok())
        .filter(|target| target.conforms_to("Property"))
        .collect();
    Ok((local, unresolved))
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
    let associations = references
        .association_membership_candidates(property.id())
        .map_err(|error| QueryError::invalid(property.id(), error))?;
    if associations.len() > 1 {
        return Err(QueryError::invalid(
            property.id(),
            "Property::association has upper bound 1",
        ));
    }
    if let Some(association) = associations.first() {
        let descriptor = schema::uml_property("Association", "memberEnd")
            .map_err(|error| QueryError::unsupported(property.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(property.id(), "missing memberEnd contract"))?;
        let value = association.properties().get("memberEnd").ok_or_else(|| {
            QueryError::unsupported(property.id(), "Association memberEnd is not materialized")
        })?;
        let contract = match super::property_contracts::check_value(references, *association, descriptor, value) {
            super::property_contracts::PropertyOutcome::Satisfied => Ok(()),
            super::property_contracts::PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(property.id(), message));
            }
            super::property_contracts::PropertyOutcome::Unverified(message) => {
                Err(QueryError::unsupported(property.id(), message))
            }
        };
        let mut seen = HashSet::new();
        let contexts = value.as_array().into_iter().flatten()
            .filter_map(|value| reference_id(value).ok().flatten())
            .filter_map(|id| references.element(id).ok())
            .filter(|other| other.id() != property.id() && other.conforms_to("Property"))
            .map(|other| resolve_feature_type(references, other))
            .filter(|result| match result {
                Ok(element) => seen.insert(element.id()),
                Err(_) => true,
            })
            .collect::<Vec<_>>();
        for result in &contexts {
            if let Err(error @ QueryError::Invalid { .. }) = result {
                return Err(error.clone());
            }
        }
        contract?;
        return contexts.into_iter().collect();
    }
    let Some(owner) = references
        .model()
        .owner(property)
        .map_err(|error| QueryError::invalid(property.id(), error))?
    else {
        return Ok(Vec::new());
    };
    if owner.metaclass().is_none() {
        return Err(QueryError::unsupported(
            property.id(),
            "Property owner metaclass is unresolved",
        ));
    }
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
    let contexts = subsetting_context(references, specific);
    let general_contexts = subsetting_context(references, general);
    for result in [&contexts, &general_contexts] {
        if let Err(error @ QueryError::Invalid { .. }) = result {
            return Err(error.clone());
        }
    }
    let contexts = contexts?;
    if contexts.is_empty() {
        return Ok(false);
    }
    let general_contexts = general_contexts?;
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
    let specific_contract = require_property(specific);
    let general_contract = require_property(general);
    for result in [&specific_contract, &general_contract] {
        if let Err(error @ QueryError::Invalid { .. }) = result {
            return Err(error.clone());
        }
    }
    specific_contract?;
    general_contract?;
    let upper = (|| {
        let specific = resolve_feature_upper_bound(references, specific);
        let general = resolve_feature_upper_bound(references, general);
        for result in [&specific, &general] {
            if let Err(error @ QueryError::Invalid { .. }) = result {
                return Err(error.clone());
            }
        }
        Ok(specific?.fits_within(&general?))
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
    let (local, unresolved) = resolve_subsetted_properties(references, property)?;
    let results = local
        .into_iter()
        .map(|general| compare_subsetting(references, property, general))
        .chain(unresolved.then(|| Err(unresolved_subset(property))));
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

fn subsetted_property_names(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let Some(value) = property.properties().get("subsettedProperty") else {
        return Ok(true);
    };
    let descriptor = schema::uml_property("Property", "subsettedProperty")
        .map_err(|error| QueryError::unsupported(property.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(property.id(), "missing subsetting contract"))?;
    let contract =
        match super::property_contracts::check_value(references, property, descriptor, value) {
            super::property_contracts::PropertyOutcome::Satisfied => Ok(true),
            super::property_contracts::PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(property.id(), message));
            }
            super::property_contracts::PropertyOutcome::Unverified(message) => {
                Err(QueryError::unsupported(property.id(), message))
            }
        };
    let mut checks = vec![contract];
    for target in value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| super::references::reference_id(value).ok().flatten())
        .filter_map(|id| references.element(id).ok())
    {
        // Unknown metaclasses cannot supply a NamedElement name, even if the
        // storage record has editor text. Keep the contract's Unknown instead.
        if target.conforms_to("Property") {
            checks.push(Ok(property.named().and_then(|named| named.name())
                != target.named().and_then(|named| named.name())));
        }
    }
    require_all(checks)
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues: Vec<_> = references
        .model()
        .elements()
        .filter_map(|element| subsetting_diagnostic(references, element))
        .collect();
    for property in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Property"))
    {
        let (severity, code, message) = match subsetted_property_names(references, property) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_PROPERTY_SUBSETTED_PROPERTY_NAMES",
                "a Property may not subset a Property with the same name".to_owned(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "MDE_PROPERTY_SUBSETTED_PROPERTY_NAMES_INVALID",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_PROPERTY_SUBSETTED_PROPERTY_NAMES_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: property.id().into(),
            message: format!("Property-subsetted_property_names: {message}"),
        });
    }
    issues
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
