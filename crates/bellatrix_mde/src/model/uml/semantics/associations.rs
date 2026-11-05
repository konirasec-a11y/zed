// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2019-2026 Starion Group S.A.
// Query adaptations from uml4net 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d,
// Extend/AssociationExtensions.cs and Extend/PropertyExtensions.cs.
// Modified for Bellatrix's canonical graph: derive opposites, reject duplicate
// identities, and enforce the pinned OMG contracts rather than mutable lists.
// License and provenance: docs/bellatrix/references/uml4net/.

use super::{
    classifier_hierarchy, classifiers,
    feature_compatibility::compare_type_conformance,
    references::{ReferenceIndex, reference_id},
    root::{ElementRef, Model},
    schema,
    types::{Integer, MultiplicityRange, UnlimitedNatural},
    value_specifications::{self, EvaluationError},
};
use crate::{
    Element, ModelingSection, Project, Relation, StructuralDiagnostic, StructuralSeverity,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

pub fn member_ends<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<&'a Element>, String> {
    let association = references.element(association_id)?;
    if !association.conforms_to("Association") {
        return Err("memberEnd owner must be an Association".into());
    }
    let descriptor = schema::uml_property("Association", "memberEnd")
        .map_err(|error| error.to_string())?
        .ok_or("Association::memberEnd contract is missing")?;
    let value = association.properties().get("memberEnd")
        .ok_or("Association memberEnd identities are not materialized; migrate the legacy binary notation first")?;
    references
        .validate_many(descriptor, value)?
        .into_iter()
        .map(|element| match element {
            ElementRef::Object(property) => Ok(property),
            _ => Err("Association memberEnd requires a Property object identity".into()),
        })
        .collect()
}

/// Explicit role references preserve ownedEnd order; otherwise use composite edge order.
pub fn owned_ends<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<&'a Element>, String> {
    let association = references.element(association_id)?;
    // Resolve the collection first so an invalid explicit ownedEnd retains its
    // ownership-integrity diagnostic instead of becoming a disjointness error.
    // The model validator checks disjointness independently of this full query.
    let owned = resolve_owned_ends(references, association_id)?;
    if association.conforms_to("AssociationClass") {
        match association_class_disjoint_attributes_ends(references, association) {
            Ok(true) => {}
            Ok(false) => return Err("AssociationClass-disjoint_attributes_ends: ownedAttribute and ownedEnd must be disjoint".into()),
            Err(classifiers::QueryError::Invalid { message, .. }) => return Err(format!("AssociationClass-disjoint_attributes_ends: {message}")),
            // Attribute-role uncertainty does not make the ownedEnd collection
            // incomplete. The independent validator reports this Unknown.
            Err(classifiers::QueryError::Unsupported { .. }) => {}
        }
    }
    Ok(owned)
}

fn resolve_owned_ends<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<&'a Element>, String> {
    let association = references.element(association_id)?;
    let members = member_ends(references, association_id)?;
    let children = references.explicit_owned_elements(association_id)?;
    let explicit = association
        .properties()
        .get("ownedEnd")
        .map(|value| -> Result<Vec<ElementRef<'a>>, String> {
            let descriptor = schema::uml_property(
                association
                    .metaclass()
                    .ok_or("Association metaclass is missing")?,
                "ownedEnd",
            )
            .map_err(|error| error.to_string())?
            .ok_or("Association::ownedEnd contract is missing")?;
            let explicit = if matches!(descriptor.upper, "0" | "1") {
                references
                    .validate(descriptor, value)?
                    .into_iter()
                    .collect()
            } else {
                references.validate_many(descriptor, value)?
            };
            let expected = children
                .iter()
                .filter(|child| members.iter().any(|member| member.id == child.id()))
                .map(|child| child.id())
                .collect::<HashSet<_>>();
            if explicit.iter().map(|end| end.id()).collect::<HashSet<_>>() != expected {
                return Err(
                    "Association::ownedEnd must match its canonically owned member ends".into(),
                );
            }
            Ok(explicit)
        })
        .transpose()?;
    let mut owned = Vec::new();
    for child in children
        .into_iter()
        .filter(|child| child.conforms_to("Property"))
    {
        let is_end = members.iter().any(|member| member.id == child.id());
        let is_owned_end =
            classifiers::contributes_to_owned_union(association, child, "Association-ownedEnd")
                .map_err(|error| error.to_string())?;
        if is_end && !is_owned_end {
            return Err("Association-owned memberEnd has a conflicting composite role".into());
        }
        if is_owned_end {
            let ElementRef::Object(property) = child else {
                return Err("Association::ownedEnd requires a Property object identity".into());
            };
            if !is_end {
                return Err("Association::ownedEnd must subset memberEnd".into());
            }
            owned.push(property);
        }
    }
    if let Some(explicit) = explicit {
        return explicit
            .into_iter()
            .map(|end| match end {
                ElementRef::Object(property) => Ok(property),
                _ => Err("Association::ownedEnd requires a Property object identity".into()),
            })
            .collect();
    }
    Ok(owned)
}

fn declared_navigable_owned_ends<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<&'a Element>, String> {
    let association = references.element(association_id)?;
    // Navigation checks membership in ownedEnd, not disjointness with the
    // independent attribute role; neither constraint may hide the other.
    let owned = resolve_owned_ends(references, association_id)?;
    let descriptor = schema::uml_property("Association", "navigableOwnedEnd")
        .map_err(|error| error.to_string())?
        .ok_or("Association::navigableOwnedEnd contract is missing")?;
    let Some(value) = association.properties().get("navigableOwnedEnd") else {
        return Ok(Vec::new());
    };
    references
        .validate_many(descriptor, value)?
        .into_iter()
        .map(|navigable| {
            owned
                .iter()
                .copied()
                .find(|end| end.id == navigable.id())
                .ok_or_else(|| {
                    "Association::navigableOwnedEnd must subset Association::ownedEnd".into()
                })
        })
        .collect()
}

/// Checks the authored declarations without claiming that the complete UML
/// navigableOwnedEnd set is known.
pub(crate) fn validate_navigation_declarations(
    references: &ReferenceIndex<'_>,
    association_id: &str,
) -> Result<(), String> {
    let association = references.element(association_id)?;
    if !association.conforms_to("Association") {
        return Err("navigation knowledge owner must be an Association".into());
    }
    let navigable = declared_navigable_owned_ends(references, association_id)?;
    let Some(knowledge) = references
        .model()
        .authoring_knowledge()
        .associations
        .get(association_id)
    else {
        return Ok(());
    };
    let owned = resolve_owned_ends(references, association_id)?;
    let mut seen = HashSet::new();
    for id in &knowledge.navigation_unverified {
        if !seen.insert(id) {
            return Err("unverified navigation references must be unique".into());
        }
        if !owned.iter().any(|end| end.id == *id) {
            return Err(format!(
                "unverified navigation {id} must reference an Association-owned memberEnd"
            ));
        }
        if navigable.iter().any(|end| end.id == *id) {
            return Err(format!(
                "unverified navigation {id} conflicts with navigableOwnedEnd"
            ));
        }
    }
    Ok(())
}

pub fn navigable_owned_ends<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<&'a Element>, String> {
    validate_navigation_declarations(references, association_id)?;
    if references
        .model()
        .authoring_knowledge()
        .associations
        .get(association_id)
        .is_some_and(|knowledge| !knowledge.navigation_unverified.is_empty())
    {
        return Err(
            "Unsupported complete navigableOwnedEnd query: authored navigation is unverified"
                .into(),
        );
    }
    declared_navigable_owned_ends(references, association_id)
}

