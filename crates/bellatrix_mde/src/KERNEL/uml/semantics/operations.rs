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
    let owner = references.element(owner_id).map_err(|error| QueryError::invalid(owner_id, error))?;
    let role = if owner.conforms_to("BehavioralFeature") {
        "BehavioralFeature-ownedParameter"
    } else if owner.conforms_to("Behavior") {
        "Behavior-ownedParameter"
    } else {
        return Err(QueryError::invalid(owner_id, "parameter owner must be a BehavioralFeature or Behavior"));
    };
    let explicit = if let Some(value) = owner.properties().get("ownedParameter") {
        let descriptor = schema::uml_property(
            owner.metaclass().ok_or_else(|| QueryError::unsupported(owner_id, "parameter owner has no UML metaclass"))?,
            "ownedParameter",
        )
        .map_err(|error| QueryError::unsupported(owner_id, error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(owner_id, "ownedParameter contract is missing"))?;
        match super::property_contracts::check_value(references, owner, descriptor, value) {
            super::property_contracts::PropertyOutcome::Satisfied => {}
            super::property_contracts::PropertyOutcome::Violation(error) =>
                return Err(QueryError::invalid(owner_id, error)),
            super::property_contracts::PropertyOutcome::Unverified(error) =>
                return Err(QueryError::unsupported(owner_id, error)),
        }
        Some(references.validate_many(descriptor, value)
            .map_err(|error| QueryError::invalid(owner_id, error))?)
    } else {
        None
    };
    let mut parameters = Vec::new();
    for element in references.explicit_owned_elements(owner_id)
        .map_err(|error| QueryError::invalid(owner_id, error))? {
        if let ElementRef::Object(parameter) = element
            && element.conforms_to("Parameter")
            && classifiers::contributes_to_owned_union(owner, element, role)?
        {
            parameters.push(parameter);
        }
    }
    if let Some(explicit) = explicit {
        if !explicit.iter().map(|parameter| parameter.id())
            .eq(parameters.iter().map(|parameter| parameter.id.as_str()))
        {
            return Err(QueryError::invalid(owner_id,
                "ownedParameter projection must match canonical owns order and membership"));
        }
    }
    Ok(parameters)
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
    if !references.element(operation_id)
        .map_err(|error| QueryError::invalid(operation_id, error))?.conforms_to("Operation") {
        return Err(QueryError::invalid(operation_id, "return-parameter query requires an Operation"));
    }
    let mut result = None;
    for parameter in checked_owned_parameters(references, operation_id)? {
        if direction(parameter).map_err(|error| QueryError::invalid(parameter.id.as_str(), error))? == "return" && result.replace(parameter).is_some() {
            return Err(QueryError::invalid(
                operation_id,
                "Operation::at_most_one_return permits at most one return Parameter",
            ));
        }
    }
    Ok(result)
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

fn validate_body_condition(
    references: &ReferenceIndex<'_>,
    operation: ElementRef<'_>,
) -> Result<(), String> {
    let Some(value) = operation
        .properties()
        .get("bodyCondition")
        .filter(|value| !value.is_null())
    else {
        return Ok(());
    };
    if !schema::read_effective_boolean(operation, "isQuery")? {
        return Err(
            "Operation::only_body_for_query requires isQuery when bodyCondition is specified"
                .into(),
        );
    }
    let descriptor = schema::uml_property("Operation", "bodyCondition")
        .map_err(|error| error.to_string())?
        .ok_or("Operation::bodyCondition contract is missing")?;
    let constraint = references
        .validate(descriptor, value)?
        .ok_or("bodyCondition is missing")?;
    if references
        .model()
        .owner(constraint)?
        .is_none_or(|owner| owner.id() != operation.id())
    {
        return Err("Operation::bodyCondition must be a Constraint owned by this Operation".into());
    }
    Ok(())
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

fn validate_parameter_relations(
    references: &ReferenceIndex<'_>,
    parameter: ElementRef<'_>,
    issues: &mut Vec<StructuralDiagnostic>,
) {
    if ["end", "collaboration"]
        .iter()
        .any(|key| parameter.properties().contains_key(*key))
    {
        issues.push(StructuralDiagnostic {
            severity: StructuralSeverity::Warning,
            code: "MDE_PARAMETER_CONSTRAINT_UNVERIFIED",
            subject_id: parameter.id().into(),
            message: "Parameter end and collaboration must be derived from ConnectorEnd::role and Collaboration::collaborationRole, not stored opposite values".into(),
        });
    }
    for (rule, requirement, result) in [
        (
            "Parameter-connector_end",
            "a Parameter used as a ConnectorEnd role must be a Collaboration role",
            connector_end_allowed(references, parameter),
        ),
        (
            "Parameter-reentrant_behaviors",
            "a streaming Parameter must not belong to a reentrant Behavior",
            reentrant_behavior_allowed(references, parameter),
        ),
    ] {
        let (severity, code, message) = match result {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_PARAMETER_CONSTRAINT",
                format!("{rule}: {requirement}"),
            ),
            Err(error) => (
                StructuralSeverity::Warning,
                "MDE_PARAMETER_CONSTRAINT_UNVERIFIED",
                format!("{rule}: {error}"),
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: parameter.id().into(),
            message,
        });
    }
}

fn reception_reference<'a>(
    references: &ReferenceIndex<'a>,
    owner: ElementRef<'a>,
    property_name: &str,
) -> Result<Option<ElementRef<'a>>, QueryError> {
    let descriptor = schema::uml_property(
        owner.metaclass().ok_or_else(|| QueryError::unsupported(owner.id(), "unresolved metaclass"))?,
        property_name,
    ).map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(owner.id(), "missing reference contract"))?;
    let value = owner.properties().get(property_name).unwrap_or(&Value::Null);
    match super::property_contracts::check_value(references, owner, descriptor, value) {
        super::property_contracts::PropertyOutcome::Satisfied => {}
        super::property_contracts::PropertyOutcome::Violation(message) =>
            return Err(QueryError::invalid(owner.id(), message)),
        super::property_contracts::PropertyOutcome::Unverified(message) =>
            return Err(QueryError::unsupported(owner.id(), message)),
    }
    references.validate(descriptor, value)
        .map_err(|error| QueryError::invalid(owner.id(), error))
}

