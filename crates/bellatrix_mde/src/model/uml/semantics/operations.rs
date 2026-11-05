// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2019-2026 Starion Group S.A.
// Return-property query patterns adapted from uml4net OperationExtensions.cs,
// 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d. Bellatrix derives from canonical
// ownership and reports unevaluated bounds rather than assuming their values.
// Provenance and license: docs/bellatrix/references/uml4net/.
// Contracts: UML 2.5.1 9.9.2, 9.9.11 and 9.9.13; provenance and bounded
// comparison with RUML mutation recipes: docs/bellatrix/references/RUML.md.

use super::value_specifications::{self, EvaluationError};
use super::{
    classifiers::{self, QueryError},
    references::ReferenceIndex,
    root::ElementRef,
    schema,
    types::{Integer, MultiplicityRange, UnlimitedNatural},
};
use crate::{Element, ModelingSection, StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;

/// UML 9.9.2/9.9.11/13.2.3: canonical parameter ownership is an ordered set.
pub fn owned_parameters<'a>(
    references: &ReferenceIndex<'a>,
    owner_id: &str,
) -> Result<Vec<&'a Element>, String> {
    checked_owned_parameters(references, owner_id).map_err(|error| error.to_string())
}

pub(crate) fn checked_owned_parameters<'a>(
    references: &ReferenceIndex<'a>,
    owner_id: &str,
) -> Result<Vec<&'a Element>, QueryError> {
    let (parameters, completeness) = owned_parameter_evidence(references, owner_id)?;
    completeness?;
    Ok(parameters)
}

fn unambiguous_parameter_role(
    owner: ElementRef<'_>,
    parameter: ElementRef<'_>,
    role: &'static str,
) -> Result<bool, QueryError> {
    let mut found = false;
    for property in classifiers::properties(owner)? {
        if property.aggregation != schema::Aggregation::Composite || property.is_derived {
            continue;
        }
        let required = property
            .property_type
            .target
            .and_then(schema::classifier)
            .ok_or_else(|| {
                QueryError::unsupported(owner.id(), "composite role target type is unresolved")
            })?;
        if parameter.conforms_to(required.name) {
            if !classifiers::contributes(property, role)
                .map_err(|error| QueryError::unsupported(owner.id(), error))?
            {
                return Ok(false);
            }
            found = true;
        }
    }
    Ok(found)
}

