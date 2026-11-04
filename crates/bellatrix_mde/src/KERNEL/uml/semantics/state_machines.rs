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

fn final_state_regions(references: &ReferenceIndex<'_>, state: ElementRef<'_>) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("FinalState", "region")
        .map_err(|error| QueryError::unsupported(state.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(state.id(), "missing State::region contract"))?;
    let opposite = schema::uml_property("Region", "state")
        .map_err(|error| QueryError::unsupported(state.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(state.id(), "missing Region::state contract"))?;
    let mut checks = Vec::new();
    if let Some(value) = state.properties().get("region") {
        checks.push(match property_contracts::check_value(references, state, descriptor, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(state.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(state.id(), message)),
        });
        // Even unresolved members establish a nonzero authored cardinality.
        checks.push(Ok(value.as_array().is_some_and(Vec::is_empty)));
    }
    let mut owned_regions = HashSet::new();
    for child in references.explicit_owned_elements(state.id())
        .map_err(|error| QueryError::invalid(state.id(), error))?
    {
        if child.conforms_to("Region") {
            owned_regions.insert(child.id());
            checks.push(Ok(false));
            checks.push(region_owned(references, child));
            checks.push(classifiers::contributes_to_owned_union(state, child, "State-region")
                .and_then(|member| if member { Ok(true) } else {
                    Err(QueryError::invalid(state.id(), "canonical ownership is excluded from State::region"))
                }));
        } else if child.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(child.id(), "owned element metaclass is unresolved; Region membership is unknown")));
        }
    }
    for region in references.model().elements().filter(|element| element.conforms_to("Region")) {
        if owned_regions.contains(region.id()) { continue; }
        let Some(value) = region.properties().get("state") else { continue; };
        match reference_id(value) {
            Ok(Some(id)) if id == state.id() => {
                references.element(region.id()).map_err(|error| QueryError::invalid(region.id(), error))?;
                checks.push(Ok(false));
                checks.push(region_owned(references, region));
            }
            Ok(_) => continue,
            Err(_) => {
                // A different canonical owner excludes this FinalState even
                // when the stored inverse is incomplete. Its own contract is
                // still checked on the Region; do not spread that uncertainty.
                let owner = references.model().owner(region)
                    .map_err(|error| QueryError::invalid(region.id(), error))?;
                if owner.is_some_and(|owner| owner.id() != state.id()) { continue; }
            }
        }
        checks.push(match property_contracts::check_value(references, region, opposite, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(region.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(region.id(), message)),
        });
    }
    require_all(checks)
}

fn outgoing_at_most(references: &ReferenceIndex<'_>, vertex: ElementRef<'_>, maximum: usize) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("Transition", "source")
        .map_err(|error| QueryError::unsupported(vertex.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(vertex.id(), "missing Transition::source contract"))?;
    let mut checks = Vec::new();
    let mut known = 0_usize;
    let mut possible = 0_usize;
    let mut unresolved = None;
    // Vertex::outgoing is derived from Transition::source. The persisted
    // outgoing cache is never evidence for either presence or absence.
    for transition in references.model().elements().filter(|element| element.conforms_to("Transition")) {
        references.element(transition.id()).map_err(|error| QueryError::invalid(transition.id(), error))?;
        let canonical = match transition {
            ElementRef::Relationship(relation) => Some(serde_json::json!({"$ref":relation.from})),
            ElementRef::Object(_) => None,
        };
        let authored = transition.properties().get("source");
        let Some(source) = canonical.as_ref().or(authored) else {
            possible += 1;
            unresolved.get_or_insert_with(|| QueryError::unsupported(transition.id(), "Transition source is not materialized"));
            continue;
        };
        let identity = reference_id(source).ok().flatten();
        for value in canonical.as_ref().into_iter().chain(authored) {
            checks.push(match property_contracts::check_value(references, transition, descriptor, value) {
                PropertyOutcome::Satisfied => Ok(true),
                PropertyOutcome::Violation(message) => Err(QueryError::invalid(transition.id(), message)),
                // Complete local identities decide membership independently
                // of unresolved metaclasses or redundant reference envelopes.
                PropertyOutcome::Unverified(message) => {
                    if identity.is_none() {
                        unresolved.get_or_insert_with(|| QueryError::unsupported(transition.id(), message));
                    }
                    Ok(true)
                },
            });
        }
        if let (Some(canonical), Some(authored)) = (canonical.as_ref(), authored)
            && let (Ok(expected), Ok(actual)) = (reference_id(canonical), reference_id(authored))
            && expected != actual
        {
            checks.push(Err(QueryError::invalid(transition.id(), "authored source contradicts canonical relation.from")));
        }
        if let Some(source) = identity {
            if source == vertex.id() { known += 1; }
        } else {
            possible += 1;
        }
    }
    // Every unresolved source contributes at most one additional Transition.
    // A known bound violation dominates uncertainty, while an upper bound
    // already within the limit does not need those source identities resolved.
    checks.push(if known > maximum { Ok(false) }
        else if known.saturating_add(possible) <= maximum { Ok(true) }
        else { Err(unresolved.unwrap_or_else(|| QueryError::unsupported(vertex.id(), "outgoing source membership is unresolved"))) });
    require_all(checks)
}