fn signal_owned_attributes<'a>(
    references: &ReferenceIndex<'a>,
    signal: ElementRef<'a>,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    let explicit = if let Some(value) = signal.properties().get("ownedAttribute") {
        let descriptor = schema::uml_property("Signal", "ownedAttribute")
            .map_err(|error| QueryError::unsupported(signal.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(signal.id(), "missing Signal::ownedAttribute contract"))?;
        match super::property_contracts::check_value(references, signal, descriptor, value) {
            super::property_contracts::PropertyOutcome::Satisfied => {}
            super::property_contracts::PropertyOutcome::Violation(message) =>
                return Err(QueryError::invalid(signal.id(), message)),
            super::property_contracts::PropertyOutcome::Unverified(message) =>
                return Err(QueryError::unsupported(signal.id(), message)),
        }
        Some(references.validate_many(descriptor, value)
            .map_err(|error| QueryError::invalid(signal.id(), error))?)
    } else { None };
    let mut attributes = Vec::new();
    for child in references.explicit_owned_elements(signal.id())
        .map_err(|error| QueryError::invalid(signal.id(), error))?
    {
        if classifiers::contributes_to_owned_union(signal, child, "Signal-ownedAttribute")? {
            attributes.push(child);
        }
    }
    if let Some(explicit) = explicit {
        if explicit.len() != attributes.len()
            || attributes.iter().any(|attribute| !explicit.iter().any(|item| item.id() == attribute.id()))
        {
            return Err(QueryError::invalid(signal.id(),
                "Signal::ownedAttribute must match its canonically owned attributes"));
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
        value_specifications::upper_bound(references, parameter).and_then(|parameter_bound|
            value_specifications::upper_bound(references, attribute)
                .map(|attribute_bound| parameter_bound == attribute_bound))
    } else {
        value_specifications::lower_bound(references, parameter).and_then(|parameter_bound|
            value_specifications::lower_bound(references, attribute)
                .map(|attribute_bound| parameter_bound == attribute_bound))
    };
    result.map_err(|error| match error {
        EvaluationError::Invalid { message, .. } => QueryError::invalid(parameter.id(), message),
        EvaluationError::Unsupported { message, .. } => QueryError::unsupported(parameter.id(), message),
    })
}