pub fn navigation_knowledge(
    references: &ReferenceIndex<'_>,
    property_id: &str,
) -> Result<crate::NavigationKnowledge, String> {
    let property = references.element(property_id)?;
    if !property.conforms_to("Property") {
        return Err("Property::isNavigable requires a Property".into());
    }
    let association = association_of(references, property_id)?;
    if let Some(association) = association {
        validate_navigation_declarations(references, association.id())?;
        if references
            .model()
            .authoring_knowledge()
            .associations
            .get(association.id())
            .is_some_and(|knowledge| {
                knowledge
                    .navigation_unverified
                    .iter()
                    .any(|id| id == property_id)
            })
        {
            return Ok(crate::NavigationKnowledge::Unverified);
        }
    }
    if let Some(owner) = references.model().owner(property)?
        && owner.conforms_to("Classifier")
        && classifiers::is_owned_attribute(owner, property).map_err(|error| error.to_string())?
    {
        return Ok(crate::NavigationKnowledge::Known(true));
    }
    let Some(association) = association else {
        return Ok(crate::NavigationKnowledge::Known(false));
    };
    Ok(crate::NavigationKnowledge::Known(
        declared_navigable_owned_ends(references, association.id())?
            .iter()
            .any(|end| end.id == property_id),
    ))
}

pub fn is_navigable(references: &ReferenceIndex<'_>, property_id: &str) -> Result<bool, String> {
    match navigation_knowledge(references, property_id)? {
        crate::NavigationKnowledge::Known(value) => Ok(value),
        crate::NavigationKnowledge::Unverified => {
            Err("Unsupported Property::isNavigable query: authored navigation is unverified".into())
        }
    }
}

pub(crate) fn set_navigation_knowledge(
    section: &mut ModelingSection,
    association_id: &str,
    property_id: &str,
    navigation: crate::NavigationKnowledge,
) -> Result<(), String> {
    let references = ReferenceIndex::new(Model::from_section(section));
    validate_navigation_declarations(&references, association_id)?;
    if !owned_ends(&references, association_id)?
        .iter()
        .any(|end| end.id == property_id)
    {
        return Err("navigation declaration requires an Association-owned memberEnd".into());
    }
    let member_ids = member_ends(&references, association_id)?
        .iter()
        .map(|end| end.id.clone())
        .collect::<Vec<_>>();
    let mut navigable_ids = declared_navigable_owned_ends(&references, association_id)?
        .iter()
        .filter(|end| end.id != property_id)
        .map(|end| end.id.clone())
        .collect::<HashSet<_>>();
    if navigation == crate::NavigationKnowledge::Known(true) {
        navigable_ids.insert(property_id.into());
    }
    let navigable = member_ids
        .iter()
        .filter(|id| navigable_ids.contains(*id))
        .map(|id| json!({"$ref":id}))
        .collect::<Vec<_>>();
    if let Some(association) = section
        .relations
        .iter_mut()
        .find(|relation| relation.id == association_id)
    {
        association
            .properties
            .insert("navigableOwnedEnd".into(), Value::Array(navigable));
    } else {
        section
            .elements
            .iter_mut()
            .find(|element| element.id == association_id)
            .ok_or("Association not found")?
            .properties
            .insert("navigableOwnedEnd".into(), Value::Array(navigable));
    }
    let knowledge = section
        .authoring_knowledge
        .associations
        .entry(association_id.into())
        .or_default();
    knowledge
        .navigation_unverified
        .retain(|id| id != property_id);
    if navigation == crate::NavigationKnowledge::Unverified {
        knowledge.navigation_unverified.push(property_id.into());
    }
    knowledge
        .navigation_unverified
        .sort_by_key(|id| member_ids.iter().position(|member| member == id));
    if knowledge.navigation_unverified.is_empty() {
        section
            .authoring_knowledge
            .associations
            .remove(association_id);
    }
    validate_navigation_declarations(
        &ReferenceIndex::new(Model::from_section(section)),
        association_id,
    )
}

pub fn association_of<'a>(
    references: &ReferenceIndex<'a>,
    property_id: &str,
) -> Result<Option<ElementRef<'a>>, String> {
    if !references.element(property_id)?.conforms_to("Property") {
        return Err("association requires a Property".into());
    }
    let mut found = None;
    for association in references.association_membership_candidates(property_id)? {
        member_ends(references, association.id())?;
        if found.replace(association).is_some() {
            return Err("Property::association has upper bound 1".into());
        }
    }
    Ok(found)
}

pub fn owning_association<'a>(
    references: &ReferenceIndex<'a>,
    property_id: &str,
) -> Result<Option<ElementRef<'a>>, String> {
    let property = references.element(property_id)?;
    if !property.conforms_to("Property") {
        return Err("owningAssociation requires a Property".into());
    }
    let Some(owner) = references
        .model()
        .owner(property)?
        .filter(|owner| owner.conforms_to("Association"))
    else {
        return Ok(None);
    };
    Ok(owned_ends(references, owner.id())?
        .iter()
        .any(|end| end.id == property_id)
        .then_some(owner))
}

pub fn opposite<'a>(
    references: &ReferenceIndex<'a>,
    property_id: &str,
) -> Result<Option<&'a Element>, String> {
    let Some(association) = association_of(references, property_id)? else {
        return Ok(None);
    };
    let ends = member_ends(references, association.id())?;
    Ok(if ends.len() == 2 {
        ends.into_iter().find(|end| end.id != property_id)
    } else {
        None
    })
}

pub fn property_type<'a>(
    references: &ReferenceIndex<'a>,
    property: &'a Element,
) -> Result<Option<ElementRef<'a>>, String> {
    if !ElementRef::Object(property).conforms_to("Property") {
        return Err("Property::type query requires a Property".into());
    }
    let metaclass = ElementRef::Object(property)
        .metaclass()
        .ok_or("Property metaclass is missing")?;
    let descriptor = schema::uml_property(metaclass, "type")
        .map_err(|error| error.to_string())?
        .ok_or("Property::type contract is missing")?;
    references.validate(
        descriptor,
        property.properties.get("type").unwrap_or(&Value::Null),
    )
}

/// UML Association-endType.1-spec: all member Types, including classifier-owned ends, as a set.
pub fn end_types<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<ElementRef<'a>>, String> {
    let mut seen = HashSet::new();
    let mut types = Vec::new();
    for end in member_ends(references, association_id)? {
        if let Some(target) = property_type(references, end)?
            && seen.insert(target.id())
        {
            types.push(target);
        }
    }
    Ok(types)
}

pub fn aggregation(property: &Element) -> Result<&str, String> {
    if !ElementRef::Object(property).conforms_to("Property") {
        return Err("Property::aggregation query requires a Property".into());
    }
    let value = property.properties.get("aggregation");
    let kind = value.and_then(Value::as_str).unwrap_or("none");
    if value.is_some_and(|value| !value.is_string())
        || !matches!(kind, "none" | "shared" | "composite")
    {
        return Err("Property::aggregation requires an AggregationKind".into());
    }
    Ok(kind)
}

pub fn is_composite(property: &Element) -> Result<bool, String> {
    Ok(aggregation(property)? == "composite")
}

pub fn multiplicity(
    references: &ReferenceIndex<'_>,
    property: &Element,
) -> Result<MultiplicityRange, EvaluationError> {
    value_specifications::multiplicity(references, ElementRef::Object(property))
}

