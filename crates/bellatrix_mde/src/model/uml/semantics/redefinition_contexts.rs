// SPDX-License-Identifier: Apache-2.0
// Copyright 2019-2026 Starion Group S.A.
// Derived-union patterns adapted from pinned uml4net RedefinableElementExtensions.
// The generated ownership-subset and cycle-safe graph adapters are Bellatrix's.
//! UML 2.5.1 §9.9.18: canonical redefinition contexts and the default context predicate.

use super::{
    classifier_hierarchy,
    classifiers::{self, QueryError},
    feature_compatibility::accept_any,
    references::ReferenceIndex,
    root::ElementRef,
};

/// Ownership is a context only through a property contributing to the opposite
/// redefinition-context union. Reception/ActivityNode ownership alone is not enough.
pub fn resolve_redefinition_contexts<'a>(
    references: &ReferenceIndex<'a>,
    element: ElementRef<'a>,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    if !element.conforms_to("RedefinableElement") {
        return Err(QueryError::invalid(
            element.id(),
            "a RedefinableElement is required",
        ));
    }
    for property in classifiers::properties(element)? {
        if !classifiers::contributes(property, "RedefinableElement-redefinitionContext")
            .map_err(|error| QueryError::unsupported(element.id(), error))?
        {
            continue;
        }
        if element.properties().contains_key(property.name) {
            return Err(QueryError::unsupported(
                element.id(),
                format!(
                    "{} must be derived from canonical context ownership",
                    property.name
                ),
            ));
        }
        if property.is_derived && property.id.fragment != "RedefinableElement-redefinitionContext" {
            return Err(QueryError::unsupported(
                element.id(),
                format!(
                    "{} context derivation is not implemented",
                    property.id.fragment
                ),
            ));
        }
    }
    let Some(owner) = references
        .model()
        .owner(element)
        .map_err(|error| QueryError::invalid(element.id(), error))?
    else {
        return Ok(Vec::new());
    };
    if owner.metaclass().is_none() {
        return Err(QueryError::unsupported(
            element.id(),
            "canonical context owner metaclass is unresolved",
        ));
    }
    if !owner.conforms_to("Classifier") {
        return Ok(Vec::new());
    }
    if classifiers::contributes_to_owned_union(
        owner,
        element,
        "A_redefinitionContext_redefinableElement-redefinableElement",
    )? {
        Ok(vec![owner])
    } else {
        Ok(Vec::new())
    }
}

/// The official default OCL requires one source context whose allParents includes
/// every target context. It is strict ancestry, not same-metaclass/name equality.
/// Subclasses with a specialized predicate are deliberately not evaluated here.
pub fn compare_redefinition_contexts(
    references: &ReferenceIndex<'_>,
    redefining: ElementRef<'_>,
    redefined: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if ["Region", "StateMachine", "Vertex"]
        .iter()
        .any(|metaclass| redefining.conforms_to(metaclass))
    {
        return Err(QueryError::unsupported(
            redefining.id(),
            "specialized state-machine redefinition context predicate is not implemented",
        ));
    }
    let contexts = resolve_redefinition_contexts(references, redefining)?;
    if contexts.is_empty() {
        return Ok(false);
    }
    let targets = resolve_redefinition_contexts(references, redefined)?;
    accept_any(contexts.into_iter().map(|context| {
        let ancestry = classifier_hierarchy::resolve_ancestry(references, context.id())?;
        Ok(targets.iter().all(|target| {
            ancestry
                .ancestors
                .iter()
                .any(|ancestor| ancestor.id() == target.id())
        }))
    }))
}

/// Property-redefined_property_inherited is stricter than context ancestry:
/// every referenced Property must actually occur among ancestral allFeatures().
pub(crate) fn property_redefined_inherited(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::{
        feature_compatibility::require_all,
        property_contracts::{self, PropertyOutcome},
        schema,
    };
    let Some(value) = property.properties().get("redefinedProperty") else {
        return Ok(true);
    };
    let descriptor = schema::uml_property("Property", "redefinedProperty")
        .map_err(|error| QueryError::unsupported(property.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(property.id(), "missing redefinedProperty contract")
        })?;
    let mut checks = Vec::new();
    match property_contracts::check_value(references, property, descriptor, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(property.id(), message));
        }
        PropertyOutcome::Unverified(message) => {
            checks.push(Err(QueryError::unsupported(property.id(), message)))
        }
    }
    let values = value.as_array().ok_or_else(|| {
        QueryError::invalid(property.id(), "redefinedProperty must be a collection")
    })?;
    if values.is_empty() {
        return Ok(true);
    }
    if let Some(value) = property.properties().get("redefinitionContext") {
        let descriptor = schema::uml_property("RedefinableElement", "redefinitionContext")
            .map_err(|error| QueryError::unsupported(property.id(), error.to_string()))?
            .ok_or_else(|| {
                QueryError::unsupported(property.id(), "missing redefinitionContext contract")
            })?;
        if let PropertyOutcome::Violation(message) = property_contracts::check_value(
            references,
            property,
            &schema::PropertyDescriptor {
                is_derived: false,
                ..*descriptor
            },
            value,
        ) {
            return Err(QueryError::invalid(property.id(), message));
        }
    }
    let contexts = resolve_redefinition_contexts(references, property)?;
    checks.push(Ok(!contexts.is_empty()));
    // Even an external redefined target cannot hide a known malformed/cyclic
    // context hierarchy. Unknown ancestry is combined per target below.
    for context in &contexts {
        if let Err(error @ QueryError::Invalid { .. }) =
            classifier_hierarchy::has_ancestor(references, context.id(), context.id())
        {
            checks.push(Err(error));
        }
    }
    for value in values {
        let Some(id) = super::references::reference_id(value).ok().flatten() else {
            continue;
        };
        let target = references
            .element(id)
            .map_err(|error| QueryError::invalid(property.id(), error))?;
        if target.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(
                target.id(),
                "redefined Property metaclass is unresolved",
            )));
            continue;
        }
        let mut witnesses = Vec::new();
        for context in &contexts {
            // Keep unresolved ancestry as a possible witness, but inspect all
            // proven local ancestors as well. Cycles and Invalid always dominate.
            match classifier_hierarchy::resolve_ancestry(references, context.id()) {
                Err(error) => witnesses.push(Err(error)),
                Ok(_) => {}
            }
            for ancestor in references
                .model()
                .elements()
                .filter(|element| element.conforms_to("Classifier"))
            {
                match classifier_hierarchy::has_ancestor(references, context.id(), ancestor.id()) {
                    Ok(true) => {
                        let member =
                            classifiers::has_feature(references, ancestor.id(), target.id());
                        witnesses.push(member);
                    }
                    Ok(false) => {}
                    Err(error @ QueryError::Invalid { .. }) => witnesses.push(Err(error)),
                    Err(_) => {}
                }
            }
        }
        checks.push(accept_any(witnesses));
    }
    require_all(checks)
}