fn reception_structure(
    references: &ReferenceIndex<'_>,
    reception: ElementRef<'_>,
    signal: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let parameters = checked_owned_parameters(references, reception.id())?;
    let attributes = signal_owned_attributes(references, signal)?;
    if parameters.len() != attributes.len() { return Ok(false); }
    super::feature_compatibility::require_all(parameters.into_iter().zip(attributes).map(
        |(parameter, attribute)| {
            let parameter = ElementRef::Object(parameter);
            super::feature_compatibility::require_all([
                schema::read_effective_scalar(parameter, "direction")
                    .map(|direction| direction == "in")
                    .map_err(|error| QueryError::invalid(parameter.id(), error)),
                Ok(parameter.named().and_then(|named| named.name())
                    == attribute.named().and_then(|named| named.name())),
                reception_reference(references, parameter, "type").and_then(|parameter_type|
                    reception_reference(references, attribute, "type")
                        .map(|attribute_type| parameter_type.map(ElementRef::id) == attribute_type.map(ElementRef::id))),
                reception_bounds_match(references, parameter, attribute, false),
                reception_bounds_match(references, parameter, attribute, true),
            ])
        },
    ))
}

fn validate_reception(
    references: &ReferenceIndex<'_>,
    reception: ElementRef<'_>,
    issues: &mut Vec<StructuralDiagnostic>,
) {
    // The editor creates receptions before a Signal is selected. Preserve the
    // existing omitted-reference draft policy without treating it as conformance.
    if !reception.properties().contains_key("signal") {
        issues.push(StructuralDiagnostic {
            severity: StructuralSeverity::Warning,
            code: "MDE_RECEPTION_SIGNAL_INCOMPLETE",
            subject_id: reception.id().into(),
            message: "Reception is incomplete: signal has not been selected; same_name_as_signal and same_structure_as_signal are not evaluated, and UML conformance is not established".into(),
        });
        return;
    }
    let signal = reception_reference(references, reception, "signal")
        .and_then(|signal| signal.ok_or_else(|| QueryError::invalid(reception.id(), "Reception requires a Signal")));
    for (rule, result) in [
        ("Reception-same_name_as_signal", signal.clone().map(|signal|
            reception.named().and_then(|named| named.name()) == signal.named().and_then(|named| named.name()))),
        ("Reception-same_structure_as_signal", signal.and_then(|signal|
            reception_structure(references, reception, signal))),
    ] {
        let (severity, code, message) = match result {
            Ok(true) => continue,
            Ok(false) => (StructuralSeverity::Error, "UML_RECEPTION_CONSTRAINT", rule.to_owned()),
            Err(QueryError::Invalid { message, .. }) =>
                (StructuralSeverity::Error, "UML_RECEPTION_CONSTRAINT", format!("{rule}: {message}")),
            Err(QueryError::Unsupported { message, .. }) =>
                (StructuralSeverity::Warning, "MDE_RECEPTION_CONSTRAINT_UNVERIFIED", format!("{rule}: {message}")),
        };
        issues.push(StructuralDiagnostic {
            severity, code, subject_id: reception.id().into(), message,
        });
    }
}

