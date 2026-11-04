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

// Composite roles can share a target type (State entry/exit/doActivity).
// Resolve canonical containment without arbitrarily choosing an ambiguous role.
fn composite_role_empty(
    references: &ReferenceIndex<'_>, owner: ElementRef<'_>, role: &str, target_type: &str,
) -> Result<bool, QueryError> {
    let metaclass = owner.metaclass().ok_or_else(|| QueryError::unsupported(owner.id(), "owner metaclass is unresolved"))?;
    let descriptor = schema::uml_property(metaclass, role)
        .map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(owner.id(), format!("missing {metaclass}::{role} contract")))?;
    let mut checks = Vec::new();
    if let Some(value) = owner.properties().get(role) {
        checks.push(match property_contracts::check_value(references, owner, descriptor, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(owner.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(owner.id(), message)),
        });
        // A well-formed external reference also establishes nonempty content.
        // Invalid contracts are still checked and dominate the cardinality result.
        checks.push(Ok(if descriptor.upper == "1" { value.is_null() }
            else { value.as_array().is_some_and(Vec::is_empty) }));
    }
    for child in references.explicit_owned_elements(owner.id())
        .map_err(|error| QueryError::invalid(owner.id(), error))?
    {
        if child.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(child.id(), format!("owned element metaclass is unresolved; {role} membership is unknown"))));
        } else if child.conforms_to(target_type) {
            checks.push(classifiers::contributes_to_owned_union(owner, child, descriptor.id.fragment)
                .map(|member| !member));
        }
    }
    require_all(checks)
}

fn state_submachine_empty(references: &ReferenceIndex<'_>, state: ElementRef<'_>) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("State", "submachine")
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

fn state_regions_empty(references: &ReferenceIndex<'_>, state: ElementRef<'_>) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("State", "region")
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
                // A different canonical owner excludes this State even
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

fn transition_count(references: &ReferenceIndex<'_>, vertex: ElementRef<'_>, role: &str, minimum: usize, maximum: usize) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("Transition", role)
        .map_err(|error| QueryError::unsupported(vertex.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(vertex.id(), format!("missing Transition::{role} contract")))?;
    let mut checks = Vec::new();
    let mut known = 0_usize;
    let mut possible = 0_usize;
    let mut unresolved = None;
    // Vertex incoming/outgoing derive from Transition target/source. Persisted
    // caches never establish presence or absence.
    for transition in references.model().elements().filter(|element| element.conforms_to("Transition")) {
        references.element(transition.id()).map_err(|error| QueryError::invalid(transition.id(), error))?;
        let canonical = match transition {
            ElementRef::Relationship(relation) => Some(serde_json::json!({"$ref":if role == "source" { &relation.from } else { &relation.to }})),
            ElementRef::Object(_) => None,
        };
        let authored = transition.properties().get(role);
        let Some(source) = canonical.as_ref().or(authored) else {
            possible += 1;
            unresolved.get_or_insert_with(|| QueryError::unsupported(transition.id(), format!("Transition {role} is not materialized")));
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
            checks.push(Err(QueryError::invalid(transition.id(), format!("authored {role} contradicts canonical relation.{}", if role == "source" { "from" } else { "to" }))));
        }
        if let Some(source) = identity {
            if source == vertex.id() { known += 1; }
        } else {
            possible += 1;
        }
    }
    // Each unresolved identity adds at most one candidate. Evaluate the whole
    // cardinality interval: known counterexamples dominate uncertainty, while
    // a range entirely inside the bounds needs no further identity resolution.
    let upper = known.saturating_add(possible);
    checks.push(if known > maximum || upper < minimum { Ok(false) }
        else if known >= minimum && upper <= maximum { Ok(true) }
        else { Err(unresolved.unwrap_or_else(|| QueryError::unsupported(vertex.id(), format!("{role} membership is unresolved")))) });
    require_all(checks)
}

fn outgoing_at_most(references: &ReferenceIndex<'_>, vertex: ElementRef<'_>, maximum: usize) -> Result<bool, QueryError> {
    transition_count(references, vertex, "source", 0, maximum)
}