struct MemberEndEvidence<'a> {
    known: Vec<(usize, &'a Element)>,
    cardinality: usize,
    unresolved: Option<classifiers::QueryError>,
}

fn member_end_evidence<'a>(
    references: &ReferenceIndex<'a>,
    association: ElementRef<'a>,
) -> Result<MemberEndEvidence<'a>, classifiers::QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    use classifiers::QueryError;
    let property = schema::uml_property("Association", "memberEnd")
        .map_err(|error| QueryError::unsupported(association.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(association.id(), "missing memberEnd contract"))?;
    let value = association.properties().get("memberEnd").ok_or_else(|| {
        QueryError::unsupported(
            association.id(),
            "memberEnd identities are not materialized",
        )
    })?;
    let mut unresolved = match check_value(references, association, property, value) {
        PropertyOutcome::Satisfied => None,
        PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(association.id(), message));
        }
        PropertyOutcome::Unverified(message) => {
            Some(QueryError::unsupported(association.id(), message))
        }
    };
    let values = value
        .as_array()
        .ok_or_else(|| QueryError::invalid(association.id(), "memberEnd must be a collection"))?;
    let member = schema::PropertyDescriptor {
        lower: "1",
        upper: "1",
        ..*property
    };
    let mut known = Vec::new();
    for (position, value) in values.iter().enumerate() {
        match check_value(references, association, &member, value) {
            PropertyOutcome::Satisfied => {}
            PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(association.id(), message));
            }
            PropertyOutcome::Unverified(message) => {
                unresolved
                    .get_or_insert_with(|| QueryError::unsupported(association.id(), message));
                continue;
            }
        }
        match references
            .validate(&member, value)
            .map_err(|error| QueryError::invalid(association.id(), error))?
        {
            Some(ElementRef::Object(property)) => known.push((position, property)),
            _ => {
                return Err(QueryError::invalid(
                    association.id(),
                    "memberEnd requires a Property object identity",
                ));
            }
        }
    }
    Ok(MemberEndEvidence {
        known,
        cardinality: values.len(),
        unresolved,
    })
}

// UML.xmi Property-type_of_opposite_end compares the inverse attribute role,
// not just the canonical owner. The caller establishes a binary memberEnd set;
// neither applicability nor equality depends on this end's own Type.
fn property_type_of_opposite_end(
    references: &ReferenceIndex<'_>,
    end: ElementRef<'_>,
    opposite: Option<ElementRef<'_>>,
) -> Result<bool, classifiers::QueryError> {
    use super::feature_compatibility::{accept_any, resolve_feature_type};
    use classifiers::QueryError;
    let owner = references
        .model()
        .owner(end)
        .map_err(|error| QueryError::invalid(end.id(), error))?;
    if owner.is_some_and(|owner| owner.metaclass().is_none()) {
        return Err(QueryError::unsupported(
            end.id(),
            "member end owner has no resolved UML metaclass",
        ));
    }
    let owning_association = match owner.filter(|owner| owner.conforms_to("Association")) {
        Some(owner) => classifiers::contributes_to_owned_union(owner, end, "Association-ownedEnd"),
        None => Ok(false),
    };
    if matches!(owning_association, Ok(true)) {
        return Ok(true);
    }
    let classifier = match owner.filter(|owner| owner.conforms_to("Classifier")) {
        Some(owner) => {
            classifiers::is_owned_attribute(owner, end).map(|attribute| attribute.then_some(owner))
        }
        None => Ok(None),
    };
    let opposite_type = match opposite {
        Some(opposite) if opposite.properties().get("type").is_none_or(Value::is_null) => Ok(None),
        Some(opposite) => resolve_feature_type(references, opposite).map(Some),
        None => Err(QueryError::unsupported(
            end.id(),
            "opposite member end is unresolved",
        )),
    };
    let same_classifier = (|| {
        for result in [&classifier, &opposite_type] {
            if let Err(error @ QueryError::Invalid { .. }) = result {
                return Err(error.clone());
            }
        }
        Ok(classifier?.map(|classifier| classifier.id())
            == opposite_type?.map(|opposite_type| opposite_type.id()))
    })();
    accept_any([owning_association, same_classifier])
}

// UML.xmi Association-ends_must_be_typed, Association-binary_associations,
// Association-association_ends and Property-type_of_opposite_end.
// A quantified violation needs only a local witness, not a complete end query.
fn association_end_constraints(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Vec<StructuralDiagnostic> {
    use classifiers::QueryError;
    let evidence = match member_end_evidence(references, association) {
        Ok(evidence) => evidence,
        Err(error) => {
            let (severity, code) = match &error {
                QueryError::Invalid { .. } => (StructuralSeverity::Error, "UML_ASSOCIATION_ENDS"),
                QueryError::Unsupported { .. } => {
                    (StructuralSeverity::Warning, "MDE_ASSOCIATION_UNSUPPORTED")
                }
            };
            return vec![StructuralDiagnostic {
                severity,
                code,
                subject_id: association.id().into(),
                message: error.to_string(),
            }];
        }
    };
    let typed = super::feature_compatibility::require_all(
        evidence
            .unresolved
            .clone()
            .map(Err)
            .into_iter()
            .chain(evidence.known.iter().map(|(_, end)| {
                if end.properties.get("type").is_none_or(Value::is_null) {
                    Ok(false)
                } else {
                    super::feature_compatibility::resolve_feature_type(
                        references,
                        ElementRef::Object(end),
                    )
                    .map(|_| true)
                }
            })),
    );
    let binary = (|| {
        let aggregations = evidence
            .known
            .iter()
            .map(|(_, end)| aggregation(end).map_err(|error| QueryError::invalid(&end.id, error)))
            .collect::<Result<Vec<_>, _>>()?;
        let aggregate_count = aggregations.iter().filter(|kind| **kind != "none").count();
        // Two distinct local aggregates can never satisfy the binary rule.
        // Three known ends also disprove size = 2. External identities are not
        // counted as distinct local ends without a resolver.
        if aggregate_count > 1 || (aggregate_count > 0 && evidence.known.len() > 2) {
            return Ok(false);
        }
        if let Some(error) = &evidence.unresolved {
            return Err(error.clone());
        }
        Ok(aggregate_count == 0 || evidence.known.len() == 2)
    })();
    let nary_ownership = if evidence.cardinality <= 2 {
        Ok(true)
    } else {
        super::feature_compatibility::require_all(
            evidence
                .unresolved
                .clone()
                .map(Err)
                .into_iter()
                .chain(evidence.known.iter().map(|(_, end)| {
                    let end = ElementRef::Object(end);
                    let owner = references
                        .model()
                        .owner(end)
                        .map_err(|error| QueryError::invalid(end.id(), error))?;
                    if owner.is_none_or(|owner| owner.id() != association.id()) {
                        return Ok(false);
                    }
                    // Identity alone is insufficient for AssociationClass:
                    // ownedAttribute and ownedEnd are disjoint composite roles.
                    classifiers::contributes_to_owned_union(
                        association,
                        end,
                        "Association-ownedEnd",
                    )
                })),
        )
    };
    let opposite_types = if evidence.cardinality != 2 {
        Ok(true)
    } else {
        super::feature_compatibility::require_all(
            evidence
                .unresolved
                .clone()
                .map(Err)
                .into_iter()
                .chain(evidence.known.iter().map(|(position, end)| {
                    let opposite = evidence
                        .known
                        .iter()
                        .find(|(other_position, _)| *other_position == 1 - *position)
                        .map(|(_, opposite)| ElementRef::Object(opposite));
                    property_type_of_opposite_end(references, ElementRef::Object(end), opposite)
                })),
        )
    };
    [
        (
            "Association::ends_must_be_typed",
            "UML_ASSOCIATION_ENDS_MUST_BE_TYPED",
            "MDE_ASSOCIATION_END_TYPES_UNVERIFIED",
            "every member end must have a Type",
            typed,
        ),
        (
            "Association::binary_associations",
            "UML_ASSOCIATION_BINARY_ASSOCIATIONS",
            "MDE_ASSOCIATION_AGGREGATION_UNVERIFIED",
            "aggregation is permitted only on one end of a binary Association",
            binary,
        ),
        (
            "Association::association_ends",
            "UML_ASSOCIATION_NARY_END_OWNERSHIP",
            "MDE_ASSOCIATION_NARY_END_OWNERSHIP_UNVERIFIED",
            "all n-ary member ends must be Association-owned ends",
            nary_ownership,
        ),
        (
            "Property::type_of_opposite_end",
            "UML_ASSOCIATION_ENDS",
            "MDE_ASSOCIATION_UNSUPPORTED",
            "a classifier-owned end's classifier must equal the opposite end Type",
            opposite_types,
        ),
    ]
    .into_iter()
    .filter_map(|(rule, code, unknown, violation, outcome)| {
        let (severity, code, message) = match outcome {
            Ok(true) => return None,
            Ok(false) => (StructuralSeverity::Error, code, violation.to_owned()),
            Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, code, message),
            Err(QueryError::Unsupported { message, .. }) => {
                (StructuralSeverity::Warning, unknown, message)
            }
        };
        Some(StructuralDiagnostic {
            severity,
            code,
            subject_id: association.id().into(),
            message: format!("{rule}: {message}"),
        })
    })
    .collect()
}