pub(crate) fn owned_parameter_evidence<'a>(
    references: &ReferenceIndex<'a>,
    owner_id: &str,
) -> Result<(Vec<&'a Element>, Result<(), QueryError>), QueryError> {
    let owner = references
        .element(owner_id)
        .map_err(|error| QueryError::invalid(owner_id, error))?;
    let role = if owner.conforms_to("BehavioralFeature") {
        "BehavioralFeature-ownedParameter"
    } else if owner.conforms_to("Behavior") {
        "Behavior-ownedParameter"
    } else {
        return Err(QueryError::invalid(
            owner_id,
            "parameter owner must be a BehavioralFeature or Behavior",
        ));
    };
    let mut completeness = Ok(());
    let explicit = if let Some(value) = owner.properties().get("ownedParameter") {
        let descriptor = schema::uml_property(
            owner.metaclass().ok_or_else(|| {
                QueryError::unsupported(owner_id, "parameter owner has no UML metaclass")
            })?,
            "ownedParameter",
        )
        .map_err(|error| QueryError::unsupported(owner_id, error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(owner_id, "ownedParameter contract is missing"))?;
        match super::property_contracts::check_value(references, owner, descriptor, value) {
            super::property_contracts::PropertyOutcome::Satisfied => {}
            super::property_contracts::PropertyOutcome::Violation(error) => {
                return Err(QueryError::invalid(owner_id, error));
            }
            super::property_contracts::PropertyOutcome::Unverified(error) => {
                completeness = Err(QueryError::unsupported(owner_id, error));
            }
        }
        if completeness.is_ok() {
            Some(
                references
                    .validate_many(descriptor, value)
                    .map_err(|error| QueryError::invalid(owner_id, error))?,
            )
        } else {
            None
        }
    } else {
        None
    };
    let mut parameters = Vec::new();
    let owned = references
        .explicit_owned_elements(owner_id)
        .map_err(|error| QueryError::invalid(owner_id, error))?;
    for element in owned.iter().copied() {
        if element.metaclass().is_none() {
            completeness = Err(QueryError::unsupported(
                owner_id,
                "unknown owned metaclass may be a Parameter",
            ));
        } else if let ElementRef::Object(parameter) = element
            && element.conforms_to("Parameter")
        {
            match classifiers::contributes_to_owned_union(owner, element, role) {
                Ok(true) => parameters.push(parameter),
                Ok(false) => {}
                Err(error @ QueryError::Invalid { .. }) => return Err(error),
                Err(error @ QueryError::Unsupported { .. }) => {
                    completeness = Err(error);
                    // An unresolved stored projection cannot erase canonical
                    // membership when every compatible composite role is a Parameter role.
                    match unambiguous_parameter_role(owner, element, role) {
                        Ok(true) => parameters.push(parameter),
                        Ok(false) => {}
                        Err(error @ QueryError::Invalid { .. }) => return Err(error),
                        Err(error @ QueryError::Unsupported { .. }) => completeness = Err(error),
                    }
                }
            }
        }
    }
    if let Some(values) = owner
        .properties()
        .get("ownedParameter")
        .and_then(Value::as_array)
    {
        if parameters.len() > values.len() {
            return Err(QueryError::invalid(
                owner_id,
                "ownedParameter projection must match canonical owns order and membership; too few entries for the known Parameters",
            ));
        }
        let local_ids = values
            .iter()
            .filter_map(|value| super::references::reference_id(value).ok().flatten())
            .collect::<Vec<_>>();
        let canonical = owned
            .iter()
            .filter(|element| local_ids.contains(&element.id()))
            .map(|element| element.id());
        if !local_ids.iter().copied().eq(canonical) {
            return Err(QueryError::invalid(
                owner_id,
                "ownedParameter projection must match canonical owns order and membership; known local order contradicts it",
            ));
        }
        // Explicit local references occupy fixed slots even when their metaclass
        // is unknown; only external references may fill gaps between anchors.
        let required_ids = owned
            .iter()
            .filter(|element| {
                local_ids.contains(&element.id())
                    || parameters
                        .iter()
                        .any(|parameter| parameter.id == element.id())
            })
            .map(|element| element.id())
            .collect::<Vec<_>>();
        let mut consumed_parameters = 0;
        let mut consumed_entries = 0;
        for (entry_index, value) in values.iter().enumerate() {
            if let Some(parameter_index) = super::references::reference_id(value)
                .ok()
                .flatten()
                .and_then(|id| required_ids.iter().position(|required| *required == id))
            {
                if parameter_index - consumed_parameters > entry_index - consumed_entries {
                    return Err(QueryError::invalid(
                        owner_id,
                        "ownedParameter projection has too few entries between known local anchors",
                    ));
                }
                consumed_parameters = parameter_index + 1;
                consumed_entries = entry_index + 1;
            }
        }
        if required_ids.len() - consumed_parameters > values.len() - consumed_entries {
            return Err(QueryError::invalid(
                owner_id,
                "ownedParameter projection has too few entries after the last known local anchor",
            ));
        }
    }
    if let Some(explicit) = explicit {
        if !explicit
            .iter()
            .map(|parameter| parameter.id())
            .eq(parameters.iter().map(|parameter| parameter.id.as_str()))
        {
            return Err(QueryError::invalid(
                owner_id,
                "ownedParameter projection must match canonical owns order and membership",
            ));
        }
    }
    Ok((parameters, completeness))
}

pub fn direction(parameter: &Element) -> Result<String, String> {
    schema::read_effective_scalar(ElementRef::Object(parameter), "direction")?
        .as_str()
        .map(str::to_owned)
        .ok_or("invalid Parameter::direction".into())
}

pub fn return_parameter<'a>(
    references: &ReferenceIndex<'a>,
    operation_id: &str,
) -> Result<Option<&'a Element>, String> {
    checked_return_parameter(references, operation_id).map_err(|error| error.to_string())
}

pub(crate) fn checked_return_parameter<'a>(
    references: &ReferenceIndex<'a>,
    operation_id: &str,
) -> Result<Option<&'a Element>, QueryError> {
    if !references
        .element(operation_id)
        .map_err(|error| QueryError::invalid(operation_id, error))?
        .conforms_to("Operation")
    {
        return Err(QueryError::invalid(
            operation_id,
            "return-parameter query requires an Operation",
        ));
    }
    let operation = references
        .element(operation_id)
        .map_err(|error| QueryError::invalid(operation_id, error))?;
    if !at_most_one_return(references, operation)? {
        return Err(QueryError::invalid(
            operation_id,
            "Operation::at_most_one_return permits at most one return Parameter",
        ));
    }
    let mut result = None;
    for parameter in checked_owned_parameters(references, operation_id)? {
        if direction(parameter)
            .map_err(|error| QueryError::invalid(parameter.id.as_str(), error))?
            == "return"
        {
            result = Some(parameter);
        }
    }
    Ok(result)
}