fn pseudostate_incident_transitions(
    references: &ReferenceIndex<'_>, vertex: ElementRef<'_>, required_kind: &str,
    incoming: (usize, usize), outgoing: (usize, usize),
) -> Result<bool, QueryError> {
    let kind = schema::read_effective_scalar(vertex, "kind")
        .map_err(|error| QueryError::invalid(vertex.id(), error))?;
    if kind.as_str() != Some(required_kind) { return Ok(true); }
    require_all([
        transition_count(references, vertex, "source", outgoing.0, outgoing.1),
        transition_count(references, vertex, "target", incoming.0, incoming.1),
    ])
}

fn pseudostate_single_outgoing(references: &ReferenceIndex<'_>, vertex: ElementRef<'_>, kinds: &[&str]) -> Result<bool, QueryError> {
    let kind = schema::read_effective_scalar(vertex, "kind")
        .map_err(|error| QueryError::invalid(vertex.id(), error))?;
    if kind.as_str().is_some_and(|kind| kinds.contains(&kind)) { outgoing_at_most(references, vertex, 1) }
    else { Ok(true) }
}

fn transition_endpoint<'a>(
    references: &ReferenceIndex<'a>, transition: ElementRef<'a>, role: &str,
) -> Result<ElementRef<'a>, QueryError> {
    references.element(transition.id()).map_err(|error| QueryError::invalid(transition.id(), error))?;
    let descriptor = schema::uml_property("Transition", role)
        .map_err(|error| QueryError::unsupported(transition.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(transition.id(), format!("missing Transition::{role} contract")))?;
    let canonical = match transition {
        ElementRef::Relationship(relation) => Some(serde_json::json!({"$ref": if role == "source" { &relation.from } else { &relation.to }})),
        ElementRef::Object(_) => None,
    };
    let authored = transition.properties().get(role);
    for value in canonical.as_ref().into_iter().chain(authored) {
        if let PropertyOutcome::Violation(message) = property_contracts::check_value(references, transition, descriptor, value) {
            return Err(QueryError::invalid(transition.id(), message));
        }
    }
    if let (Some(canonical), Some(authored)) = (canonical.as_ref(), authored)
        && let (Ok(expected), Ok(actual)) = (reference_id(canonical), reference_id(authored))
        && expected != actual
    {
        return Err(QueryError::invalid(transition.id(), format!("authored {role} contradicts canonical relation endpoint")));
    }
    let value = canonical.as_ref().or(authored)
        .ok_or_else(|| QueryError::unsupported(transition.id(), format!("Transition {role} is not materialized")))?;
    let id = reference_id(value).ok().flatten()
        .ok_or_else(|| QueryError::unsupported(transition.id(), format!("Transition {role} is unresolved")))?;
    references.element(id).map_err(|error| QueryError::invalid(transition.id(), error))
}

fn is_state(endpoint: Result<ElementRef<'_>, QueryError>) -> Result<bool, QueryError> {
    let endpoint = endpoint?;
    if endpoint.metaclass().is_none() {
        Err(QueryError::unsupported(endpoint.id(), "endpoint metaclass is unresolved"))
    } else { Ok(endpoint.conforms_to("State")) }
}

fn is_pseudostate_kind(endpoint: Result<ElementRef<'_>, QueryError>, kind: &str) -> Result<bool, QueryError> {
    let endpoint = endpoint?;
    if endpoint.metaclass().is_none() {
        return Err(QueryError::unsupported(endpoint.id(), "endpoint metaclass is unresolved"));
    }
    if !endpoint.conforms_to("Pseudostate") { return Ok(false); }
    schema::read_effective_scalar(endpoint, "kind")
        .map(|value| value.as_str() == Some(kind))
        .map_err(|error| QueryError::invalid(endpoint.id(), error))
}

fn implies(antecedent: Result<bool, QueryError>, consequent: Result<bool, QueryError>) -> Result<bool, QueryError> {
    // Invalid contracts cannot be concealed by an unrelated Boolean result.
    match (antecedent, consequent) {
        (Err(error @ QueryError::Invalid { .. }), _) | (_, Err(error @ QueryError::Invalid { .. })) => Err(error),
        (Ok(false), _) | (_, Ok(true)) => Ok(true),
        (Ok(true), result) => result,
        (Err(error), _) => Err(error),
    }
}

