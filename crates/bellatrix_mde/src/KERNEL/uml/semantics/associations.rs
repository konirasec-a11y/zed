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
    types::{Integer, MultiplicityRange},
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
        if is_end
            && (!is_owned_end
                || classifiers::is_owned_attribute(association, child)
                    .map_err(|error| error.to_string())?)
        {
            return Err("Association-owned memberEnd has a conflicting composite role; AssociationClass::disjoint_attributes_ends forbids an end also being an ownedAttribute".into());
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
    let owned = owned_ends(references, association_id)?;
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
    let owned = owned_ends(references, association_id)?;
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

pub(crate) fn validate_authoring_changes(
    before: &ModelingSection,
    after: &ModelingSection,
) -> Result<(), String> {
    let previous = ReferenceIndex::new(Model::from_section(before));
    let current = ReferenceIndex::new(Model::from_section(after));
    for association_id in after.authoring_knowledge.associations.keys() {
        if let Err(error) = validate_navigation_declarations(&current, association_id)
            && (before.authoring_knowledge.associations.get(association_id)
                != after.authoring_knowledge.associations.get(association_id)
                || validate_navigation_declarations(&previous, association_id)
                    .as_ref()
                    .err()
                    != Some(&error))
        {
            return Err(format!("MDE_NAVIGATION_KNOWLEDGE_INVALID: {error}"));
        }
    }
    Ok(())
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

fn validate_association(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Result<(), String> {
    if association.properties().contains_key("endType") {
        return Err(
            "Association::endType must be derived from canonical memberEnd and owns edges".into(),
        );
    }
    let ends = member_ends(references, association.id())?;
    let owned = owned_ends(references, association.id())?;
    validate_navigation_declarations(references, association.id())?;
    if ends.len() > 2 && owned.len() != ends.len() {
        return Err(
            "Association::association_ends requires all n-ary member ends to be Association-owned"
                .into(),
        );
    }
    let aggregation_count = ends
        .iter()
        .map(|end| aggregation(end))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|kind| *kind != "none")
        .count();
    if aggregation_count > 0 && (ends.len() != 2 || aggregation_count != 1) {
        return Err("Association::binary_associations permits aggregation only on one end of a binary Association".into());
    }
    for (index, end) in ends.iter().enumerate() {
        property_type(references, end)?
            .ok_or("Association::ends_must_be_typed requires every member end to have a Type")?;
        let owner = references
            .model()
            .owner(ElementRef::Object(end))?
            .ok_or("Association member end requires an owner")?;
        if owner.id() != association.id() {
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
            if ends.len() == 2 {
                let other = ends
                    .get(1 - index)
                    .ok_or("binary opposite end is missing")?;
                if property_type(references, other)?.is_none_or(|target| target.id() != owner.id())
                {
                    return Err("Property::type_of_opposite_end requires a classifier-owned end's owner to be the opposite end Type".into());
                }
            }
        }
        match multiplicity(references, end) {
            Ok(_) => {}
            Err(error) if error.is_unsupported() => {}
            Err(error) => return Err(format!("Property {} multiplicity: {error}", end.id)),
        }
        if is_composite(end)? {
            let other = ends
                .get(1 - index)
                .ok_or("composite Association must be binary")?;
            let range = multiplicity(references, other).map_err(|error| match error {
                error if error.is_unsupported() => format!(
                    "Unsupported Property::multiplicity_of_composite evaluation of opposite bound: {error}"
                ),
                _ => format!("Property::multiplicity_of_composite invalid opposite bound: {error}"),
            })?;
            if range
                .upper_bound()
                .finite_value()
                .is_none_or(|upper| upper > &Integer::from(1))
            {
                return Err("Property::multiplicity_of_composite requires the opposite end upper bound to be at most 1".into());
            }
        }
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
    validate_specialization(references, association, &ends)?;
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
    Ok(())
}

fn validate_specialization(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
    ends: &[&Element],
) -> Result<(), String> {
    let ancestry = classifier_hierarchy::resolve_ancestry(references, association.id())
        .map_err(|error| error.to_string())?;
    let mut unsupported = None;
    for general in ancestry
        .parents
        .iter()
        .filter(|parent| parent.conforms_to("Association"))
    {
        if !general.properties().contains_key("memberEnd") {
            unsupported = Some(format!(
                "Unsupported Association specialization: general {} has no materialized memberEnd identities",
                general.id(),
            ));
            continue;
        }
        let general_ends = member_ends(references, general.id())?;
        if ends.len() != general_ends.len() {
            return Err(format!(
                "Association::specialized_end_number requires the same number of member ends as general {} ({} instead of {})",
                general.id(),
                ends.len(),
                general_ends.len(),
            ));
        }
        for (position, (specific, general_end)) in ends.iter().zip(general_ends).enumerate() {
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
                    unsupported = Some(error.to_string());
                }
                Err(error) => return Err(error.to_string()),
            }
        }
    }
    unsupported.map_or(Ok(()), Err)
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

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for (association_id, knowledge) in &references.model().authoring_knowledge().associations {
        match validate_navigation_declarations(references, association_id) {
            Err(message) => issues.push(issue(
                "MDE_NAVIGATION_KNOWLEDGE_INVALID",
                association_id,
                message,
            )),
            Ok(()) if !knowledge.navigation_unverified.is_empty() => {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Warning,
                    code: "MDE_ASSOCIATION_NAVIGATION_UNVERIFIED",
                    subject_id: association_id.clone(),
                    message: "Association navigation is not fully specified in authoring knowledge; this is not a proved UML violation".into(),
                });
                issues.push(issue("UML_MODEL_SEMANTICS_UNVERIFIED", association_id,
                    "Association navigation is authored as unverified; known UML rules still run, but this model cannot be reported as semantically verified"));
            }
            Ok(()) => {}
        }
    }
    let mut membership = BTreeMap::<&str, &str>::new();
    for association in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Association"))
    {
        if association.conforms_to("AssociationClass") {
            let outcome = match association_class_cannot_be_defined(references, association) {
                Ok(true) => None,
                Ok(false) => Some((
                    StructuralSeverity::Error,
                    "UML_ASSOCIATION_CLASS_CANNOT_BE_DEFINED",
                    "endType or an end Type's allParents contains this AssociationClass".to_owned(),
                )),
                Err(classifiers::QueryError::Invalid { message, .. }) => Some((
                    StructuralSeverity::Error,
                    "UML_ASSOCIATION_CLASS_CANNOT_BE_DEFINED",
                    message,
                )),
                Err(classifiers::QueryError::Unsupported { message, .. }) => Some((
                    StructuralSeverity::Warning,
                    "MDE_ASSOCIATION_CLASS_UNVERIFIED",
                    message,
                )),
            };
            if let Some((severity, code, message)) = outcome {
                issues.push(StructuralDiagnostic {
                    severity,
                    code,
                    subject_id: association.id().into(),
                    message: format!("AssociationClass-cannot_be_defined: {message}"),
                });
            }
        }
        if association.conforms_to("CommunicationPath") {
            let outcome = match communication_path_association_ends(references, association) {
                Ok(true) => None,
                Ok(false) => Some((
                    StructuralSeverity::Error,
                    "UML_COMMUNICATION_PATH_ASSOCIATION_ENDS",
                    "end Types must be DeploymentTargets".to_owned(),
                )),
                Err(classifiers::QueryError::Invalid { message, .. }) => Some((
                    StructuralSeverity::Error,
                    "MDE_COMMUNICATION_PATH_ASSOCIATION_ENDS_INVALID",
                    message,
                )),
                Err(classifiers::QueryError::Unsupported { message, .. }) => Some((
                    StructuralSeverity::Warning,
                    "MDE_COMMUNICATION_PATH_ASSOCIATION_ENDS_UNVERIFIED",
                    message,
                )),
            };
            if let Some((severity, code, message)) = outcome {
                issues.push(StructuralDiagnostic {
                    severity,
                    code,
                    subject_id: association.id().into(),
                    message: format!("CommunicationPath-association_ends: {message}"),
                });
            }
        }
        if !association.properties().contains_key("memberEnd") {
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Warning,
                code: "MDE_ASSOCIATION_ENDS_UNMATERIALIZED",
                subject_id: association.id().into(),
                message: "Association memberEnd identities are not materialized; legacy endpoint notation is not a complete Association kernel model".into(),
            });
            continue;
        }
        // This specific rule already has its own diagnostic above. The command
        // gate uses the same rule, but global output reports it only once.
        if let Err(message) = validate_association(references, association)
            && !message.contains("AssociationClass-cannot_be_defined:")
            && !message.contains("CommunicationPath-association_ends:")
        {
            if message.starts_with("Unsupported ") {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Warning,
                    code: "MDE_ASSOCIATION_UNSUPPORTED",
                    subject_id: association.id().into(),
                    message,
                });
            } else {
                issues.push(issue("UML_ASSOCIATION_ENDS", association.id(), message));
            }
        }
        if let Ok(ends) = member_ends(references, association.id()) {
            for end in ends {
                if membership.insert(&end.id, association.id()).is_some() {
                    issues.push(issue(
                        "UML_PROPERTY_ASSOCIATION",
                        &end.id,
                        "Property::association has upper bound 1",
                    ));
                }
            }
        }
    }
    for property in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Property"))
    {
        if ["association", "owningAssociation", "opposite"]
            .iter()
            .any(|key| property.properties().contains_key(*key))
        {
            issues.push(issue("UML_PROPERTY_ASSOCIATION_STORAGE", property.id(), "Property association, owningAssociation and opposite must be derived from canonical memberEnd and owns edges"));
        }
    }
    issues
}

