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

fn final_state_submachine(references: &ReferenceIndex<'_>, state: ElementRef<'_>) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("FinalState", "submachine")
        .map_err(|error| QueryError::unsupported(state.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(state.id(), "missing submachine contract"))?;
    let opposite = schema::uml_property("StateMachine", "submachineState")
        .map_err(|error| QueryError::unsupported(state.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(state.id(), "missing submachineState contract"))?;
    let direct = state.properties().get("submachine");
    let value = direct.unwrap_or(&serde_json::Value::Null);
    let mut checks = vec![match property_contracts::check_value(references, state, descriptor, value) {
        PropertyOutcome::Satisfied => Ok(true),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(state.id(), message)),
        PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(state.id(), message)),
    }, Ok(value.is_null())];
    let mut witnesses = HashSet::new();
    // An authored opposite is association evidence, unlike containment or a
    // stored isSubmachineState cache. Do not silently discard that evidence.
    for machine in references.model().elements().filter(|element| element.conforms_to("StateMachine")) {
        let Some(inverse) = machine.properties().get("submachineState") else { continue; };
        let checked = property_contracts::check_value(references, machine, opposite, inverse);
        // Local identities decide membership independently of an unrelated
        // member's unresolved metaclass. Its property warning remains reported
        // by property-contract validation, without obscuring known exclusion.
        let identities_complete = inverse.as_array().is_some_and(|values| values.iter()
            .all(|value| matches!(reference_id(value), Ok(Some(_)))));
        checks.push(match checked {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(machine.id(), message)),
            PropertyOutcome::Unverified(_) if identities_complete => Ok(true),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(machine.id(), message)),
        });
        let contains = inverse.as_array().is_some_and(|values| values.iter().any(|value|
            matches!(reference_id(value), Ok(Some(id)) if id == state.id())));
        if contains {
            witnesses.insert(machine.id());
            checks.push(Ok(false));
            if let Some(direct) = direct
                && let Ok(target) = reference_id(direct)
                && target != Some(machine.id())
            {
                checks.push(Err(QueryError::invalid(state.id(), "submachine contradicts authored StateMachine::submachineState")));
            }
        } else if identities_complete && matches!(reference_id(value), Ok(Some(id)) if id == machine.id()) {
            checks.push(Err(QueryError::invalid(state.id(), "submachine is excluded by its authored submachineState opposite")));
        }
    }
    if witnesses.len() > 1 {
        checks.push(Err(QueryError::invalid(state.id(), "submachineState opposites identify more than one submachine")));
    }
    // A well-formed external reference still proves nonempty submachine. Type
    // uncertainty cannot turn that known cardinality violation into a warning.
    require_all(checks)
}