fn at_most_one_return(
    references: &ReferenceIndex<'_>,
    operation: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let (parameters, completeness) = owned_parameter_evidence(references, operation.id())?;
    let mut returns = 0;
    // Inspect all local Parameter directions before deciding a known count:
    // an unresolved role or two earlier returns cannot hide malformed values.
    for child in references
        .explicit_owned_elements(operation.id())
        .map_err(|error| QueryError::invalid(operation.id(), error))?
    {
        if let ElementRef::Object(parameter) = child
            && child.conforms_to("Parameter")
        {
            let direction =
                direction(parameter).map_err(|error| QueryError::invalid(child.id(), error))?;
            if direction == "return" && parameters.iter().any(|known| known.id == parameter.id) {
                returns += 1;
            }
        }
    }
    super::feature_compatibility::require_all([Ok(returns <= 1), completeness.map(|()| true)])
}

pub fn return_type<'a>(
    references: &ReferenceIndex<'a>,
    operation_id: &str,
) -> Result<Option<ElementRef<'a>>, String> {
    let Some(parameter) = return_parameter(references, operation_id)? else {
        return Ok(None);
    };
    let descriptor = schema::uml_property("Parameter", "type")
        .map_err(|error| error.to_string())?
        .ok_or("Parameter::type contract is missing")?;
    references.validate(
        descriptor,
        parameter.properties.get("type").unwrap_or(&Value::Null),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnProperties {
    pub is_ordered: bool,
    pub is_unique: bool,
    pub multiplicity: Result<Option<MultiplicityRange>, EvaluationError>,
}

impl ReturnProperties {
    pub fn lower(&self) -> Result<Option<&Integer>, EvaluationError> {
        self.multiplicity
            .as_ref()
            .map(|range| range.as_ref().map(MultiplicityRange::lower_bound))
            .map_err(Clone::clone)
    }

    pub fn upper(&self) -> Result<Option<&UnlimitedNatural>, EvaluationError> {
        self.multiplicity
            .as_ref()
            .map(|range| range.as_ref().map(MultiplicityRange::upper_bound))
            .map_err(Clone::clone)
    }
}

/// UML Operation-isOrdered.1/isUnique.1/lower.1/upper.1: absence of a return
/// Parameter is different from a return Parameter whose multiplicity is absent.
pub fn return_properties(
    references: &ReferenceIndex<'_>,
    operation_id: &str,
) -> Result<ReturnProperties, String> {
    let Some(parameter) = return_parameter(references, operation_id)? else {
        return Ok(ReturnProperties {
            is_ordered: false,
            is_unique: true,
            multiplicity: Ok(None),
        });
    };
    Ok(ReturnProperties {
        is_ordered: schema::read_effective_boolean(ElementRef::Object(parameter), "isOrdered")?,
        is_unique: schema::read_effective_boolean(ElementRef::Object(parameter), "isUnique")?,
        multiplicity: value_specifications::multiplicity(references, ElementRef::Object(parameter))
            .map(Some),
    })
}

fn body_condition_contract(
    references: &ReferenceIndex<'_>,
    operation: ElementRef<'_>,
) -> Result<(), QueryError> {
    let Some(value) = operation.properties().get("bodyCondition") else {
        return Ok(());
    };
    let descriptor = schema::uml_property("Operation", "bodyCondition")
        .map_err(|error| QueryError::unsupported(operation.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(
                operation.id(),
                "Operation::bodyCondition contract is missing",
            )
        })?;
    match super::property_contracts::check_value(references, operation, descriptor, value) {
        super::property_contracts::PropertyOutcome::Satisfied => Ok(()),
        super::property_contracts::PropertyOutcome::Violation(message) => {
            Err(QueryError::invalid(operation.id(), message))
        }
        super::property_contracts::PropertyOutcome::Unverified(message) => {
            Err(QueryError::unsupported(operation.id(), message))
        }
    }
}

fn only_body_for_query(
    references: &ReferenceIndex<'_>,
    operation: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let contract = body_condition_contract(references, operation);
    if let Err(error @ QueryError::Invalid { .. }) = contract {
        return Err(error);
    }
    let query = schema::read_effective_boolean(operation, "isQuery")
        .map_err(|error| QueryError::invalid(operation.id(), error))?;
    // A well-formed non-null reference establishes that a bodyCondition was
    // specified even when its external identity or metaclass remains unresolved.
    Ok(query
        || operation
            .properties()
            .get("bodyCondition")
            .is_none_or(Value::is_null))
}

pub(crate) fn move_parameter(
    section: &mut ModelingSection,
    parameter_id: &str,
    up: bool,
) -> Result<(), String> {
    let model = super::root::Model::from_section(section);
    let references = ReferenceIndex::new(model);
    let parameter = references.element(parameter_id)?;
    if !parameter.conforms_to("Parameter") {
        return Err("element must be a Parameter".into());
    }
    let owner = model.owner(parameter)?.ok_or("Parameter has no owner")?;
    if !owner.conforms_to("Operation") {
        return Err("parameter reordering requires an Operation owner".into());
    }
    let parameters = owned_parameters(&references, owner.id())?;
    let position = parameters
        .iter()
        .position(|parameter| parameter.id == parameter_id)
        .ok_or("Parameter is not owned by this Operation")?;
    let adjacent = if up {
        position.checked_sub(1)
    } else {
        position.checked_add(1)
    }
    .and_then(|index| parameters.get(index));
    let Some(adjacent) = adjacent else {
        return Ok(());
    };
    let edge = |id: &str| {
        section
            .relations
            .iter()
            .position(|relation| {
                relation.kind == "owns" && relation.from == owner.id() && relation.to == id
            })
            .ok_or("Parameter ownership edge is missing")
    };
    let first = edge(parameter_id)?;
    let second = edge(&adjacent.id)?;
    section.relations.swap(first, second);
    Ok(())
}

fn connector_end_allowed(
    references: &ReferenceIndex<'_>,
    parameter: ElementRef<'_>,
) -> Result<bool, String> {
    let role = schema::uml_property("ConnectorEnd", "role")
        .map_err(|error| error.to_string())?
        .ok_or("ConnectorEnd::role contract is missing")?;
    let has_end = references.has_reference_source(role, parameter.id());
    if has_end == Ok(false) {
        return Ok(true);
    }
    let collaboration_role = schema::uml_property("Collaboration", "collaborationRole")
        .map_err(|error| error.to_string())?
        .ok_or("Collaboration::collaborationRole contract is missing")?;
    match references.has_reference_source(collaboration_role, parameter.id()) {
        Ok(true) => Ok(true),
        Ok(false) => has_end.map(|present| !present),
        Err(error) => Err(error),
    }
}

fn reentrant_behavior_allowed(
    references: &ReferenceIndex<'_>,
    parameter: ElementRef<'_>,
) -> Result<bool, String> {
    if !schema::read_effective_boolean(parameter, "isStream")? {
        return Ok(true);
    }
    let Some(owner) = references.model().owner(parameter)? else {
        return Ok(true);
    };
    let owner = references.element(owner.id())?;
    if !owner.conforms_to("Behavior") {
        return Ok(true);
    }
    if !owned_parameters(references, owner.id())?
        .iter()
        .any(|owned| owned.id == parameter.id())
    {
        return Ok(true);
    }
    schema::read_effective_boolean(owner, "isReentrant").map(|reentrant| !reentrant)
}

fn reception_reference<'a>(
    references: &ReferenceIndex<'a>,
    owner: ElementRef<'a>,
    property_name: &str,
) -> Result<Option<ElementRef<'a>>, QueryError> {
    let descriptor = schema::uml_property(
        owner
            .metaclass()
            .ok_or_else(|| QueryError::unsupported(owner.id(), "unresolved metaclass"))?,
        property_name,
    )
    .map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
    .ok_or_else(|| QueryError::unsupported(owner.id(), "missing reference contract"))?;
    let value = owner
        .properties()
        .get(property_name)
        .unwrap_or(&Value::Null);
    match super::property_contracts::check_value(references, owner, descriptor, value) {
        super::property_contracts::PropertyOutcome::Satisfied => {}
        super::property_contracts::PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(owner.id(), message));
        }
        super::property_contracts::PropertyOutcome::Unverified(message) => {
            return Err(QueryError::unsupported(owner.id(), message));
        }
    }
    references
        .validate(descriptor, value)
        .map_err(|error| QueryError::invalid(owner.id(), error))
}