fn issue(code: &'static str, id: &str, message: impl Into<String>) -> StructuralDiagnostic {
    StructuralDiagnostic {
        severity: StructuralSeverity::Error,
        code,
        subject_id: id.into(),
        message: message.into(),
    }
}

const PROJECTION_KEYS: [&str; 6] = [
    "sourceRole",
    "targetRole",
    "sourceMultiplicity",
    "targetMultiplicity",
    "end1Aggregation",
    "end2Aggregation",
];

fn binary_projection(
    references: &ReferenceIndex<'_>,
    relation: &Relation,
) -> Result<Relation, String> {
    let ends = member_ends(references, &relation.id)?;
    let [source, target] = ends.as_slice() else {
        return Err("Unsupported binary Relation notation: exactly two member ends are required by this adapter".into());
    };
    let mut result = relation.clone();
    result.from = property_type(references, source)
        .map_err(|error| format!("UML_ASSOCIATION_END_TYPE: {error}"))?
        .ok_or("UML_ASSOCIATION_END_TYPE: Association source end requires a Type")?
        .id()
        .into();
    result.to = property_type(references, target)
        .map_err(|error| format!("UML_ASSOCIATION_END_TYPE: {error}"))?
        .ok_or("UML_ASSOCIATION_END_TYPE: Association target end requires a Type")?
        .id()
        .into();
    for key in PROJECTION_KEYS {
        result.properties.remove(key);
    }
    for (end, role, range, diamond) in [
        (
            source,
            "sourceRole",
            "sourceMultiplicity",
            "end2Aggregation",
        ),
        (
            target,
            "targetRole",
            "targetMultiplicity",
            "end1Aggregation",
        ),
    ] {
        if !end.name.is_empty() {
            result.properties.insert(role.into(), json!(end.name));
        }
        if value_specifications::has_bound_references(ElementRef::Object(end)) {
            match multiplicity(references, end) {
                Ok(bounds) => {
                    result.properties.insert(
                        range.into(),
                        json!(format!(
                            "{}..{}",
                            bounds.lower_bound(),
                            bounds.upper_bound()
                        )),
                    );
                }
                Err(error) if error.is_unsupported() => {}
                Err(error) => return Err(error.to_string()),
            }
        } else if let Some(value) = end.properties.get("multiplicity") {
            result.properties.insert(range.into(), value.clone());
        }
        let kind = aggregation(end)?;
        // UML places the diamond at the composing end, opposite the composite Property's Type.
        if kind != "none" {
            result.properties.insert(diamond.into(), json!(kind));
        }
    }
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

/// Explicit, lossless migration of the existing binary adapter. Reads never mutate authored models.
pub(crate) fn migrate(
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
        return validate_association(
            &ReferenceIndex::new(Model::from_section(section)),
            ElementRef::Relationship(&relation),
        );
    }
    for key in ["ownedEnd", "endType", "navigableOwnedEnd"] {
        if relation.properties.contains_key(key) {
            return Err(format!(
                "Legacy Association already contains {key}; reconcile its authored references before migration"
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
        if let Some(value) = relation.properties.get(range) {
            properties.insert("multiplicity".into(), value.clone());
        }
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
        let ownership_id = fresh_id(section, project_id, "mde-association-owns")?;
        section.relations.push(Relation {
            id: ownership_id,
            kind: "owns".into(),
            from: association_id.into(),
            to: id.clone(),
            properties: BTreeMap::new(),
        });
        ends.push(json!({"$ref": id}));
    }
    section
        .relations
        .iter_mut()
        .find(|relation| relation.id == association_id)
        .ok_or("Association disappeared")?
        .properties
        .insert("memberEnd".into(), Value::Array(ends));
    synchronize_projection(section, association_id)
}

pub(crate) fn set_member_ends(
    section: &mut ModelingSection,
    association_id: &str,
    ends: &[String],
) -> Result<(), String> {
    let references = ReferenceIndex::new(Model::from_section(section));
    let association = references.element(association_id)?;
    if !association.conforms_to("Association") {
        return Err("memberEnd owner must be an Association".into());
    }
    let value = Value::Array(ends.iter().map(|id| json!({"$ref": id})).collect());
    let descriptor = schema::uml_property("Association", "memberEnd")
        .map_err(|error| error.to_string())?
        .ok_or("Association::memberEnd contract is missing")?;
    references.validate_many(descriptor, &value)?;
    if matches!(association, ElementRef::Relationship(_)) && ends.len() > 2 {
        return Err("Unsupported binary Relation notation: use an n-ary Association element for more than two member ends".into());
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
    synchronize_projection(section, association_id)?;
    let references = ReferenceIndex::new(Model::from_section(section));
    validate_association(&references, references.element(association_id)?)?;
    for end in member_ends(&references, association_id)? {
        association_of(&references, &end.id)?;
    }
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
    let id = member_ends(&references, association_id)?
        .get(index)
        .ok_or("Association member end is missing")?
        .id
        .clone();
    let end = section
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("Association member end disappeared")?;
    if property == "multiplicity"
        && value_specifications::has_bound_references(ElementRef::Object(end))
    {
        return Err("Multiplicity is derived from lowerValue/upperValue identities; edit the bound ValueSpecification instead".into());
    }
    if property == "name" {
        end.name = value.as_str().ok_or("expected string")?.into();
    } else {
        end.properties.insert(property.into(), value.clone());
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
        let relation = after
            .relations
            .iter_mut()
            .find(|relation| relation.id == id)
            .ok_or("new Association disappeared")?;
        if !relation.properties.contains_key("memberEnd") {
            // Newly authored ends need distinguishable names in their Namespace.
            // Migration of existing authored records deliberately does not invent names.
            for role in ["sourceRole", "targetRole"] {
                if relation.properties.contains_key(role) {
                    continue;
                }
                let mut sequence = 1_u64;
                loop {
                    let name = format!("end{sequence}");
                    if !["sourceRole", "targetRole"].iter().any(|role| {
                        relation.properties.get(*role).and_then(Value::as_str)
                            == Some(name.as_str())
                    }) {
                        relation.properties.insert(role.into(), json!(name));
                        break;
                    }
                    sequence = sequence
                        .checked_add(1)
                        .ok_or("Association role name overflow")?;
                }
            }
        }
        migrate(after, &before.id, &id)?;
    }
    Ok(())
}

pub(crate) fn validate_changes(
    before: &ModelingSection,
    after: &ModelingSection,
) -> Result<(), String> {
    let references = ReferenceIndex::new(Model::from_section(after));
    let mut changed_ends = HashSet::new();
    for association in references.model().elements().filter(|element| {
        element.conforms_to("Association")
            && element.properties().contains_key("memberEnd")
            && canonical_inputs_changed(before, after, *element)
    }) {
        validate_association(&references, association).map_err(|error| {
            if error.starts_with("Unsupported ") {
                format!("MDE_ASSOCIATION_UNSUPPORTED: {error}")
            } else {
                format!("UML_ASSOCIATION_ENDS: {error}")
            }
        })?;
        changed_ends.extend(
            member_ends(&references, association.id())?
                .iter()
                .map(|end| end.id.as_str()),
        );
    }
    let existing = validate(&ReferenceIndex::new(Model::from_section(before)));
    for issue in validate(&references) {
        if issue.code == "UML_PROPERTY_ASSOCIATION"
            && changed_ends.contains(issue.subject_id.as_str())
        {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        if issue.severity != StructuralSeverity::Error
            || issue.code == "UML_MODEL_SEMANTICS_UNVERIFIED"
        {
            continue;
        }
        if !existing.iter().any(|previous| {
            previous.code == issue.code
                && previous.subject_id == issue.subject_id
                && previous.message == issue.message
        }) {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
    }
    Ok(())
}
