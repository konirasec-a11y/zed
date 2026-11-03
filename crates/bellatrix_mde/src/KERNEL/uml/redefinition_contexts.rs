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