fn signal_owned_attributes<'a>(
    references: &ReferenceIndex<'a>,
    signal: ElementRef<'a>,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    let explicit = if let Some(value) = signal.properties().get("ownedAttribute") {
        let descriptor = schema::uml_property("Signal", "ownedAttribute")
            .map_err(|error| QueryError::unsupported(signal.id(), error.to_string()))?
            .ok_or_else(|| {
                QueryError::unsupported(signal.id(), "missing Signal::ownedAttribute contract")
            })?;
        match super::property_contracts::check_value(references, signal, descriptor, value) {
            super::property_contracts::PropertyOutcome::Satisfied => {}
            super::property_contracts::PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(signal.id(), message));
            }
            super::property_contracts::PropertyOutcome::Unverified(message) => {
                return Err(QueryError::unsupported(signal.id(), message));
            }
        }
        Some(
            references
                .validate_many(descriptor, value)
                .map_err(|error| QueryError::invalid(signal.id(), error))?,
        )
    } else {
        None
    };
    let mut attributes = Vec::new();
    for child in references
        .explicit_owned_elements(signal.id())
        .map_err(|error| QueryError::invalid(signal.id(), error))?
    {
        if classifiers::contributes_to_owned_union(signal, child, "Signal-ownedAttribute")? {
            attributes.push(child);
        }
    }
    if let Some(explicit) = explicit {
        if explicit.len() != attributes.len()
            || attributes
                .iter()
                .any(|attribute| !explicit.iter().any(|item| item.id() == attribute.id()))
        {
            return Err(QueryError::invalid(
                signal.id(),
                "Signal::ownedAttribute must match its canonically owned attributes",
            ));
        }
        return Ok(explicit);
    }
    Ok(attributes)
}