fn parameter_set_same_entity(
    references: &ReferenceIndex<'_>,
    parameter_set: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::property_contracts::{check_value, PropertyOutcome};
    let descriptor = schema::uml_property("ParameterSet", "parameter")
        .map_err(|error| QueryError::unsupported(parameter_set.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(parameter_set.id(), "missing ParameterSet::parameter contract"))?;
    let value = parameter_set.properties().get("parameter")
        .ok_or_else(|| QueryError::unsupported(parameter_set.id(), "ParameterSet::parameter identities are not materialized"))?;
    let mut checks = Vec::new();
    match check_value(references, parameter_set, descriptor, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) => return Err(QueryError::invalid(parameter_set.id(), message)),
        PropertyOutcome::Unverified(message) => checks.push(Err(QueryError::unsupported(parameter_set.id(), message))),
    }
    let values = value.as_array().ok_or_else(|| QueryError::invalid(parameter_set.id(), "parameter must be a collection"))?;
    let owner_id = |element| -> Result<Option<&str>, QueryError> {
        references.model().owner(element)
            .map_err(|error| QueryError::invalid(parameter_set.id(), error))?
            .map(|owner| references.element(owner.id()).map(ElementRef::id)
                .map_err(|error| QueryError::invalid(parameter_set.id(), error)))
            .transpose()
    };
    let owner = owner_id(parameter_set)?;
    let single = schema::PropertyDescriptor { lower: "1", upper: "1", ..*descriptor };
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
        if matches!(super::references::external_reference_href(value), Ok(Some(_))) { continue; }
        let parameter = super::references::reference_id(value)
            .map_err(|error| QueryError::invalid(parameter_set.id(), error))
            .and_then(|id| id.ok_or_else(|| QueryError::invalid(parameter_set.id(), "parameter cannot be absent")))
            .and_then(|id| references.element(id).map_err(|error| QueryError::invalid(parameter_set.id(), error)));
        let parameter = match parameter {
            Ok(parameter) => parameter,
            Err(error) => { checks.push(Err(error)); continue; }
        };
        checks.push(owner_id(parameter).map(|parameter_owner| parameter_owner == owner));
        if !parameter.conforms_to("Parameter") { continue; }
        let parameter_direction = schema::read_effective_scalar(parameter, "direction")
            .map_err(|error| QueryError::invalid(parameter.id(), error));
        match parameter_direction {
            Ok(direction) => {
                if let Some(known) = &known_direction { checks.push(Ok(known == &direction)); }
                else { known_direction = Some(direction); }
            }
            Err(error) => checks.push(Err(error)),
        }
    }
    // Each owner equality and each known direction is examined even if another
    // member is external. A malformed member/owner/direction dominates false,
    // and known disagreement dominates uncertainty without claiming a full set.
    super::feature_compatibility::require_all(checks)
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for element in references.model().elements() {
        if element.conforms_to("ParameterSet") {
            let outcome = match parameter_set_same_entity(references, element) {
                Ok(true) => None,
                Ok(false) => Some((StructuralSeverity::Error, "UML_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY",
                    "Parameters must share their ParameterSet's canonical owner and direction".to_owned())),
                Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error,
                    "MDE_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY_INVALID", message)),
                Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning,
                    "MDE_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY_UNVERIFIED", message)),
            };
            if let Some((severity, code, message)) = outcome {
                issues.push(StructuralDiagnostic { severity, code, subject_id: element.id().into(),
                    message: format!("ParameterSet-same_parameterized_entity: {message}") });
            }
        }
        if element.conforms_to("Reception") {
            validate_reception(references, element, &mut issues);
        }
        if (element.conforms_to("BehavioralFeature") || element.conforms_to("Behavior"))
            && element.properties().contains_key("ownedParameter")
            && let Err(error) = checked_owned_parameters(references, element.id())
        {
            issues.push(StructuralDiagnostic {
                severity: if matches!(error, QueryError::Unsupported { .. }) {
                    StructuralSeverity::Warning
                } else {
                    StructuralSeverity::Error
                },
                code: if matches!(error, QueryError::Unsupported { .. }) {
                    "MDE_PARAMETER_OWNERSHIP_UNVERIFIED"
                } else {
                    "UML_PARAMETER_OWNERSHIP_STORAGE"
                },
                subject_id: element.id().into(),
                message: error.to_string(),
            });
        }
        if element.conforms_to("Parameter")
            && ["operation", "behavior"]
                .iter()
                .any(|key| element.properties().contains_key(*key))
        {
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Error,
                code: "UML_PARAMETER_OWNERSHIP_STORAGE",
                subject_id: element.id().into(),
                message: "operation and behavior must be projected from canonical owns edges"
                    .into(),
            });
        }
        if element.conforms_to("Operation") {
            if let Err(error) = checked_return_parameter(references, element.id()) {
                issues.push(StructuralDiagnostic {
                    severity: if matches!(error, QueryError::Unsupported { .. }) {
                        StructuralSeverity::Warning
                    } else {
                        StructuralSeverity::Error
                    },
                    code: if matches!(error, QueryError::Unsupported { .. }) {
                        "MDE_OPERATION_PARAMETERS_UNVERIFIED"
                    } else {
                        "UML_OPERATION_PARAMETERS"
                    },
                    subject_id: element.id().into(),
                    message: error.to_string(),
                });
            }
            if let Err(message) = validate_body_condition(references, element) {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Error,
                    code: "UML_OPERATION_BODY_CONDITION",
                    subject_id: element.id().into(),
                    message,
                });
            } else if element
                .properties()
                .get("bodyCondition")
                .is_some_and(|value| !value.is_null())
            {
                issues.push(StructuralDiagnostic { severity: StructuralSeverity::Warning,
                    code: "MDE_OPERATION_CONDITION_UNSUPPORTED", subject_id: element.id().into(),
                    message: "Operation bodyCondition ownership and only_body_for_query are checked; its constraint expression is not evaluated".into() });
            }
        } else if element.conforms_to("Parameter") {
            validate_parameter_relations(references, element, &mut issues);
            let ElementRef::Object(parameter) = element else {
                continue;
            };
            let problem = match direction(parameter) {
                Err(error) => Some(error),
                Ok(direction) => {
                    let effect = parameter.properties.get("effect").and_then(Value::as_str);
                    let is_exception = parameter
                        .properties
                        .get("isException")
                        .and_then(Value::as_bool)
                        == Some(true);
                    let is_stream = parameter
                        .properties
                        .get("isStream")
                        .and_then(Value::as_bool)
                        == Some(true);
                    if (effect == Some("delete") && !matches!(direction.as_str(), "in" | "inout"))
                        || (effect == Some("create")
                            && !matches!(direction.as_str(), "out" | "inout" | "return"))
                    {
                        Some(
                            "Parameter::in_and_out restricts create/delete effects by direction"
                                .into(),
                        )
                    } else if is_exception && matches!(direction.as_str(), "in" | "inout") {
                        Some("Parameter::not_exception prohibits input exception Parameters".into())
                    } else if is_exception && is_stream {
                        Some("Parameter::stream_and_exception prohibits both flags together".into())
                    } else if effect.is_some()
                        && parameter
                            .properties
                            .get("type")
                            .and_then(|value| super::references::reference_id(value).ok().flatten())
                            .and_then(|id| references.element(id).ok())
                            .is_some_and(|target| target.conforms_to("DataType"))
                    {
                        Some(
                            "Parameter::object_effect prohibits an effect on a DataType Parameter"
                                .into(),
                        )
                    } else {
                        None
                    }
                }
            };
            if let Some(message) = problem {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Error,
                    code: "UML_PARAMETER_CONSTRAINT",
                    subject_id: element.id().into(),
                    message,
                });
            }
        }
    }
    issues
}