fn pseudostate_initial_vertex(references: &ReferenceIndex<'_>, vertex: ElementRef<'_>) -> Result<bool, QueryError> {
    let kind = schema::read_effective_scalar(vertex, "kind")
        .map_err(|error| QueryError::invalid(vertex.id(), error))?;
    if kind.as_str() == Some("initial") { outgoing_at_most(references, vertex, 1) }
    else { Ok(true) }
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
        if element.conforms_to("Pseudostate") {
            constraints.push((pseudostate_initial_vertex(references, element),
                "UML_PSEUDOSTATE_INITIAL_VERTEX", "MDE_PSEUDOSTATE_INITIAL_VERTEX_UNVERIFIED",
                "Pseudostate-initial_vertex", "an initial Pseudostate can have at most one outgoing Transition"));
        }
        if element.conforms_to("Region") {
            constraints.push((region_owned(references, element),
                "UML_REGION_OWNED", "MDE_REGION_OWNED_UNVERIFIED",
                "Region-owned", "a Region cannot have both state and stateMachine owners"));
        }
        if element.conforms_to("FinalState") {
            constraints.push((outgoing_at_most(references, element, 0),
                "UML_FINAL_STATE_NO_OUTGOING_TRANSITIONS", "MDE_FINAL_STATE_OUTGOING_UNVERIFIED",
                "FinalState-no_outgoing_transitions", "a FinalState cannot have outgoing Transitions"));
            constraints.push((final_state_regions(references, element),
                "UML_FINAL_STATE_NO_REGIONS", "MDE_FINAL_STATE_REGIONS_UNVERIFIED",
                "FinalState-no_regions", "a FinalState cannot have Regions"));
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
    fn final_state_transaction_gate_rejects_new_outgoing_source_and_accepts_repair() -> Result<(), Box<dyn std::error::Error>> {
        for kind in ["uml.transition", "uml.protocol_transition"] {
            let project: Project = serde_json::from_value(json!({
                "id":"model", "name":"Final state outgoing gate", "elements":[
                    {"id":"final", "kind":"uml.final_state", "name":"Done"},
                    {"id":"state", "kind":"uml.state", "name":"State"}
                ], "relations":[{"id":"edge", "kind":kind, "from":"state", "to":"final"}]
            }))?;
            let before = project.clone();
            let mut proposed = project.clone();
            proposed.relations.first_mut().ok_or("Transition")?.from = "final".into();
            let failure = validate_changes(&project, &proposed).expect_err("outgoing source must be rejected");
            assert!(failure.starts_with("UML_FINAL_STATE_NO_OUTGOING_TRANSITIONS:"), "{failure}");
            assert_eq!(project, before);
            validate_changes(&proposed, &project)?;
        }
        Ok(())
    }

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