fn reception_bounds_match(
    references: &ReferenceIndex<'_>,
    parameter: ElementRef<'_>,
    attribute: ElementRef<'_>,
    upper: bool,
) -> Result<bool, QueryError> {
    let result = if upper {
        value_specifications::upper_bound(references, parameter).and_then(|parameter_bound| {
            value_specifications::upper_bound(references, attribute)
                .map(|attribute_bound| parameter_bound == attribute_bound)
        })
    } else {
        value_specifications::lower_bound(references, parameter).and_then(|parameter_bound| {
            value_specifications::lower_bound(references, attribute)
                .map(|attribute_bound| parameter_bound == attribute_bound)
        })
    };
    result.map_err(|error| match error {
        EvaluationError::Invalid { message, .. } => QueryError::invalid(parameter.id(), message),
        EvaluationError::Unsupported { message, .. } => {
            QueryError::unsupported(parameter.id(), message)
        }
    })
}

fn reception_structure(
    references: &ReferenceIndex<'_>,
    reception: ElementRef<'_>,
    signal: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let parameters = checked_owned_parameters(references, reception.id())?;
    let attributes = signal_owned_attributes(references, signal)?;
    if parameters.len() != attributes.len() {
        return Ok(false);
    }
    super::feature_compatibility::require_all(parameters.into_iter().zip(attributes).map(
        |(parameter, attribute)| {
            let parameter = ElementRef::Object(parameter);
            super::feature_compatibility::require_all([
                schema::read_effective_scalar(parameter, "direction")
                    .map(|direction| direction == "in")
                    .map_err(|error| QueryError::invalid(parameter.id(), error)),
                Ok(parameter.named().and_then(|named| named.name())
                    == attribute.named().and_then(|named| named.name())),
                reception_reference(references, parameter, "type").and_then(|parameter_type| {
                    reception_reference(references, attribute, "type").map(|attribute_type| {
                        parameter_type.map(ElementRef::id) == attribute_type.map(ElementRef::id)
                    })
                }),
                reception_bounds_match(references, parameter, attribute, false),
                reception_bounds_match(references, parameter, attribute, true),
            ])
        },
    ))
}