fn validate_association(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Result<(), String> {
    let end_issues = association_end_constraints(references, association);
    let structure = if member_ends(references, association.id()).is_ok() {
        validate_association_structure(references, association, end_issues.is_empty())
    } else {
        Ok(())
    };
    if let Some(issue) = end_issues
        .iter()
        .find(|issue| issue.severity == StructuralSeverity::Error)
    {
        return Err(issue.message.clone());
    }
    let mut unresolved = end_issues
        .first()
        .map(|issue| format!("Unsupported {}", issue.message));
    for result in [structure, validate_specialization(references, association)] {
        match result {
            Err(message) if message.starts_with("Unsupported ") => {
                unresolved.get_or_insert(message);
            }
            Err(message) => return Err(message),
            Ok(()) => {}
        }
    }
    unresolved.map_or(Ok(()), Err)
}

fn validate_association_structure(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
    evaluate_type_constraints: bool,
) -> Result<(), String> {
    if association.properties().contains_key("endType") {
        return Err(
            "Association::endType must be derived from canonical memberEnd and owns edges".into(),
        );
    }
    let ends = member_ends(references, association.id())?;
    owned_ends(references, association.id())?;
    validate_navigation_declarations(references, association.id())?;
    let mut unresolved = None;
    for (index, end) in ends.iter().enumerate() {
        let owner = references
            .model()
            .owner(ElementRef::Object(end))?
            .ok_or("Association member end requires an owner")?;
        if owner.id() != association.id() && owner.metaclass().is_none() {
            unresolved.get_or_insert_with(|| {
                format!(
                    "Unsupported Association member end owner {} has no resolved UML metaclass",
                    owner.id(),
                )
            });
        } else if owner.id() != association.id() {
            if owner.conforms_to("Association")
                && classifiers::contributes_to_owned_union(
                    owner,
                    ElementRef::Object(end),
                    "Association-ownedEnd",
                )
                .map_err(|error| error.to_string())?
            {
                return Err("Property::owningAssociation must subset Property::association; an end owned by another Association cannot be a member here".into());
            }
            if !owner.conforms_to("Classifier") {
                return Err(
                    "Association member end must be owned by its Association or a Classifier"
                        .into(),
                );
            }
        }
        match multiplicity(references, end) {
            Ok(_) => {}
            Err(error) if error.is_unsupported() => {}
            Err(error) => return Err(format!("Property {} multiplicity: {error}", end.id)),
        }
        // opposite is defined only for binary Associations. The independent
        // binary_associations predicate diagnoses aggregation on n-ary ends.
        if is_composite(end)? && ends.len() == 2 {
            let other = ends
                .get(1 - index)
                .ok_or("composite Association must be binary")?;
            // UML Property-multiplicity_of_composite needs only upperBound;
            // neither the opposite Type nor an unevaluated lower bound matters.
            let upper = match value_specifications::upper_bound(
                references,
                ElementRef::Object(other),
            ) {
                Ok(upper) => upper,
                Err(error) if error.is_unsupported() => {
                    unresolved.get_or_insert_with(|| format!(
                        "Unsupported Property::multiplicity_of_composite evaluation of opposite bound: {error}"
                    ));
                    continue;
                }
                Err(error) => {
                    return Err(format!(
                        "Property::multiplicity_of_composite invalid opposite bound: {error}"
                    ));
                }
            };
            if upper
                .finite_value()
                .is_none_or(|upper| upper > &Integer::from(1))
            {
                return Err("Property::multiplicity_of_composite requires the opposite end upper bound to be at most 1".into());
            }
        }
    }
    // Ownership, multiplicity and composition above are independent of Type
    // resolution. Only the remaining projections/comparisons require all Types.
    if !evaluate_type_constraints {
        return unresolved.map_or(Ok(()), Err);
    }
    if let ElementRef::Relationship(relation) = association {
        if ends.len() != 2 {
            return Err("Unsupported binary Relation notation: use an n-ary Association element for more than two member ends".into());
        }
        let projection = binary_projection(references, relation)?;
        let unchecked_ranges = ends
            .iter()
            .zip(["sourceMultiplicity", "targetMultiplicity"])
            .filter(|(end, _)| {
                value_specifications::has_bound_references(ElementRef::Object(end))
                    && multiplicity(references, end).is_err_and(|error| error.is_unsupported())
            })
            .map(|(_, key)| key)
            .collect::<HashSet<_>>();
        if relation.from != projection.from
            || relation.to != projection.to
            || PROJECTION_KEYS.iter().any(|key| {
                !unchecked_ranges.contains(key)
                    && relation.properties.get(*key) != projection.properties.get(*key)
            })
        {
            return Err(
                "Association binary projection is inconsistent with canonical memberEnd Properties"
                    .into(),
            );
        }
    }
    if association.conforms_to("AssociationClass") {
        match association_class_cannot_be_defined(references, association) {
            Ok(true) => {}
            Ok(false) => return Err("AssociationClass-cannot_be_defined: endType or an end Type's allParents contains this AssociationClass".into()),
            Err(classifiers::QueryError::Invalid { message, .. }) => return Err(format!("AssociationClass-cannot_be_defined: {message}")),
            Err(error) => return Err(format!("Unsupported AssociationClass-cannot_be_defined: {error}")),
        }
    }
    if association.conforms_to("CommunicationPath") {
        match communication_path_association_ends(references, association) {
            Ok(true) => {}
            Ok(false) => {
                return Err(
                    "CommunicationPath-association_ends: end Types must be DeploymentTargets"
                        .into(),
                );
            }
            Err(classifiers::QueryError::Invalid { message, .. }) => {
                return Err(format!("CommunicationPath-association_ends: {message}"));
            }
            Err(error) => {
                return Err(format!(
                    "Unsupported CommunicationPath-association_ends: {error}"
                ));
            }
        }
    }
    // UML 2.5.1 §12.4.1.6 and UML.xmi both cast every memberEnd to
    // ExtensionEnd inside metaclassEnd(), including its non-owned Property.
    // Do not silently replace that narrowing cast with identity subtraction.
    if association.conforms_to("Extension") {
        return Err("Unsupported Extension-non_owned_end: published metaclassEnd() narrows all member ends to ExtensionEnd; the non-owned Property case requires normative clarification".into());
    }
    // CommunicationPath has one own constraint and no own properties/operations
    // in UML.xmi; all inherited Association checks have already run above.
    if !matches!(
        association.metaclass(),
        Some("Association" | "CommunicationPath")
    ) {
        return Err(format!(
            "Unsupported {} constraints: inherited Association rules were checked, but specialized metaclass contracts are not implemented",
            association.metaclass().unwrap_or("Association subtype"),
        ));
    }
    unresolved.map_or(Ok(()), Err)
}

fn validate_specialization(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Result<(), String> {
    // UML Association-specialized_end_number/types quantify immediate parents,
    // not their complete transitive closure. Reuse the hierarchy's checked
    // direct evidence without claiming that unresolved parents are absent.
    let evidence = classifier_hierarchy::HierarchyQueries::new(references)
        .parent_evidence(association.id())
        .map_err(|error| error.to_string())?;
    let mut unsupported = evidence.unresolved.map(|error| error.to_string());
    let parents = evidence
        .parents
        .into_iter()
        .filter(|parent| parent.conforms_to("Association"))
        .collect::<Vec<_>>();
    if parents.is_empty() {
        return unsupported.map_or(Ok(()), Err);
    }
    let ends = member_end_evidence(references, association).map_err(|error| error.to_string())?;
    if let Some(error) = &ends.unresolved {
        unsupported.get_or_insert_with(|| error.to_string());
    }
    for general in parents {
        let general_ends = match member_end_evidence(references, general) {
            Ok(ends) => ends,
            Err(error @ classifiers::QueryError::Unsupported { .. }) => {
                unsupported.get_or_insert_with(|| error.to_string());
                continue;
            }
            Err(error) => return Err(error.to_string()),
        };
        if let Some(error) = &general_ends.unresolved {
            unsupported.get_or_insert_with(|| error.to_string());
        }
        // A well-formed ordered reference collection has a represented size
        // even when some targets are external. Never collapse unknown slots.
        if ends.cardinality != general_ends.cardinality {
            return Err(format!(
                "Association::specialized_end_number requires the same number of member ends as general {} ({} instead of {})",
                general.id(),
                ends.cardinality,
                general_ends.cardinality,
            ));
        }
        for (position, specific) in &ends.known {
            let Some((_, general_end)) = general_ends
                .known
                .iter()
                .find(|(general_position, _)| general_position == position)
            else {
                continue;
            };
            match compare_type_conformance(
                references,
                ElementRef::Object(specific),
                ElementRef::Object(general_end),
            ) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(format!(
                        "Association::specialized_end_types requires end {} ({}) to conform to the corresponding end {} of general {}",
                        position + 1,
                        specific.id,
                        general_end.id,
                        general.id(),
                    ));
                }
                Err(error @ classifiers::QueryError::Unsupported { .. }) => {
                    unsupported.get_or_insert_with(|| error.to_string());
                }
                Err(error) => return Err(error.to_string()),
            }
        }
    }
    unsupported.map_or(Ok(()), Err)
}

