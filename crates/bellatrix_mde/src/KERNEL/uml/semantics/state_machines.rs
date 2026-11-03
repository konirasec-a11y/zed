use super::{
    classifiers::{self, QueryError},
    feature_compatibility::require_all,
    property_contracts::{self, PropertyOutcome},
    references::{ReferenceIndex, reference_id},
    root::ElementRef,
    schema,
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use std::collections::HashSet;

fn connection_point_kind(point: ElementRef<'_>) -> Result<bool, QueryError> {
    if point.metaclass().is_none() {
        return Err(QueryError::unsupported(point.id(), "connection point metaclass is unresolved"));
    }
    if !point.conforms_to("Pseudostate") {
        return Err(QueryError::invalid(point.id(), "connectionPoint requires a Pseudostate"));
    }
    let kind = schema::read_effective_scalar(point, "kind")
        .map_err(|error| QueryError::invalid(point.id(), error))?;
    Ok(matches!(kind.as_str(), Some("entryPoint" | "exitPoint")))
}

fn connection_points(references: &ReferenceIndex<'_>, owner: ElementRef<'_>) -> Result<bool, QueryError> {
    let metaclass = owner.metaclass().ok_or_else(|| QueryError::unsupported(owner.id(), "owner metaclass is unresolved"))?;
    let descriptor = schema::uml_property(metaclass, "connectionPoint")
        .map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(owner.id(), "missing connectionPoint contract"))?;
    let mut checks = Vec::new();
    let mut inspected = HashSet::new();
    if let Some(value) = owner.properties().get("connectionPoint") {
        checks.push(match property_contracts::check_value(references, owner, descriptor, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(owner.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(owner.id(), message)),
        });
        // Continue inspecting local members beside an unresolved external member:
        // a known bad kind is decisive even when the complete role is unknown.
        if let Some(values) = value.as_array() {
            for value in values {
                if let Ok(Some(id)) = reference_id(value)
                    && let Ok(point) = references.element(id)
                {
                    inspected.insert(id.to_owned());
                    checks.push(connection_point_kind(point));
                }
            }
        }
    }
    let owned = references.explicit_owned_elements(owner.id())
        .map_err(|error| QueryError::invalid(owner.id(), error))?;
    for child in owned {
        if child.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(child.id(), "owned element metaclass is unresolved; connectionPoint membership is unknown")));
            continue;
        }
        if !child.conforms_to("Pseudostate") { continue; }
        match classifiers::contributes_to_owned_union(owner, child, descriptor.id.fragment) {
            Ok(true) => {
                if inspected.insert(child.id().to_owned()) {
                    checks.push(connection_point_kind(child));
                }
            }
            Ok(false) => {}
            Err(error) => checks.push(Err(error)),
        }
    }
    require_all(checks)
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for element in references.model().elements() {
        let (code, unknown, constraint) = if element.conforms_to("StateMachine") {
            ("UML_STATE_MACHINE_CONNECTION_POINTS", "MDE_STATE_MACHINE_CONNECTION_POINTS_UNVERIFIED", "StateMachine-connection_points")
        } else if element.conforms_to("State") {
            ("UML_STATE_ENTRY_OR_EXIT", "MDE_STATE_ENTRY_OR_EXIT_UNVERIFIED", "State-entry_or_exit")
        } else { continue; };
        let (severity, code, message) = match connection_points(references, element) {
            Ok(true) => continue,
            Ok(false) => (StructuralSeverity::Error, code, "connection points must have kind entryPoint or exitPoint".to_owned()),
            Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, code, message),
            Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, unknown, message),
        };
        issues.push(StructuralDiagnostic {
            severity, code, subject_id: element.id().to_owned(), message: format!("{constraint}: {message}"),
        });
    }
    issues
}