fn transition_internal(references: &ReferenceIndex<'_>, transition: ElementRef<'_>) -> Result<bool, QueryError> {
    let kind = schema::read_effective_scalar(transition, "kind")
        .map_err(|error| QueryError::invalid(transition.id(), error))?;
    if kind.as_str() != Some("internal") { return Ok(true); }
    let source = transition_endpoint(references, transition, "source");
    let target = transition_endpoint(references, transition, "target");
    let state = is_state(transition_endpoint(references, transition, "source"));
    let identical = match (source, target) {
        (Ok(source), Ok(target)) => Ok(source.id() == target.id()),
        (Err(error @ QueryError::Invalid { .. }), _) | (_, Err(error @ QueryError::Invalid { .. })) => Err(error),
        (Err(error), _) | (_, Err(error)) => Err(error),
    };
    require_all([state, identical])
}

fn transition_external(references: &ReferenceIndex<'_>, transition: ElementRef<'_>) -> Result<bool, QueryError> {
    let kind = schema::read_effective_scalar(transition, "kind")
        .map_err(|error| QueryError::invalid(transition.id(), error))?;
    if kind.as_str() != Some("external") { return Ok(true); }
    is_pseudostate_kind(transition_endpoint(references, transition, "source"), "entryPoint").map(|entry| !entry)
}

fn transition_local(references: &ReferenceIndex<'_>, transition: ElementRef<'_>) -> Result<bool, QueryError> {
    let kind = schema::read_effective_scalar(transition, "kind")
        .map_err(|error| QueryError::invalid(transition.id(), error))?;
    if kind.as_str() != Some("local") { return Ok(true); }
    let source = transition_endpoint(references, transition, "source")?;
    if source.metaclass().is_none() {
        return Err(QueryError::unsupported(source.id(), "source metaclass is unresolved"));
    }
    if source.conforms_to("State") {
        // UML State::isComposite = region->notEmpty(). A serialized derived
        // Boolean is not evidence of the underlying graph's cardinality.
        state_regions_empty(references, source).map(|empty| !empty)
    } else {
        is_pseudostate_kind(Ok(source), "entryPoint")
    }
}

fn state_submachine_or_regions(references: &ReferenceIndex<'_>, state: ElementRef<'_>) -> Result<bool, QueryError> {
    // isComposite implies not isSubmachineState, evaluated from the two
    // association projections, including their authored opposite evidence.
    implies(state_regions_empty(references, state).map(|empty| !empty),
        state_submachine_empty(references, state))
}

fn transition_segment_state(references: &ReferenceIndex<'_>, transition: ElementRef<'_>, fork: bool) -> Result<bool, QueryError> {
    let (condition_role, state_role, kind) = if fork { ("source", "target", "fork") } else { ("target", "source", "join") };
    implies(
        is_pseudostate_kind(transition_endpoint(references, transition, condition_role), kind),
        is_state(transition_endpoint(references, transition, state_role)),
    )
}

fn transition_segment_guards(references: &ReferenceIndex<'_>, transition: ElementRef<'_>, fork: bool) -> Result<bool, QueryError> {
    let (role, kind) = if fork { ("source", "fork") } else { ("target", "join") };
    implies(
        is_pseudostate_kind(transition_endpoint(references, transition, role), kind),
        require_all([
            composite_role_empty(references, transition, "guard", "Constraint"),
            composite_role_empty(references, transition, "trigger", "Trigger"),
        ]),
    )
}

fn transition_outgoing_pseudostates(references: &ReferenceIndex<'_>, transition: ElementRef<'_>) -> Result<bool, QueryError> {
    let condition = transition_endpoint(references, transition, "source").and_then(|source| {
        if source.metaclass().is_none() {
            return Err(QueryError::unsupported(source.id(), "source metaclass is unresolved"));
        }
        if !source.conforms_to("Pseudostate") { return Ok(false); }
        schema::read_effective_scalar(source, "kind")
            .map(|kind| kind.as_str() != Some("initial"))
            .map_err(|error| QueryError::invalid(source.id(), error))
    });
    implies(condition, composite_role_empty(references, transition, "trigger", "Trigger"))
}