// UML.xmi AssociationClass-disjoint_attributes_ends intersects owned roles,
// not the complete memberEnd collection. Unknown neighbors cannot erase a
// Property whose canonical owner and both local roles are already established.
fn association_class_disjoint_attributes_ends(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Result<bool, classifiers::QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    use classifiers::QueryError;
    let mut checks = Vec::new();
    for property in classifiers::properties(association)? {
        if property.aggregation != schema::Aggregation::Composite || property.is_derived {
            continue;
        }
        let attribute = classifiers::contributes(property, "Classifier-attribute")
            .map_err(|error| QueryError::unsupported(association.id(), error))?;
        let end = classifiers::contributes(property, "Association-ownedEnd")
            .map_err(|error| QueryError::unsupported(association.id(), error))?;
        if !attribute && !end {
            continue;
        }
        if let Some(value) = association.properties().get(property.name) {
            match check_value(references, association, property, value) {
                PropertyOutcome::Satisfied => {}
                PropertyOutcome::Violation(message) => {
                    checks.push(Err(QueryError::invalid(association.id(), message)))
                }
                PropertyOutcome::Unverified(message) => {
                    checks.push(Err(QueryError::unsupported(association.id(), message)))
                }
            }
        }
    }
    for child in references
        .explicit_owned_elements(association.id())
        .map_err(|error| QueryError::invalid(association.id(), error))?
    {
        if child.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(
                child.id(),
                "owned member metaclass is unresolved",
            )));
        } else if child.conforms_to("Property") {
            checks.push(
                super::feature_compatibility::require_all([
                    classifiers::is_owned_attribute(association, child),
                    classifiers::contributes_to_owned_union(
                        association,
                        child,
                        "Association-ownedEnd",
                    ),
                ])
                .map(|intersection| !intersection),
            );
        }
    }
    super::feature_compatibility::require_all(checks)
}

// UML.xmi AssociationClass-cannot_be_defined. Evaluate known ends even when
// another member is unresolved; this is a quantified rule, not a partial query
// pretending to return the complete endType collection.
fn association_class_cannot_be_defined(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Result<bool, classifiers::QueryError> {
    use super::property_contracts::PropertyOutcome;
    use classifiers::QueryError;
    let property = schema::uml_property("Association", "memberEnd")
        .map_err(|error| QueryError::unsupported(association.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(association.id(), "missing memberEnd contract"))?;
    let value = association.properties().get("memberEnd").ok_or_else(|| {
        QueryError::unsupported(
            association.id(),
            "memberEnd identities are not materialized",
        )
    })?;
    let mut checks = Vec::new();
    match super::property_contracts::check_value(references, association, property, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(association.id(), message));
        }
        PropertyOutcome::Unverified(message) => {
            checks.push(Err(QueryError::unsupported(association.id(), message)))
        }
    }
    let values = value
        .as_array()
        .ok_or_else(|| QueryError::invalid(association.id(), "memberEnd must be a collection"))?;
    let member = schema::PropertyDescriptor {
        lower: "1",
        upper: "1",
        ..*property
    };
    for value in values {
        checks.push((|| {
            match super::property_contracts::check_value(references, association, &member, value) {
                PropertyOutcome::Satisfied => {}
                PropertyOutcome::Violation(message) => {
                    return Err(QueryError::invalid(association.id(), message));
                }
                PropertyOutcome::Unverified(message) => {
                    return Err(QueryError::unsupported(association.id(), message));
                }
            }
            let end = references
                .validate(&member, value)
                .map_err(|error| QueryError::invalid(association.id(), error))?
                .ok_or_else(|| {
                    QueryError::invalid(association.id(), "memberEnd cannot be absent")
                })?;
            // Optional absent Types do not contribute to the derived endType
            // collection; Association-ends_must_be_typed diagnoses them separately.
            if end.properties().get("type").is_none_or(Value::is_null) {
                return Ok(true);
            }
            let endpoint = super::feature_compatibility::resolve_feature_type(references, end)?;
            if endpoint.id() == association.id() {
                return Ok(false);
            }
            classifier_hierarchy::has_ancestor(references, endpoint.id(), association.id())
                .map(|contains_self| !contains_self)
        })());
    }
    super::feature_compatibility::require_all(checks)
}