fn parameter_effect<'a>(
    references: &ReferenceIndex<'a>,
    parameter: ElementRef<'a>,
) -> Result<Option<&'a str>, QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    let descriptor = schema::uml_property("Parameter", "effect")
        .map_err(|error| QueryError::unsupported(parameter.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(parameter.id(), "missing Parameter::effect contract")
        })?;
    let effect = parameter.properties().get("effect").unwrap_or(&Value::Null);
    match check_value(references, parameter, descriptor, effect) {
        PropertyOutcome::Satisfied => Ok(effect.as_str()),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(parameter.id(), message)),
        PropertyOutcome::Unverified(message) => {
            Err(QueryError::unsupported(parameter.id(), message))
        }
    }
}

// UML Parameter-object_effect: validate both operands before deciding the
// implication. An absent effect proves it even if the type is external, but
// malformed operands remain Invalid rather than disappearing through .ok().
fn parameter_object_effect(
    references: &ReferenceIndex<'_>,
    parameter: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let absent_effect = parameter_effect(references, parameter).map(|effect| effect.is_none());
    let not_data_type = reception_reference(references, parameter, "type")
        .map(|target| target.is_none_or(|target| !target.conforms_to("DataType")));
    super::feature_compatibility::accept_any([not_data_type, absent_effect])
}

// This is exact signature equality, not Operation::isConsistentWith's
// bidirectional type conformance or directional multiplicity containment.
fn equal_signature_values<T: PartialEq>(
    first: Result<T, QueryError>,
    second: Result<T, QueryError>,
) -> Result<bool, QueryError> {
    match (first, second) {
        (Ok(first), Ok(second)) => Ok(first == second),
        (first, second) => {
            super::feature_compatibility::require_all([first.map(|_| true), second.map(|_| true)])
        }
    }
}

fn operation_template_default_signature(
    references: &ReferenceIndex<'_>,
    parameter: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if parameter
        .properties()
        .get("default")
        .is_none_or(Value::is_null)
    {
        if parameter
            .properties()
            .get("ownedDefault")
            .is_some_and(|value| !value.is_null())
        {
            return Err(QueryError::unsupported(
                parameter.id(),
                "default/ownedDefault projection is not materialized",
            ));
        }
        for child in references
            .explicit_owned_elements(parameter.id())
            .map_err(|error| QueryError::invalid(parameter.id(), error))?
        {
            // ownedParameteredElement and ownedDefault share their target type;
            // the explicit composite role, not the type alone, distinguishes them.
            if child.conforms_to("ParameterableElement")
                && classifiers::contributes_to_owned_union(
                    parameter,
                    child,
                    "TemplateParameter-ownedDefault",
                )?
            {
                return Err(QueryError::unsupported(
                    parameter.id(),
                    "default/ownedDefault projection is not materialized",
                ));
            }
        }
    }
    let Some(default) = reception_reference(references, parameter, "default")? else {
        return Ok(true);
    };
    if !default.conforms_to("Operation") {
        return Ok(false);
    }
    // The legacy template adapter only supports generic/classifier parameters.
    // Do not guess an exposed operation from its legacy relation or ownership.
    if !parameter.properties().contains_key("parameteredElement") {
        return Err(QueryError::unsupported(
            parameter.id(),
            "OperationTemplateParameter parameteredElement projection is not materialized",
        ));
    }
    let exposed = reception_reference(references, parameter, "parameteredElement")?
        .ok_or_else(|| QueryError::invalid(parameter.id(), "parameteredElement is required"))?;
    let default_parameters = checked_owned_parameters(references, default.id());
    let exposed_parameters = checked_owned_parameters(references, exposed.id());
    let (default_parameters, exposed_parameters) = match (default_parameters, exposed_parameters) {
        (Ok(default), Ok(exposed)) => (default, exposed),
        (default, exposed) => {
            return super::feature_compatibility::require_all([
                default.map(|_| true),
                exposed.map(|_| true),
            ]);
        }
    };
    if default_parameters.len() != exposed_parameters.len() {
        return Ok(false);
    }
    let mut checks = Vec::new();
    for (default, exposed) in default_parameters.into_iter().zip(exposed_parameters) {
        let default = ElementRef::Object(default);
        let exposed = ElementRef::Object(exposed);
        checks.push(equal_signature_values(
            reception_reference(references, default, "type").map(|value| value.map(ElementRef::id)),
            reception_reference(references, exposed, "type").map(|value| value.map(ElementRef::id)),
        ));
        for property in ["direction", "isOrdered", "isUnique"] {
            checks.push(equal_signature_values(
                schema::read_effective_scalar(default, property)
                    .map_err(|error| QueryError::invalid(default.id(), error)),
                schema::read_effective_scalar(exposed, property)
                    .map_err(|error| QueryError::invalid(exposed.id(), error)),
            ));
        }
        let classify = |id: &str, error| match error {
            EvaluationError::Invalid { message, .. } => QueryError::invalid(id, message),
            EvaluationError::Unsupported { message, .. } => QueryError::unsupported(id, message),
        };
        checks.push(equal_signature_values(
            value_specifications::lower_bound(references, default)
                .map_err(|error| classify(default.id(), error)),
            value_specifications::lower_bound(references, exposed)
                .map_err(|error| classify(exposed.id(), error)),
        ));
        checks.push(equal_signature_values(
            value_specifications::upper_bound(references, default)
                .map_err(|error| classify(default.id(), error)),
            value_specifications::upper_bound(references, exposed)
                .map_err(|error| classify(exposed.id(), error)),
        ));
    }
    super::feature_compatibility::require_all(checks)
}