fn pseudostate_outgoing_from_initial(references: &ReferenceIndex<'_>, point: ElementRef<'_>) -> Result<bool, QueryError> {
    let kind = schema::read_effective_scalar(point, "kind")
        .map_err(|error| QueryError::invalid(point.id(), error))?;
    if kind.as_str() != Some("initial") { return Ok(true); }
    let mut checks = Vec::new();
    for transition in references.model().elements().filter(|element| element.conforms_to("Transition")) {
        // Identity establishes membership even if the endpoint's metaclass is
        // unresolved. An external/missing source remains only a possible member.
        let member = transition_endpoint(references, transition, "source")
            .map(|source| source.id() == point.id());
        if matches!(&member, Ok(false)) { continue; }
        // If both roles are empty, a possible member cannot violate the rule.
        // Conversely, a known violating member dominates other unknown members.
        checks.push(implies(member, require_all([
            composite_role_empty(references, transition, "guard", "Constraint"),
            composite_role_empty(references, transition, "trigger", "Trigger"),
        ])));
    }
    require_all(checks)
}

fn region_pseudostate_limit(references: &ReferenceIndex<'_>, region: ElementRef<'_>, kind: &str) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("Region", "subvertex")
        .map_err(|error| QueryError::unsupported(region.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(region.id(), "missing Region::subvertex contract"))?;
    let opposite = schema::uml_property("Vertex", "container")
        .map_err(|error| QueryError::unsupported(region.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(region.id(), "missing Vertex::container contract"))?;
    let mut checks = Vec::new();
    // Local identities are deduplicated across forward, canonical and inverse
    // evidence. true means membership is proven, false means merely possible.
    let mut members = std::collections::HashMap::<String, bool>::new();
    let mut external = HashSet::new();
    if let Some(value) = region.properties().get("subvertex") {
        if let PropertyOutcome::Violation(message) = property_contracts::check_value(references, region, descriptor, value) {
            checks.push(Err(QueryError::invalid(region.id(), message)));
        }
        if let Some(values) = value.as_array() {
            for value in values {
                if let Ok(Some(id)) = reference_id(value) {
                    members.insert(id.to_owned(), true);
                } else {
                    external.insert(value.to_string());
                }
            }
        }
    }
    for child in references.explicit_owned_elements(region.id())
        .map_err(|error| QueryError::invalid(region.id(), error))?
    {
        if child.conforms_to("Vertex") {
            // Region has one direct composite Vertex role. An unresolved
            // forward projection cannot erase this canonical membership proof.
            members.insert(child.id().to_owned(), true);
            match classifiers::contributes_to_owned_union(region, child, descriptor.id.fragment) {
                Ok(true) | Err(QueryError::Unsupported { .. }) => {}
                Ok(false) => checks.push(Err(QueryError::invalid(child.id(), "canonical Region ownership excludes subvertex"))),
                Err(error) => checks.push(Err(error)),
            }
        } else if child.metaclass().is_none() {
            members.entry(child.id().to_owned()).or_insert(false);
        }
    }
    for vertex in references.model().elements().filter(|element| element.conforms_to("Vertex") || element.metaclass().is_none()) {
        let Some(value) = vertex.properties().get("container") else { continue; };
        let owner = references.model().owner(vertex)
            .map_err(|error| QueryError::invalid(vertex.id(), error))?;
        let canonical_member = owner.is_some_and(|owner| owner.id() == region.id());
        match reference_id(value) {
            Ok(Some(id)) if id == region.id() => {
                // A same-named extension property is not proven to be the UML
                // Vertex opposite until the element's metaclass is resolved.
                members.entry(vertex.id().to_owned())
                    .and_modify(|certain| *certain |= vertex.conforms_to("Vertex"))
                    .or_insert_with(|| vertex.conforms_to("Vertex"));
                if vertex.conforms_to("Vertex") && !canonical_member {
                    checks.push(Err(QueryError::invalid(vertex.id(), "Vertex::container contradicts canonical Region ownership")));
                }
            }
            Ok(_) => {
                if canonical_member && vertex.conforms_to("Vertex") {
                    checks.push(Err(QueryError::invalid(vertex.id(), "Vertex::container excludes its canonical Region")));
                }
                continue;
            }
            Err(_) if owner.is_some() && !canonical_member => continue,
            Err(_) => { members.entry(vertex.id().to_owned()).or_insert(false); }
        }
        if vertex.conforms_to("Vertex")
            && let PropertyOutcome::Violation(message) = property_contracts::check_value(references, vertex, opposite, value)
        {
            checks.push(Err(QueryError::invalid(vertex.id(), message)));
        }
    }
    let mut known = 0_usize;
    let mut possible = external.len();
    for (id, certain) in members {
        let vertex = references.element(&id).map_err(|error| QueryError::invalid(region.id(), error))?;
        if vertex.metaclass().is_none() {
            possible += 1;
        } else if vertex.conforms_to("Pseudostate") {
            match schema::read_effective_scalar(vertex, "kind") {
                Ok(value) if value.as_str() == Some(kind) => {
                    if certain { known += 1; } else { possible += 1; }
                }
                Ok(_) => {}
                Err(error) => checks.push(Err(QueryError::invalid(vertex.id(), error))),
            }
        }
    }
    checks.push(if known > 1 { Ok(false) }
        else if known.saturating_add(possible) <= 1 { Ok(true) }
        else { Err(QueryError::unsupported(region.id(), format!("{kind} subvertex membership or kind is unresolved"))) });
    require_all(checks)
}

fn state_connections_empty(
    references: &ReferenceIndex<'_>, state: ElementRef<'_>, role: &str, point_type: &str,
) -> Result<bool, QueryError> {
    let opposite = schema::uml_property(point_type, "state")
        .map_err(|error| QueryError::unsupported(state.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(state.id(), format!("missing {point_type}::state contract")))?;
    let mut checks = vec![composite_role_empty(references, state, role, point_type)];
    for point in references.model().elements().filter(|element| element.conforms_to(point_type)) {
        let Some(value) = point.properties().get("state") else { continue; };
        let owner = references.model().owner(point)
            .map_err(|error| QueryError::invalid(point.id(), error))?;
        let canonical_member = owner.is_some_and(|owner| owner.id() == state.id());
        match reference_id(value) {
            Ok(Some(id)) if id == state.id() => {
                references.element(point.id()).map_err(|error| QueryError::invalid(point.id(), error))?;
                checks.push(Ok(false));
                if !canonical_member {
                    checks.push(Err(QueryError::invalid(point.id(), "authored state contradicts canonical connection ownership")));
                }
            }
            Ok(_) => {
                if canonical_member {
                    checks.push(Err(QueryError::invalid(point.id(), "authored state excludes its canonical connection owner")));
                }
                continue;
            }
            Err(_) if owner.is_some() && !canonical_member => continue,
            Err(_) => {}
        }
        checks.push(match property_contracts::check_value(references, point, opposite, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(point.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(point.id(), message)),
        });
    }
    require_all(checks)
}

fn state_composite_states(references: &ReferenceIndex<'_>, state: ElementRef<'_>) -> Result<bool, QueryError> {
    implies(state_connections_empty(references, state, "connectionPoint", "Pseudostate").map(|empty| !empty),
        state_regions_empty(references, state).map(|empty| !empty))
}

fn state_submachine_states(references: &ReferenceIndex<'_>, state: ElementRef<'_>) -> Result<bool, QueryError> {
    // Follow the published OCL literally: isSubmachineState implies
    // connection->notEmpty(). Its accompanying prose suggests the converse;
    // do not silently substitute that different implication here.
    implies(state_submachine_empty(references, state).map(|empty| !empty),
        state_connections_empty(references, state, "connection", "ConnectionPointReference").map(|empty| !empty))
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
        if element.conforms_to("State") {
            constraints.push((state_composite_states(references, element),
                "UML_STATE_COMPOSITE_STATES", "MDE_STATE_COMPOSITE_STATES_UNVERIFIED",
                "State-composite_states", "a State with connection points must have Regions"));
            constraints.push((state_submachine_states(references, element),
                "UML_STATE_SUBMACHINE_STATES", "MDE_STATE_SUBMACHINE_STATES_UNVERIFIED",
                "State-submachine_states", "a submachine State must have connection point references"));
            constraints.push((state_submachine_or_regions(references, element),
                "UML_STATE_SUBMACHINE_OR_REGIONS", "MDE_STATE_SUBMACHINE_OR_REGIONS_UNVERIFIED",
                "State-submachine_or_regions", "a composite State cannot also be a submachine State"));
        }
        if element.conforms_to("Pseudostate") {
            constraints.push((pseudostate_outgoing_from_initial(references, element),
                "UML_PSEUDOSTATE_OUTGOING_FROM_INITIAL", "MDE_PSEUDOSTATE_OUTGOING_FROM_INITIAL_UNVERIFIED",
                "Pseudostate-outgoing_from_initial", "a Transition from an initial Pseudostate cannot have a guard or triggers"));
            constraints.push((pseudostate_incident_transitions(references, element, "fork", (1, 1), (2, usize::MAX)),
                "UML_PSEUDOSTATE_FORK_VERTEX", "MDE_PSEUDOSTATE_FORK_VERTEX_UNVERIFIED",
                "Pseudostate-fork_vertex", "a fork Pseudostate requires exactly one incoming and at least two outgoing Transitions"));
            constraints.push((pseudostate_incident_transitions(references, element, "join", (2, usize::MAX), (1, 1)),
                "UML_PSEUDOSTATE_JOIN_VERTEX", "MDE_PSEUDOSTATE_JOIN_VERTEX_UNVERIFIED",
                "Pseudostate-join_vertex", "a join Pseudostate requires exactly one outgoing and at least two incoming Transitions"));
            constraints.push((pseudostate_incident_transitions(references, element, "choice", (1, usize::MAX), (1, usize::MAX)),
                "UML_PSEUDOSTATE_CHOICE_VERTEX", "MDE_PSEUDOSTATE_CHOICE_VERTEX_UNVERIFIED",
                "Pseudostate-choice_vertex", "a choice Pseudostate requires at least one incoming and one outgoing Transition"));
            constraints.push((pseudostate_incident_transitions(references, element, "junction", (1, usize::MAX), (1, usize::MAX)),
                "UML_PSEUDOSTATE_JUNCTION_VERTEX", "MDE_PSEUDOSTATE_JUNCTION_VERTEX_UNVERIFIED",
                "Pseudostate-junction_vertex", "a junction Pseudostate requires at least one incoming and one outgoing Transition"));
            constraints.push((pseudostate_single_outgoing(references, element, &["initial"]),
                "UML_PSEUDOSTATE_INITIAL_VERTEX", "MDE_PSEUDOSTATE_INITIAL_VERTEX_UNVERIFIED",
                "Pseudostate-initial_vertex", "an initial Pseudostate can have at most one outgoing Transition"));
            constraints.push((pseudostate_single_outgoing(references, element, &["deepHistory", "shallowHistory"]),
                "UML_PSEUDOSTATE_HISTORY_VERTICES", "MDE_PSEUDOSTATE_HISTORY_VERTICES_UNVERIFIED",
                "Pseudostate-history_vertices", "a history Pseudostate can have at most one outgoing Transition"));
        }
        if element.conforms_to("Region") {
            for (kind, code, unknown, constraint) in [
                ("initial", "UML_REGION_INITIAL_VERTEX", "MDE_REGION_INITIAL_VERTEX_UNVERIFIED", "Region-initial_vertex"),
                ("deepHistory", "UML_REGION_DEEP_HISTORY_VERTEX", "MDE_REGION_DEEP_HISTORY_VERTEX_UNVERIFIED", "Region-deep_history_vertex"),
                ("shallowHistory", "UML_REGION_SHALLOW_HISTORY_VERTEX", "MDE_REGION_SHALLOW_HISTORY_VERTEX_UNVERIFIED", "Region-shallow_history_vertex"),
            ] {
                constraints.push((region_pseudostate_limit(references, element, kind), code, unknown, constraint,
                    "a Region cannot have more than one direct Pseudostate of this kind"));
            }
            constraints.push((region_owned(references, element),
                "UML_REGION_OWNED", "MDE_REGION_OWNED_UNVERIFIED",
                "Region-owned", "a Region cannot have both state and stateMachine owners"));
        }
        if element.conforms_to("Transition") {
            constraints.push((transition_local(references, element),
                "UML_TRANSITION_STATE_IS_LOCAL", "MDE_TRANSITION_STATE_IS_LOCAL_UNVERIFIED",
                "Transition-state_is_local", "a local Transition must originate at a composite State or entryPoint"));
            constraints.push((transition_segment_guards(references, element, true),
                "UML_TRANSITION_FORK_SEGMENT_GUARDS", "MDE_TRANSITION_FORK_SEGMENT_GUARDS_UNVERIFIED",
                "Transition-fork_segment_guards", "a Transition from a fork cannot have a guard or triggers"));
            constraints.push((transition_segment_guards(references, element, false),
                "UML_TRANSITION_JOIN_SEGMENT_GUARDS", "MDE_TRANSITION_JOIN_SEGMENT_GUARDS_UNVERIFIED",
                "Transition-join_segment_guards", "a Transition to a join cannot have a guard or triggers"));
            constraints.push((transition_outgoing_pseudostates(references, element),
                "UML_TRANSITION_OUTGOING_PSEUDOSTATES", "MDE_TRANSITION_OUTGOING_PSEUDOSTATES_UNVERIFIED",
                "Transition-outgoing_pseudostates", "a Transition from a non-initial Pseudostate cannot have triggers"));
            constraints.push((transition_internal(references, element),
                "UML_TRANSITION_STATE_IS_INTERNAL", "MDE_TRANSITION_STATE_IS_INTERNAL_UNVERIFIED",
                "Transition-state_is_internal", "an internal Transition requires the same State as source and target"));
            constraints.push((transition_external(references, element),
                "UML_TRANSITION_STATE_IS_EXTERNAL", "MDE_TRANSITION_STATE_IS_EXTERNAL_UNVERIFIED",
                "Transition-state_is_external", "an external Transition cannot originate at an entryPoint"));
            constraints.push((transition_segment_state(references, element, true),
                "UML_TRANSITION_FORK_SEGMENT_STATE", "MDE_TRANSITION_FORK_SEGMENT_STATE_UNVERIFIED",
                "Transition-fork_segment_state", "a Transition from a fork must target a State"));
            constraints.push((transition_segment_state(references, element, false),
                "UML_TRANSITION_JOIN_SEGMENT_STATE", "MDE_TRANSITION_JOIN_SEGMENT_STATE_UNVERIFIED",
                "Transition-join_segment_state", "a Transition to a join must originate at a State"));
        }
        if element.conforms_to("ProtocolTransition") {
            constraints.push((composite_role_empty(references, element, "effect", "Behavior"),
                "UML_PROTOCOL_TRANSITION_ASSOCIATED_ACTIONS", "MDE_PROTOCOL_TRANSITION_ASSOCIATED_ACTIONS_UNVERIFIED",
                "ProtocolTransition-associated_actions", "a ProtocolTransition cannot have an effect Behavior"));
        }
        if element.conforms_to("FinalState") {
            for (role, code, unknown, constraint, violation) in [
                ("entry", "UML_FINAL_STATE_NO_ENTRY_BEHAVIOR", "MDE_FINAL_STATE_ENTRY_BEHAVIOR_UNVERIFIED",
                    "FinalState-no_entry_behavior", "a FinalState cannot have entry Behavior"),
                ("exit", "UML_FINAL_STATE_NO_EXIT_BEHAVIOR", "MDE_FINAL_STATE_EXIT_BEHAVIOR_UNVERIFIED",
                    "FinalState-no_exit_behavior", "a FinalState cannot have exit Behavior"),
                ("doActivity", "UML_FINAL_STATE_NO_STATE_BEHAVIOR", "MDE_FINAL_STATE_STATE_BEHAVIOR_UNVERIFIED",
                    "FinalState-no_state_behavior", "a FinalState cannot have doActivity Behavior"),
            ] {
                constraints.push((composite_role_empty(references, element, role, "Behavior"), code, unknown, constraint, violation));
            }
            constraints.push((outgoing_at_most(references, element, 0),
                "UML_FINAL_STATE_NO_OUTGOING_TRANSITIONS", "MDE_FINAL_STATE_OUTGOING_UNVERIFIED",
                "FinalState-no_outgoing_transitions", "a FinalState cannot have outgoing Transitions"));
            constraints.push((state_regions_empty(references, element),
                "UML_FINAL_STATE_NO_REGIONS", "MDE_FINAL_STATE_REGIONS_UNVERIFIED",
                "FinalState-no_regions", "a FinalState cannot have Regions"));
            constraints.push((state_submachine_empty(references, element),
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