// This checks the quantified constraint using local evidence without claiming
// that unresolved memberEnd or Type references form a complete endType set.
fn communication_path_association_ends(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Result<bool, classifiers::QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    use classifiers::QueryError;
    let property = schema::uml_property("Association", "memberEnd")
        .map_err(|error| QueryError::unsupported(association.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(association.id(), "missing memberEnd contract"))?;
    let value = association.properties().get("memberEnd").ok_or_else(|| {
        QueryError::unsupported(
            association.id(),
            "memberEnd identities are not materialized",
        )
    })?;
    let mut checks = Vec::new();
    match check_value(references, association, property, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(association.id(), message));
        }
        PropertyOutcome::Unverified(message) => {
            checks.push(Err(QueryError::unsupported(association.id(), message)))
        }
    }
    let values = value
        .as_array()
        .ok_or_else(|| QueryError::invalid(association.id(), "memberEnd must be a collection"))?;
    let member = schema::PropertyDescriptor {
        lower: "1",
        upper: "1",
        ..*property
    };
    for value in values {
        checks.push((|| {
            match check_value(references, association, &member, value) {
                PropertyOutcome::Satisfied => {}
                PropertyOutcome::Violation(message) => {
                    return Err(QueryError::invalid(association.id(), message));
                }
                PropertyOutcome::Unverified(message) => {
                    return Err(QueryError::unsupported(association.id(), message));
                }
            }
            let end = references
                .validate(&member, value)
                .map_err(|error| QueryError::invalid(association.id(), error))?
                .ok_or_else(|| {
                    QueryError::invalid(association.id(), "memberEnd cannot be absent")
                })?;
            // endType excludes absent optional Types. The inherited rule
            // Association-ends_must_be_typed independently rejects untyped ends.
            let Some(value) = end
                .properties()
                .get("type")
                .filter(|value| !value.is_null())
            else {
                return Ok(true);
            };
            let metaclass = end.metaclass().ok_or_else(|| {
                QueryError::unsupported(end.id(), "memberEnd metaclass is unresolved")
            })?;
            let property = schema::uml_property(metaclass, "type")
                .map_err(|error| QueryError::unsupported(end.id(), error.to_string()))?
                .ok_or_else(|| QueryError::unsupported(end.id(), "missing Type contract"))?;
            match check_value(references, end, property, value) {
                PropertyOutcome::Satisfied => {}
                PropertyOutcome::Violation(message) => {
                    return Err(QueryError::invalid(end.id(), message));
                }
                PropertyOutcome::Unverified(message) => {
                    return Err(QueryError::unsupported(end.id(), message));
                }
            }
            let target = references
                .validate(property, value)
                .map_err(|error| QueryError::invalid(end.id(), error))?
                .ok_or_else(|| QueryError::invalid(end.id(), "nonempty Type did not resolve"))?;
            Ok(target.conforms_to("DeploymentTarget"))
        })());
    }
    super::feature_compatibility::require_all(checks)
}

fn issue(code: &'static str, id: &str, message: impl Into<String>) -> StructuralDiagnostic {
    StructuralDiagnostic {
        severity: StructuralSeverity::Error,
        code,
        subject_id: id.into(),
        message: message.into(),
    }
}

const PROJECTION_KEYS: [&str; 6] = crate::contract_projection::ASSOCIATION_DISPLAY_KEYS;

fn materialized_ends<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<&'a Element>, String> {
    let association = references.element(association_id)?;
    let values = association
        .properties()
        .get("memberEnd")
        .and_then(Value::as_array)
        .ok_or("Association memberEnd must be an explicit collection")?;
    values
        .iter()
        .map(|value| {
            let id = crate::contract_projection::projection_reference_id(value)
                .ok_or("Association memberEnd requires an identity")?;
            match references.element(id)? {
                ElementRef::Object(element) => Ok(element),
                _ => Err("Association end must be an object projection".into()),
            }
        })
        .collect()
}

fn binary_projection(
    references: &ReferenceIndex<'_>,
    relation: &Relation,
) -> Result<Relation, String> {
    let ends = materialized_ends(references, &relation.id)?;
    let [source, target] = ends.as_slice() else {
        return Err("Binary association projection requires exactly two member ends".into());
    };
    let mut result = relation.clone();
    let type_id = |end: &Element| -> Result<String, String> {
        end.properties
            .get("type")
            .and_then(crate::contract_projection::projection_reference_id)
            .map(str::to_owned)
            .ok_or_else(|| format!("{}: association endpoint type is absent", end.id))
    };
    result.from = type_id(source)?;
    result.to = type_id(target)?;
    for key in PROJECTION_KEYS {
        result.properties.remove(key);
    }
    result
        .properties
        .extend(crate::contract_projection::association_display_properties(
            source,
            target,
            |id| match references.model().element(id) {
                Some(ElementRef::Object(element)) => Some(element),
                _ => None,
            },
        ));
    Ok(result)
}

pub(crate) fn synchronize_projection(
    section: &mut ModelingSection,
    association_id: &str,
) -> Result<(), String> {
    let Some(relation) = section
        .relations
        .iter()
        .find(|relation| relation.id == association_id)
    else {
        return Ok(());
    };
    let projection =
        binary_projection(&ReferenceIndex::new(Model::from_section(section)), relation)?;
    let relation = section
        .relations
        .iter_mut()
        .find(|relation| relation.id == association_id)
        .ok_or("Association disappeared")?;
    let endpoints_changed = relation.from != projection.from || relation.to != projection.to;
    *relation = projection;
    if endpoints_changed {
        let diagrams = section
            .diagrams
            .iter()
            .filter(|diagram| {
                diagram
                    .views
                    .iter()
                    .any(|view| view.subject.as_deref() == Some(association_id))
            })
            .map(|diagram| diagram.id.clone())
            .collect::<Vec<_>>();
        for diagram in diagrams {
            crate::reroute_relationships(section, &diagram);
        }
    }
    Ok(())
}

fn fresh_id(section: &ModelingSection, project_id: &str, prefix: &str) -> Result<String, String> {
    for sequence in 1_u64.. {
        let id = format!("{prefix}-{sequence}");
        if id != project_id
            && !section.elements.iter().any(|element| element.id == id)
            && !section.relations.iter().any(|relation| relation.id == id)
            && !section
                .diagrams
                .iter()
                .any(|diagram| diagram.id == id || diagram.views.iter().any(|view| view.id == id))
        {
            return Ok(id);
        }
    }
    Err("identifier overflow".into())
}

pub(crate) fn migrate(
    section: &mut ModelingSection,
    project_id: &str,
    association_id: &str,
) -> Result<(), String> {
    let mut staged = section.clone();
    materialize_binary(&mut staged, project_id, association_id)?;
    *section = staged;
    Ok(())
}