fn parameter_set_same_entity(
    references: &ReferenceIndex<'_>,
    parameter_set: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    let descriptor = schema::uml_property("ParameterSet", "parameter")
        .map_err(|error| QueryError::unsupported(parameter_set.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(
                parameter_set.id(),
                "missing ParameterSet::parameter contract",
            )
        })?;
    let value = parameter_set.properties().get("parameter").ok_or_else(|| {
        QueryError::unsupported(
            parameter_set.id(),
            "ParameterSet::parameter identities are not materialized",
        )
    })?;
    let mut checks = Vec::new();
    match check_value(references, parameter_set, descriptor, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(parameter_set.id(), message));
        }
        PropertyOutcome::Unverified(message) => {
            checks.push(Err(QueryError::unsupported(parameter_set.id(), message)))
        }
    }
    let values = value
        .as_array()
        .ok_or_else(|| QueryError::invalid(parameter_set.id(), "parameter must be a collection"))?;
    let owner_id = |element| -> Result<Option<&str>, QueryError> {
        references
            .model()
            .owner(element)
            .map_err(|error| QueryError::invalid(parameter_set.id(), error))?
            .map(|owner| {
                references
                    .element(owner.id())
                    .map(ElementRef::id)
                    .map_err(|error| QueryError::invalid(parameter_set.id(), error))
            })
            .transpose()
    };
    let owner = owner_id(parameter_set)?;
    let single = schema::PropertyDescriptor {
        lower: "1",
        upper: "1",
        ..*descriptor
    };
    let mut known_direction = None;
    for value in values {
        match check_value(references, parameter_set, &single, value) {
            PropertyOutcome::Satisfied => {}
            PropertyOutcome::Violation(message) => {
                checks.push(Err(QueryError::invalid(parameter_set.id(), message)));
                continue;
            }
            PropertyOutcome::Unverified(message) => {
                checks.push(Err(QueryError::unsupported(parameter_set.id(), message)));
            }
        }
        // An unresolved metaclass does not erase a local identity or its
        // canonical owner. Only a genuinely external member lacks that evidence.
        if matches!(
            super::references::external_reference_href(value),
            Ok(Some(_))
        ) {
            continue;
        }
        let parameter = super::references::reference_id(value)
            .map_err(|error| QueryError::invalid(parameter_set.id(), error))
            .and_then(|id| {
                id.ok_or_else(|| {
                    QueryError::invalid(parameter_set.id(), "parameter cannot be absent")
                })
            })
            .and_then(|id| {
                references
                    .element(id)
                    .map_err(|error| QueryError::invalid(parameter_set.id(), error))
            });
        let parameter = match parameter {
            Ok(parameter) => parameter,
            Err(error) => {
                checks.push(Err(error));
                continue;
            }
        };
        checks.push(owner_id(parameter).map(|parameter_owner| parameter_owner == owner));
        if !parameter.conforms_to("Parameter") {
            continue;
        }
        let parameter_direction = schema::read_effective_scalar(parameter, "direction")
            .map_err(|error| QueryError::invalid(parameter.id(), error));
        match parameter_direction {
            Ok(direction) => {
                if let Some(known) = &known_direction {
                    checks.push(Ok(known == &direction));
                } else {
                    known_direction = Some(direction);
                }
            }
            Err(error) => checks.push(Err(error)),
        }
    }
    // Each owner equality and each known direction is examined even if another
    // member is external. A malformed member/owner/direction dominates false,
    // and known disagreement dominates uncertainty without claiming a full set.
    super::feature_compatibility::require_all(checks)
}
