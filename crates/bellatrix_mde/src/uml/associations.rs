// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2019-2026 Starion Group S.A.
// Query adaptations from uml4net 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d,
// Extend/AssociationExtensions.cs and Extend/PropertyExtensions.cs.
// Modified for Bellatrix's canonical graph: derive opposites, reject duplicate
// identities, and enforce the pinned OMG contracts rather than mutable lists.
// License and provenance: docs/bellatrix/references/uml4net/.

use super::{
    references::{ReferenceIndex, reference_id},
    root::{ElementRef, Model},
    schema,
    types::{Integer, MultiplicityError, MultiplicityRange},
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

/// ownedEnd order is the canonical composite edge order, independent of memberEnd order.
pub fn owned_ends<'a>(
    references: &ReferenceIndex<'a>,
    association_id: &str,
) -> Result<Vec<&'a Element>, String> {
    let members = member_ends(references, association_id)?;
    references
        .explicit_owned_elements(association_id)?
        .into_iter()
        .filter(|element| element.conforms_to("Property"))
        .map(|element| match element {
            ElementRef::Object(property)
                if members.iter().any(|member| member.id == property.id) =>
            {
                Ok(property)
            }
            _ => Err("Association::ownedEnd must subset memberEnd".into()),
        })
        .collect()
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

/// Literal notation is retained verbatim; this adapter does not invent ValueSpecification identities.
pub fn multiplicity(property: &Element) -> Result<MultiplicityRange, MultiplicityError> {
    if ["lowerValue", "upperValue"]
        .iter()
        .any(|key| property.properties.contains_key(*key))
    {
        return Err(MultiplicityError::UnsupportedValueSpecification);
    }
    match property.properties.get("multiplicity") {
        None => Ok(MultiplicityRange::default()),
        Some(Value::String(text)) => text.parse(),
        Some(_) => Err(MultiplicityError::Representation),
    }
}

fn validate_association(
    references: &ReferenceIndex<'_>,
    association: ElementRef<'_>,
) -> Result<(), String> {
    if association.metaclass() != Some("Association") {
        return Err("Unsupported specialized Association constraints: this mutation/validation slice implements Association itself".into());
    }
    for key in ["ownedEnd", "endType"] {
        if association.properties().contains_key(key) {
            return Err(format!(
                "Association::{key} must be derived from canonical memberEnd and owns edges"
            ));
        }
    }
    let ends = member_ends(references, association.id())?;
    let owned = owned_ends(references, association.id())?;
    if let Some(value) = association.properties().get("navigableOwnedEnd") {
        let descriptor = schema::uml_property("Association", "navigableOwnedEnd")
            .map_err(|error| error.to_string())?
            .ok_or("Association::navigableOwnedEnd contract is missing")?;
        for navigable in references.validate_many(descriptor, value)? {
            if !owned.iter().any(|end| end.id == navigable.id()) {
                return Err(
                    "Association::navigableOwnedEnd must subset Association::ownedEnd".into(),
                );
            }
        }
    }
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
            if owner.conforms_to("Association") {
                return Err(if owner.metaclass() == Some("Association") {
                    "Property::owningAssociation must subset Property::association; an end owned by another Association cannot be a member here".into()
                } else {
                    "Unsupported specialized Association owner: the Property ownership slot is ambiguous".into()
                });
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
        match multiplicity(end) {
            Ok(_) | Err(MultiplicityError::UnsupportedValueSpecification) => {}
            Err(error) => return Err(format!("Property {} multiplicity: {error}", end.id)),
        }
        if is_composite(end)? {
            let other = ends
                .get(1 - index)
                .ok_or("composite Association must be binary")?;
            let range = multiplicity(other).map_err(|error| match error {
                MultiplicityError::UnsupportedValueSpecification => format!(
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
        if relation.from != projection.from
            || relation.to != projection.to
            || PROJECTION_KEYS
                .iter()
                .any(|key| relation.properties.get(*key) != projection.properties.get(*key))
        {
            return Err(
                "Association binary projection is inconsistent with canonical memberEnd Properties"
                    .into(),
            );
        }
    }
    Ok(())
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    let mut membership = BTreeMap::<&str, &str>::new();
    for association in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Association"))
    {
        if !association.properties().contains_key("memberEnd") {
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Warning,
                code: "MDE_ASSOCIATION_ENDS_UNMATERIALIZED",
                subject_id: association.id().into(),
                message: "Association memberEnd identities are not materialized; legacy endpoint notation is not a complete Association kernel model".into(),
            });
            continue;
        }
        if let Err(message) = validate_association(references, association) {
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
        if association
            .properties()
            .get("navigableOwnedEnd")
            .is_some_and(|value| !value.is_null() && !value.as_array().is_some_and(Vec::is_empty))
        {
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Warning,
                code: "MDE_ASSOCIATION_NAVIGATION_UNSUPPORTED",
                subject_id: association.id().into(),
                message: "Association navigableOwnedEnd semantics are not implemented by this end-identity slice".into(),
            });
        }
        if let Ok(ends) = member_ends(references, association.id()) {
            for end in ends {
                if multiplicity(end) == Err(MultiplicityError::UnsupportedValueSpecification) {
                    issues.push(StructuralDiagnostic {
                        severity: StructuralSeverity::Warning,
                        code: "MDE_PROPERTY_MULTIPLICITY_UNSUPPORTED",
                        subject_id: end.id.clone(),
                        message: "Association-end multiplicity contains authored notation or ValueSpecifications not evaluated by the literal adapter".into(),
                    });
                }
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
        if let Some(value) = end.properties.get("multiplicity") {
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
    if association.metaclass() != Some("Association") {
        return Err("Unsupported specialized Association constraints: this mutation currently supports Association itself".into());
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
    if previous.is_none_or(|previous| {
        previous.properties().get("memberEnd") != association.properties().get("memberEnd")
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
                    properties_changed || owners(before) != owners(after)
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
        if issue.severity != StructuralSeverity::Error {
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