fn materialize_binary(
    section: &mut ModelingSection,
    project_id: &str,
    association_id: &str,
) -> Result<(), String> {
    let relation = section
        .relations
        .iter()
        .find(|relation| relation.id == association_id)
        .ok_or("Legacy binary Association relation not found")?
        .clone();
    if super::root::relationship_metaclass(&relation.kind) != Some("Association") {
        return Err(
            "Only legacy binary Association notation can be migrated by this command".into(),
        );
    }
    if relation.properties.contains_key("memberEnd") {
        // Existing display values are checked against their canonical inputs,
        // never silently discarded as if they were unimportant metadata.
        let projection = binary_projection(
            &ReferenceIndex::new(Model::from_section(section)),
            &relation,
        )?;
        for key in PROJECTION_KEYS {
            if let Some(value) = relation.properties.get(key)
                && projection.properties.get(key) != Some(value)
            {
                return Err(format!(
                    "{association_id}: {key} conflicts with canonical memberEnd data"
                ));
            }
        }
        return synchronize_projection(section, association_id);
    }
    for key in ["ownedEnd", "endType", "navigableOwnedEnd"] {
        if relation.properties.contains_key(key) {
            return Err(format!(
                "Association already contains {key} without memberEnd; reconcile its references explicitly"
            ));
        }
    }
    let mut ends = Vec::new();
    for (endpoint, role, range, diamond) in [
        (
            &relation.from,
            "sourceRole",
            "sourceMultiplicity",
            "end2Aggregation",
        ),
        (
            &relation.to,
            "targetRole",
            "targetMultiplicity",
            "end1Aggregation",
        ),
    ] {
        let name = match relation.properties.get(role) {
            None => String::new(),
            Some(Value::String(name)) => name.clone(),
            Some(_) => {
                return Err(format!(
                    "Cannot migrate non-string {role} without losing authored data"
                ));
            }
        };
        let id = fresh_id(section, project_id, "mde-association-end")?;
        let mut properties = BTreeMap::from([("type".into(), json!({"$ref": endpoint}))]);
        properties.insert("association".into(), json!({"$ref": association_id}));
        properties.insert("owningAssociation".into(), json!({"$ref": association_id}));
        if let Some(value) = relation.properties.get(diamond) {
            properties.insert("aggregation".into(), value.clone());
        }
        section.elements.push(Element {
            id: id.clone(),
            kind: "uml.attribute".into(),
            name,
            label: None,
            description: None,
            properties,
            annotations: Vec::new(),
            implementations: Vec::new(),
            content: None,
        });
        if let Some(value) = relation.properties.get(range) {
            set_end_multiplicity(
                section,
                project_id,
                &id,
                value
                    .as_str()
                    .ok_or_else(|| format!("{range} must be a multiplicity string"))?,
            )?;
        }
        let ownership_id = fresh_id(section, project_id, "mde-association-owns")?;
        section.relations.push(Relation {
            id: ownership_id,
            kind: "owns".into(),
            from: association_id.into(),
            to: id.clone(),
            properties: BTreeMap::from([("role".into(), json!("ownedEnd"))]),
        });
        ends.push(json!({"$ref": id}));
    }
    let association = section
        .relations
        .iter_mut()
        .find(|relation| relation.id == association_id)
        .ok_or("Association disappeared")?;
    association
        .properties
        .insert("memberEnd".into(), Value::Array(ends.clone()));
    association
        .properties
        .insert("ownedEnd".into(), Value::Array(ends));
    for key in PROJECTION_KEYS {
        association.properties.remove(key);
    }
    synchronize_projection(section, association_id)
}

pub(crate) fn set_member_ends(
    section: &mut ModelingSection,
    association_id: &str,
    ends: &[String],
) -> Result<(), String> {
    let value = Value::Array(ends.iter().map(|id| json!({"$ref": id})).collect());
    for id in ends {
        if !section.elements.iter().any(|element| element.id == *id) {
            return Err(format!("Association end {id} not found"));
        }
    }
    if section
        .relations
        .iter()
        .any(|relation| relation.id == association_id)
        && ends.len() != 2
    {
        return Err("Binary association projection requires exactly two member ends; use an Association object for other arities".into());
    }
    if let Some(relation) = section
        .relations
        .iter_mut()
        .find(|relation| relation.id == association_id)
    {
        relation.properties.insert("memberEnd".into(), value);
    } else {
        section
            .elements
            .iter_mut()
            .find(|element| element.id == association_id)
            .ok_or("Association not found")?
            .properties
            .insert("memberEnd".into(), value);
    }
    let owned: Vec<String> = section
        .relations
        .iter()
        .filter(|relation| {
            relation.kind == "owns"
                && relation.from == association_id
                && ends.contains(&relation.to)
        })
        .map(|relation| relation.to.clone())
        .collect();
    for element in &mut section.elements {
        if ends.contains(&element.id) {
            element
                .properties
                .insert("association".into(), json!({"$ref": association_id}));
            element.properties.insert(
                "owningAssociation".into(),
                if owned.contains(&element.id) {
                    json!({"$ref": association_id})
                } else {
                    Value::Null
                },
            );
        } else if element
            .properties
            .get("association")
            .and_then(crate::contract_projection::projection_reference_id)
            == Some(association_id)
        {
            element.properties.insert("association".into(), Value::Null);
            if element
                .properties
                .get("owningAssociation")
                .and_then(crate::contract_projection::projection_reference_id)
                == Some(association_id)
            {
                element
                    .properties
                    .insert("owningAssociation".into(), Value::Null);
            }
        }
    }
    let owned_value = json!(
        owned
            .iter()
            .map(|id| json!({"$ref": id}))
            .collect::<Vec<_>>()
    );
    if let Some(relation) = section
        .relations
        .iter_mut()
        .find(|relation| relation.id == association_id)
    {
        relation.properties.insert("ownedEnd".into(), owned_value);
    } else if let Some(element) = section
        .elements
        .iter_mut()
        .find(|element| element.id == association_id)
    {
        element.properties.insert("ownedEnd".into(), owned_value);
    }
    synchronize_projection(section, association_id)?;
    Ok(())
}

/// Compatibility field writes reach the canonical Property, never a second writable end.
pub(crate) fn set_projection_property(
    section: &mut ModelingSection,
    association_id: &str,
    path: &str,
    value: &Value,
) -> Result<bool, String> {
    let Some(key) = path.strip_prefix("properties.") else {
        return Ok(false);
    };
    let Some((index, property)) = (match key {
        "sourceRole" => Some((0, "name")),
        "targetRole" => Some((1, "name")),
        "sourceMultiplicity" => Some((0, "multiplicity")),
        "targetMultiplicity" => Some((1, "multiplicity")),
        "end1Aggregation" => Some((1, "aggregation")),
        "end2Aggregation" => Some((0, "aggregation")),
        _ => None,
    }) else {
        return Ok(false);
    };
    let references = ReferenceIndex::new(Model::from_section(section));
    let Ok(association) = references.element(association_id) else {
        return Ok(false);
    };
    if !matches!(association, ElementRef::Relationship(_))
        || !association.conforms_to("Association")
        || !association.properties().contains_key("memberEnd")
    {
        return Ok(false);
    }
    let id = materialized_ends(&references, association_id)?
        .get(index)
        .ok_or("Association member end is missing")?
        .id
        .clone();
    if property == "multiplicity" {
        set_end_multiplicity(
            section,
            "",
            &id,
            value.as_str().ok_or("expected multiplicity string")?,
        )?;
    } else {
        let end = section
            .elements
            .iter_mut()
            .find(|element| element.id == id)
            .ok_or("Association member end disappeared")?;
        if property == "name" {
            end.name = value.as_str().ok_or("expected string")?.into();
            end.properties.insert("name".into(), value.clone());
        } else {
            end.properties.insert(property.into(), value.clone());
        }
    }
    synchronize_projection(section, association_id)?;
    Ok(true)
}

pub(crate) fn is_projection_path(path: &str) -> bool {
    path.strip_prefix("properties.")
        .is_some_and(|key| PROJECTION_KEYS.contains(&key))
}