fn region_owned(references: &ReferenceIndex<'_>, region: ElementRef<'_>) -> Result<bool, QueryError> {
    let mut checks = Vec::new();
    let owner = references.model().owner(region)
        .map_err(|error| QueryError::invalid(region.id(), error))?;
    if references.model().ownership_links().filter(|edge| edge.to == region.id()).count() > 1 {
        return Err(QueryError::invalid(region.id(), "duplicate canonical Region ownership"));
    }
    let mut canonical_role = None;
    let mut unresolved_owner = false;
    if let Some(owner) = owner {
        references.element(owner.id()).map_err(|error| QueryError::invalid(region.id(), error))?;
        let role = if owner.conforms_to("StateMachine") {
            Some(("stateMachine", "StateMachine-region"))
        } else if owner.conforms_to("State") {
            Some(("state", "State-region"))
        } else if owner.metaclass().is_none() {
            unresolved_owner = true;
            checks.push(Err(QueryError::unsupported(region.id(), "canonical owner metaclass is unresolved")));
            None
        } else { None };
        if let Some((role, composite)) = role {
            // Region has exactly one composite role under each known owner
            // type. Unresolved forward references cannot erase that canonical
            // ownership witness when testing the two inverse roles.
            canonical_role = Some(role);
            match classifiers::contributes_to_owned_union(owner, region, composite) {
                Ok(true) => {}
                Ok(false) => checks.push(Err(QueryError::invalid(region.id(), "canonical owner excludes the Region composite role"))),
                Err(error) => checks.push(Err(error)),
            }
        }
    }
    let mut present = Vec::new();
    for role in ["state", "stateMachine"] {
        let descriptor = schema::uml_property("Region", role)
            .map_err(|error| QueryError::unsupported(region.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(region.id(), format!("missing Region::{role} contract")))?;
        let authored = region.properties().get(role);
        present.push(canonical_role == Some(role) || authored.is_some_and(|value| !value.is_null()));
        let Some(value) = authored else { continue; };
        checks.push(match property_contracts::check_value(references, region, descriptor, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(region.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(region.id(), message)),
        });
        if let Ok(target) = reference_id(value) {
            if let Some(target) = target {
                // A local identity different from the canonical owner is a
                // contradiction even if that owner's metaclass is unresolved.
                if owner.map(ElementRef::id) != Some(target)
                    || (!unresolved_owner && canonical_role != Some(role))
                {
                    checks.push(Err(QueryError::invalid(region.id(), format!("{role} contradicts canonical Region ownership"))));
                }
            } else if canonical_role == Some(role) {
                checks.push(Err(QueryError::invalid(region.id(), format!("null {role} contradicts canonical Region ownership"))));
            }
        }
    }
    checks.push(Ok(present.iter().filter(|present| **present).count() <= 1));
    require_all(checks)
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for element in references.model().elements() {
        let mut constraints = Vec::new();
        if element.conforms_to("StateMachine") {
            constraints.push((connection_points(references, element),
                "UML_STATE_MACHINE_CONNECTION_POINTS", "MDE_STATE_MACHINE_CONNECTION_POINTS_UNVERIFIED",
                "StateMachine-connection_points", "connection points must have kind entryPoint or exitPoint"));
        } else if element.conforms_to("State") {
            constraints.push((connection_points(references, element),
                "UML_STATE_ENTRY_OR_EXIT", "MDE_STATE_ENTRY_OR_EXIT_UNVERIFIED",
                "State-entry_or_exit", "connection points must have kind entryPoint or exitPoint"));
        }
        if element.conforms_to("Region") {
            constraints.push((region_owned(references, element),
                "UML_REGION_OWNED", "MDE_REGION_OWNED_UNVERIFIED",
                "Region-owned", "a Region cannot have both state and stateMachine owners"));
        }
        if element.conforms_to("FinalState") {
            constraints.push((final_state_submachine(references, element),
                "UML_FINAL_STATE_CANNOT_REFERENCE_SUBMACHINE", "MDE_FINAL_STATE_SUBMACHINE_UNVERIFIED",
                "FinalState-cannot_reference_submachine", "a FinalState cannot reference a submachine"));
        }
        for (result, code, unknown, constraint, violation) in constraints {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (StructuralSeverity::Error, code, violation.to_owned()),
                Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, code, message),
                Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, unknown, message),
            };
            issues.push(StructuralDiagnostic {
                severity, code, subject_id: element.id().to_owned(), message: format!("{constraint}: {message}"),
            });
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use crate::{Project, uml::root::validate_changes};
    use serde_json::{Value, json};

    #[test]
    fn final_state_transaction_gate_rejects_new_submachine_and_accepts_repair() -> Result<(), Box<dyn std::error::Error>> {
        let project: Project = serde_json::from_value(json!({
            "id":"model", "name":"Final state gate", "elements":[
                {"id":"final", "kind":"uml.final_state", "name":"Done"},
                {"id":"machine", "kind":"uml.state_machine", "name":"Machine"}
            ]
        }))?;
        let before = project.clone();
        for reference in [json!({"$ref":"machine"}), json!({"href":"other.uml#Machine"})] {
            let mut proposed = project.clone();
            proposed.elements.iter_mut().find(|element| element.id == "final").ok_or("FinalState")?
                .properties.insert("submachine".into(), reference);
            let failure = validate_changes(&project, &proposed).expect_err("the transaction gate must reject new violations");
            assert!(failure.starts_with("UML_FINAL_STATE_CANNOT_REFERENCE_SUBMACHINE:"), "{failure}");
            assert_eq!(project, before);
            let mut repaired = proposed.clone();
            repaired.elements.iter_mut().find(|element| element.id == "final").ok_or("FinalState")?
                .properties.insert("submachine".into(), Value::Null);
            validate_changes(&proposed, &repaired)?;
        }
        let mut proposed = project.clone();
        proposed.elements.iter_mut().find(|element| element.id == "machine").ok_or("StateMachine")?
            .properties.insert("submachineState".into(), json!([{"$ref":"final"}]));
        let failure = validate_changes(&project, &proposed).expect_err("inverse-only edits must also reach the gate");
        assert!(failure.starts_with("UML_FINAL_STATE_CANNOT_REFERENCE_SUBMACHINE:"), "{failure}");
        let mut repaired = proposed.clone();
        repaired.elements.iter_mut().find(|element| element.id == "machine").ok_or("StateMachine")?
            .properties.insert("submachineState".into(), json!([]));
        validate_changes(&proposed, &repaired)?;
        assert_eq!(project, before);
        Ok(())
    }
}
