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
                return Err(QueryError::invalid(
                    property.id(),
                    format!("subsettedProperty requires unique Property identities: {message}"),
                ));
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
    let (contexts, unresolved) = resolve_subsetting_context(references, property)?;
    if unresolved {
        return Err(QueryError::unsupported(
            property.id(),
            "subsetting context is not fully resolved",
        ));
    }
    Ok(contexts)
}

fn resolve_subsetting_context<'a>(
    references: &ReferenceIndex<'a>,
    property: ElementRef<'a>,
) -> Result<(Vec<ElementRef<'a>>, bool), QueryError> {
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
        let contract = match super::property_contracts::check_value(
            references,
            *association,
            descriptor,
            value,
        ) {
            super::property_contracts::PropertyOutcome::Satisfied => Ok(()),
            super::property_contracts::PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(property.id(), message));
            }
            super::property_contracts::PropertyOutcome::Unverified(message) => {
                Err(QueryError::unsupported(property.id(), message))
            }
        };
        let mut seen = HashSet::new();
        let contexts = value
            .as_array()
            .into_iter()
            .flatten()
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
        let unresolved = contract.is_err() || contexts.iter().any(Result::is_err);
        return Ok((
            contexts.into_iter().filter_map(Result::ok).collect(),
            unresolved,
        ));
    }
    let Some(owner) = references
        .model()
        .owner(property)
        .map_err(|error| QueryError::invalid(property.id(), error))?
    else {
        return Ok((Vec::new(), false));
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
        Ok((vec![owner], false))
    } else {
        Ok((Vec::new(), false))
    }
}

fn compare_contexts(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let contexts = resolve_subsetting_context(references, specific);
    let general_contexts = resolve_subsetting_context(references, general);
    for result in [&contexts, &general_contexts] {
        if let Err(error @ QueryError::Invalid { .. }) = result {
            return Err(error.clone());
        }
    }
    // An Unsupported query has no complete collection to expose, but must not
    // erase a known false conjunct or a known existential witness elsewhere.
    let (contexts, unresolved) = contexts.unwrap_or((Vec::new(), true));
    let (general_contexts, general_unresolved) = general_contexts.unwrap_or((Vec::new(), true));
    let unknown = || {
        QueryError::unsupported(
            specific.id(),
            "subsetting context conformance is unresolved",
        )
    };
    let mut checks = vec![
        Ok(!contexts.is_empty() || unresolved),
        Ok(!general_contexts.is_empty() || general_unresolved),
    ];
    for context in contexts {
        checks.push(accept_any(
            general_contexts
                .iter()
                .map(|general| compare_resolved_types(references, context, *general))
                .chain(general_unresolved.then(|| Err(unknown()))),
        ));
    }
    if unresolved {
        checks.push(Err(unknown()));
    }
    require_all(checks)
}

fn compare_subsetting_type_and_upper(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
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
        compare_type_conformance(references, specific, general),
        upper,
    ])
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
    require_all([
        Ok(specific.named().and_then(|named| named.name())
            != general.named().and_then(|named| named.name())),
        compare_subsetting_type_and_upper(references, specific, general),
        compare_contexts(references, specific, general),
    ])
}

fn check_subsetting(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Result<(), QueryError> {
    resolve_subsetted_properties(references, property)?;
    if require_all([
        subsetted_property_names(references, property),
        subsetting_rules(references, property),
        subsetting_context_conforms(references, property),
    ])? {
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

fn subsetting_rules(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let (local, unresolved) = resolve_subsetted_properties(references, property)?;
    require_all(
        local
            .into_iter()
            .map(|general| compare_subsetting_type_and_upper(references, property, general))
            .chain(unresolved.then(|| Err(unresolved_subset(property)))),
    )
}

fn subsetting_context_conforms(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let (local, unresolved) = resolve_subsetted_properties(references, property)?;
    if local.is_empty() && !unresolved {
        return Ok(true);
    }
    let context_exists =
        resolve_subsetting_context(references, property).and_then(|(contexts, unknown)| {
            if !contexts.is_empty() {
                Ok(true)
            } else if unknown {
                Err(QueryError::unsupported(
                    property.id(),
                    "subsetting context nonemptiness is unresolved",
                ))
            } else {
                Ok(false)
            }
        });
    require_all(
        std::iter::once(context_exists)
            .chain(
                local
                    .into_iter()
                    .map(|general| compare_contexts(references, property, general)),
            )
            .chain(unresolved.then(|| Err(unresolved_subset(property)))),
    )
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