fn canonical_inputs_changed(
    before: &ModelingSection,
    after: &ModelingSection,
    association: ElementRef<'_>,
) -> bool {
    let previous_model = Model::from_section(before);
    let previous = previous_model.element(association.id());
    if before
        .authoring_knowledge
        .associations
        .get(association.id())
        != after.authoring_knowledge.associations.get(association.id())
    {
        return true;
    }
    if previous.is_none_or(|previous| {
        ["memberEnd", "ownedEnd", "navigableOwnedEnd"]
            .iter()
            .any(|key| previous.properties().get(*key) != association.properties().get(*key))
    }) {
        return true;
    }
    association
        .properties()
        .get("memberEnd")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .filter_map(|value| reference_id(value).ok().flatten())
                .any(|id| {
                    let previous = before.elements.iter().find(|element| element.id == id);
                    let current = after.elements.iter().find(|element| element.id == id);
                    let properties_changed = match (previous, current) {
                        (Some(previous), Some(current)) => {
                            previous.name != current.name
                                || previous.properties != current.properties
                        }
                        (None, None) => false,
                        _ => true,
                    };
                    let owners = |section: &ModelingSection| {
                        section
                            .relations
                            .iter()
                            .filter(|relation| relation.kind == "owns" && relation.to == id)
                            .map(|relation| relation.from.clone())
                            .collect::<Vec<_>>()
                    };
                    let bound_inputs_changed = [previous, current]
                        .into_iter()
                        .flatten()
                        .flat_map(|end| {
                            ["lowerValue", "upperValue"].into_iter().filter_map(|key| {
                                end.properties
                                    .get(key)
                                    .and_then(|value| reference_id(value).ok().flatten())
                            })
                        })
                        .any(|id| {
                            before.elements.iter().find(|element| element.id == id)
                                != after.elements.iter().find(|element| element.id == id)
                                || before
                                    .relations
                                    .iter()
                                    .filter(|relation| relation.kind == "owns" && relation.to == id)
                                    .collect::<Vec<_>>()
                                    != after
                                        .relations
                                        .iter()
                                        .filter(|relation| {
                                            relation.kind == "owns" && relation.to == id
                                        })
                                        .collect::<Vec<_>>()
                        });
                    properties_changed || bound_inputs_changed || owners(before) != owners(after)
                })
        })
}

/// Refresh only projections whose authoritative inputs changed. Unrelated legacy
/// records and inconsistent imported caches are not silently rewritten.
pub(crate) fn synchronize_changes(
    before: &ModelingSection,
    after: &mut ModelingSection,
) -> Result<(), String> {
    let changed = after
        .relations
        .iter()
        .filter(|relation| {
            super::root::relationship_metaclass(&relation.kind) == Some("Association")
                && relation.properties.contains_key("memberEnd")
                && canonical_inputs_changed(before, after, ElementRef::Relationship(relation))
        })
        .map(|relation| relation.id.clone())
        .collect::<Vec<_>>();
    for id in changed {
        synchronize_projection(after, &id)?;
    }
    Ok(())
}

pub(crate) fn materialize_new(before: &Project, after: &mut ModelingSection) -> Result<(), String> {
    let added = after
        .relations
        .iter()
        .filter(|relation| {
            super::root::relationship_metaclass(&relation.kind) == Some("Association")
                && !before
                    .relations
                    .iter()
                    .any(|previous| previous.id == relation.id)
        })
        .map(|relation| relation.id.clone())
        .collect::<Vec<_>>();
    for id in added {
        migrate(after, &before.id, &id)?;
    }
    Ok(())
}

pub(crate) fn set_end_multiplicity(
    section: &mut ModelingSection,
    project_id: &str,
    property_id: &str,
    notation: &str,
) -> Result<(), String> {
    let notation = notation.trim();
    let (lower, upper) = if notation == "*" {
        ("0", "*")
    } else {
        notation.split_once("..").unwrap_or((notation, notation))
    };
    let lower: Integer = lower.trim().parse().map_err(|error| {
        format!("Cannot translate lower bound {lower:?} into a LiteralInteger: {error}")
    })?;
    let upper: UnlimitedNatural = upper.trim().parse().map_err(|error| {
        format!("Cannot translate upper bound {upper:?} into a LiteralUnlimitedNatural: {error}")
    })?;
    // Stage both identities together: rejecting a nonliteral/shared upper bound
    // must not leave a successfully changed lower bound behind.
    let mut staged = section.clone();
    for (field, kind, value) in [
        ("lowerValue", "uml.literal_integer", lower.to_string()),
        (
            "upperValue",
            "uml.literal_unlimited_natural",
            upper.to_string(),
        ),
    ] {
        let end = staged
            .elements
            .iter()
            .find(|element| element.id == property_id)
            .ok_or("Association end Property not found")?;
        if end.properties.contains_key("multiplicity") {
            return Err(format!(
                "{property_id}: textual multiplicity is not a contract field; reconcile it explicitly before editing bounds"
            ));
        }
        let existing = end
            .properties
            .get(field)
            .filter(|value| !value.is_null())
            .map(|value| {
                crate::contract_projection::projection_reference_id(value)
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{property_id}.{field}: invalid bound reference"))
            })
            .transpose()?;
        let id = if let Some(id) = existing {
            let literal = staged
                .elements
                .iter()
                .find(|element| element.id == id)
                .ok_or_else(|| format!("{property_id}.{field}: missing bound {id}"))?;
            if literal.kind != kind {
                return Err(format!(
                    "{property_id}.{field}: edit the existing {} expression directly; a multiplicity text edit cannot replace it",
                    literal.kind
                ));
            }
            let shared = staged.elements.iter().any(|element| {
                ["lowerValue", "upperValue"].iter().any(|other_field| {
                    (element.id != property_id || *other_field != field)
                        && element
                            .properties
                            .get(*other_field)
                            .and_then(crate::contract_projection::projection_reference_id)
                            == Some(id.as_str())
                })
            });
            if shared
                || staged.relations.iter().any(|relation| {
                    relation.kind == "owns" && relation.to == id && relation.from != property_id
                })
            {
                return Err(format!(
                    "{property_id}.{field}: bound {id} is shared; edit its literal explicitly"
                ));
            }
            id
        } else {
            let id = fresh_id(&staged, project_id, &format!("{property_id}-{field}"))?;
            staged.elements.push(Element {
                id: id.clone(),
                kind: kind.into(),
                name: String::new(),
                label: None,
                description: None,
                properties: BTreeMap::new(),
                annotations: Vec::new(),
                implementations: Vec::new(),
                content: None,
            });
            id
        };
        staged
            .elements
            .iter_mut()
            .find(|element| element.id == id)
            .ok_or("Bound literal disappeared")?
            .properties
            .insert("value".into(), json!(value));
        staged
            .elements
            .iter_mut()
            .find(|element| element.id == property_id)
            .ok_or("Association end Property disappeared")?
            .properties
            .insert(field.into(), json!({"$ref": id}));
        if let Some(ownership) = staged.relations.iter_mut().find(|relation| {
            relation.kind == "owns" && relation.from == property_id && relation.to == id
        }) {
            ownership.properties.insert("role".into(), json!(field));
        } else {
            let ownership_id =
                fresh_id(&staged, project_id, &format!("{property_id}-{field}-owns"))?;
            staged.relations.push(Relation {
                id: ownership_id,
                kind: "owns".into(),
                from: property_id.into(),
                to: id,
                properties: BTreeMap::from([("role".into(), json!(field))]),
            });
        }
    }
    *section = staged;
    Ok(())
}
