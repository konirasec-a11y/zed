//! ParameterSet constraints over canonical parameter and ownership associations.
use super::{
    classifiers::{self, QueryError},
    feature_compatibility::{accept_any, require_all},
    property_contracts::{self, PropertyOutcome},
    references::{ReferenceIndex, reference_id},
    root::ElementRef,
    schema,
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;

fn descriptor(id: &'static str) -> Result<&'static schema::PropertyDescriptor, QueryError> {
    schema::property_by_id(schema::XmiId::new(schema::Document::Uml, id))
        .ok_or_else(|| QueryError::unsupported(id, "missing property contract"))
}
fn check(
    references: &ReferenceIndex<'_>,
    owner: ElementRef<'_>,
    descriptor: &schema::PropertyDescriptor,
    value: &Value,
) -> Result<bool, QueryError> {
    match property_contracts::check_value(references, owner, descriptor, value) {
        PropertyOutcome::Satisfied => Ok(true),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(owner.id(), message)),
        PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(owner.id(), message)),
    }
}
fn members<'a>(
    references: &ReferenceIndex<'a>,
    set: ElementRef<'a>,
) -> Result<(Vec<ElementRef<'a>>, Result<bool, QueryError>), QueryError> {
    let Some(value) = set.properties().get("parameter") else {
        return Ok((
            Vec::new(),
            Err(QueryError::unsupported(
                set.id(),
                "ParameterSet parameter collection is not materialized",
            )),
        ));
    };
    let contract = check(
        references,
        set,
        descriptor("ParameterSet-parameter")?,
        value,
    );
    let local = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| reference_id(v).ok().flatten())
        .filter_map(|id| references.element(id).ok())
        .collect();
    Ok((local, contract))
}
fn direction(parameter: ElementRef<'_>, expected: &str) -> Result<bool, QueryError> {
    if !parameter.conforms_to("Parameter") {
        return Err(QueryError::unsupported(
            parameter.id(),
            "Parameter metaclass is unresolved",
        ));
    }
    schema::read_effective_scalar(parameter, "direction")
        .map(|value| value.as_str() == Some(expected))
        .map_err(|error| QueryError::invalid(parameter.id(), error))
}
fn identities_complete(references: &ReferenceIndex<'_>, value: &Value) -> bool {
    value.as_array().is_some_and(|values| {
        values.iter().all(|value| {
            reference_id(value)
                .ok()
                .flatten()
                .is_some_and(|id| references.element(id).is_ok())
        })
    })
}
fn belongs_to_set(
    references: &ReferenceIndex<'_>,
    parameter: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let mut memberships = Vec::new();
    let mut known = Vec::new();
    let mut excluded = Vec::new();
    for set in references
        .model()
        .elements()
        .filter(|e| e.conforms_to("ParameterSet"))
    {
        let (members, contract) = members(references, set)?;
        let contains = members.iter().any(|p| p.id() == parameter.id());
        if contains {
            known.push(set.id());
        }
        // Unknown member types do not make a fully local collection's identities
        // unknown. External or unmaterialized references can still add members.
        let complete = set
            .properties()
            .get("parameter")
            .is_some_and(|value| identities_complete(references, value));
        if !contains && complete {
            excluded.push(set.id());
        }
        if matches!(contract, Err(QueryError::Invalid { .. })) || (contract.is_err() && !complete) {
            memberships.push(contract);
        }
        memberships.push(Ok(contains));
    }
    if let Some(value) = parameter.properties().get("parameterSet") {
        let contract = check(
            references,
            parameter,
            descriptor("Parameter-parameterSet")?,
            value,
        );
        let inverse = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| reference_id(v).ok().flatten())
            .collect::<Vec<_>>();
        // A stored inverse cannot suppress a known forward membership. A local
        // extra inverse is contradictory only when its forward identities are complete.
        if inverse.iter().any(|id| excluded.contains(id)) {
            memberships.push(Err(QueryError::invalid(
                parameter.id(),
                "parameterSet contradicts forward ParameterSet::parameter",
            )));
        }
        if identities_complete(references, value) && known.iter().any(|id| !inverse.contains(id)) {
            memberships.push(Err(QueryError::invalid(
                parameter.id(),
                "parameterSet omits known forward membership",
            )));
        }
        if contract.is_err() {
            memberships.push(contract);
        }
    }
    accept_any(memberships)
}
fn streaming_consequence(
    references: &ReferenceIndex<'_>,
    set: ElementRef<'_>,
    expected: &str,
) -> Result<bool, QueryError> {
    let owner = references
        .model()
        .owner(set)
        .map_err(|error| QueryError::invalid(set.id(), error))?;
    let mut checks = Vec::new();
    if let Some(value) = set.properties().get("behavioralFeature") {
        checks.push(check(
            references,
            set,
            descriptor("A_ownedParameterSet_behavioralFeature-behavioralFeature")?,
            value,
        ));
        if let Ok(id) = reference_id(value)
            && id != owner.map(ElementRef::id)
        {
            checks.push(Err(QueryError::invalid(
                set.id(),
                "behavioralFeature contradicts canonical ownership",
            )));
        }
    }
    let Some(owner) = owner else {
        checks.push(Err(QueryError::unsupported(
            set.id(),
            "behavioralFeature is not available",
        )));
        return require_all(checks);
    };
    if !owner.conforms_to("BehavioralFeature") {
        checks.push(Err(QueryError::unsupported(
            set.id(),
            "ParameterSet behavioralFeature navigation is unresolved",
        )));
        return require_all(checks);
    }
    checks.push(classifiers::contributes_to_owned_union(
        owner,
        set,
        "BehavioralFeature-ownedParameterSet",
    ));
    for descriptor in classifiers::properties(owner)? {
        if classifiers::contributes(descriptor, "BehavioralFeature-ownedParameter")
            .map_err(|error| QueryError::unsupported(owner.id(), error))?
            && let Some(value) = owner.properties().get(descriptor.name)
        {
            checks.push(check(references, owner, descriptor, value));
        }
    }
    for parameter in references
        .explicit_owned_elements(owner.id())
        .map_err(|error| QueryError::invalid(owner.id(), error))?
    {
        match classifiers::contributes_to_owned_union(
            owner,
            parameter,
            "BehavioralFeature-ownedParameter",
        ) {
            Ok(false) => continue,
            Err(error) => {
                checks.push(Err(error));
                continue;
            }
            Ok(true) => {}
        }
        let same_direction = direction(parameter, expected);
        if matches!(same_direction, Ok(false)) {
            continue;
        }
        let streaming = schema::read_effective_boolean(parameter, "isStream")
            .map_err(|error| QueryError::invalid(parameter.id(), error));
        checks.push(accept_any([
            same_direction.map(|same| !same),
            belongs_to_set(references, parameter),
            streaming,
        ]));
    }
    require_all(checks)
}
fn input(references: &ReferenceIndex<'_>, set: ElementRef<'_>) -> Result<bool, QueryError> {
    let (members, contract) = members(references, set)?;
    let mut checks = Vec::new();
    // Unknown membership is part of each existential, not an unconditional
    // failure: a known direction or a true consequent can settle implication.
    if let Err(error @ QueryError::Invalid { .. }) = &contract {
        return Err(error.clone());
    }
    for expected in ["in", "out"] {
        let mut evidence = members
            .iter()
            .map(|parameter| direction(*parameter, expected))
            .collect::<Vec<_>>();
        if contract.is_err() {
            evidence.push(contract.clone());
        }
        let antecedent = accept_any(evidence);
        if matches!(antecedent, Ok(false)) {
            continue;
        }
        checks.push(accept_any([
            antecedent.map(|present| !present),
            streaming_consequence(references, set, expected),
        ]));
    }
    require_all(checks)
}
pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for set in references
        .model()
        .elements()
        .filter(|e| e.conforms_to("ParameterSet"))
    {
        let (severity, code, message) = match input(references, set) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_PARAMETER_SET_INPUT",
                "unbound input/output Parameters must be streaming".into(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "UML_PARAMETER_SET_INPUT",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_PARAMETER_SET_INPUT_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: set.id().into(),
            message: format!("ParameterSet-input: {message}"),
        });
    }
    issues
}
