use super::{
    classifiers::{self, QueryError},
    feature_compatibility::{accept_any, require_all},
    operations,
    property_contracts::{self, PropertyOutcome},
    references::{ReferenceIndex, external_reference_href, reference_id},
    root::ElementRef,
    schema, value_specifications,
};
use crate::{StructuralDiagnostic, StructuralSeverity};

fn incident_edges<'a>(
    references: &ReferenceIndex<'a>,
    node: ElementRef<'a>,
    role: &str,
    minimum: usize,
    maximum: usize,
    control_only: bool,
) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property("ActivityEdge", role)
        .map_err(|error| QueryError::unsupported(node.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(node.id(), format!("missing ActivityEdge::{role} contract"))
        })?;
    let mut checks = Vec::new();
    let mut known = 0_usize;
    let mut possible = 0_usize;
    // Incidence comes from canonical edges, never a persisted inverse cache.
    let mut edges: Vec<_> = references
        .model()
        .elements()
        .filter(|element| element.conforms_to("ActivityEdge"))
        .collect();
    edges.sort_by(|left, right| left.id().cmp(right.id()));
    for edge in edges {
        let canonical = match edge {
            ElementRef::Relationship(relation) => Some(
                serde_json::json!({"$ref": if role == "source" { &relation.from } else { &relation.to }}),
            ),
            ElementRef::Object(_) => None,
        };
        let authored = edge.properties().get(role);
        let identity = canonical
            .as_ref()
            .or(authored)
            .and_then(|value| reference_id(value).ok().flatten());
        let Some(identity) = identity else {
            // The edge's property contract reports malformed endpoints. Without
            // an identity they cannot establish a violation on an unrelated node.
            if !control_only || !edge.conforms_to("ControlFlow") {
                possible += 1;
            }
            continue;
        };
        if identity != node.id() {
            continue;
        }
        references
            .element(edge.id())
            .map_err(|error| QueryError::invalid(edge.id(), error))?;
        for value in canonical.as_ref().into_iter().chain(authored) {
            checks.push(
                match property_contracts::check_value(references, edge, descriptor, value) {
                    PropertyOutcome::Satisfied => Ok(true),
                    PropertyOutcome::Violation(message) => {
                        Err(QueryError::invalid(edge.id(), message))
                    }
                    PropertyOutcome::Unverified(message) => {
                        Err(QueryError::unsupported(edge.id(), message))
                    }
                },
            );
        }
        if let (Some(canonical), Some(authored)) = (canonical.as_ref(), authored)
            && let (Ok(expected), Ok(actual)) = (reference_id(canonical), reference_id(authored))
            && expected != actual
        {
            checks.push(Err(QueryError::invalid(
                edge.id(),
                format!("authored {role} contradicts the canonical edge endpoint"),
            )));
        }
        known += 1;
        if control_only {
            checks.push(Ok(edge.conforms_to("ControlFlow")));
        }
    }
    let upper = known.saturating_add(possible);
    checks.push(if known > maximum || upper < minimum {
        Ok(false)
    } else if known >= minimum && upper <= maximum && (!control_only || possible == 0) {
        Ok(true)
    } else {
        Err(QueryError::unsupported(
            node.id(),
            format!("ActivityEdge::{role} incidence is unresolved"),
        ))
    });
    require_all(checks)
}

fn activity_endpoint<'a>(
    references: &ReferenceIndex<'a>,
    edge: ElementRef<'a>,
    role: &str,
) -> Result<ElementRef<'a>, QueryError> {
    let descriptor = schema::uml_property("ActivityEdge", role)
        .map_err(|error| QueryError::unsupported(edge.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(edge.id(), format!("missing ActivityEdge::{role} contract"))
        })?;
    let canonical = match edge {
        ElementRef::Relationship(relation) => Some(serde_json::json!({"$ref":
            if role == "source" { &relation.from } else { &relation.to }})),
        ElementRef::Object(_) => None,
    };
    let authored = edge.properties().get(role);
    if canonical.is_none() && authored.is_none() {
        return Err(QueryError::unsupported(
            edge.id(),
            format!("ActivityEdge::{role} endpoint is not represented"),
        ));
    }
    if let (Some(canonical), Some(authored)) = (canonical.as_ref(), authored)
        && !matches!((reference_id(canonical), reference_id(authored)),
            (Ok(Some(expected)), Ok(Some(actual))) if expected == actual)
    {
        return Err(QueryError::unsupported(
            edge.id(),
            format!("authored {role} does not establish the canonical edge endpoint",),
        ));
    }
    let value = canonical.as_ref().or(authored).ok_or_else(|| {
        QueryError::unsupported(
            edge.id(),
            format!("ActivityEdge::{role} endpoint is not represented"),
        )
    })?;
    match property_contracts::check_value(references, edge, descriptor, value) {
        PropertyOutcome::Satisfied => {}
        // Property validation reports malformed data separately. It does not
        // prove an edge predicate false when its endpoint is unresolved.
        PropertyOutcome::Violation(message) | PropertyOutcome::Unverified(message) => {
            return Err(QueryError::unsupported(edge.id(), message));
        }
    }
    let identifier = reference_id(value)
        .map_err(|error| QueryError::unsupported(edge.id(), error))?
        .ok_or_else(|| QueryError::unsupported(edge.id(), "ActivityEdge endpoint is unresolved"))?;
    references
        .element(identifier)
        .map_err(|error| QueryError::unsupported(edge.id(), error))
}

fn control_flow_endpoint(
    references: &ReferenceIndex<'_>,
    edge: ElementRef<'_>,
    role: &str,
) -> Result<bool, QueryError> {
    let endpoint = activity_endpoint(references, edge, role)?;
    if !endpoint.conforms_to("ObjectNode") {
        return Ok(true);
    }
    schema::read_effective_boolean(endpoint, "isControlType")
        .map_err(|error| QueryError::unsupported(endpoint.id(), error))
}

fn activity_edge_candidates<'a>(references: &ReferenceIndex<'a>) -> Vec<ElementRef<'a>> {
    let mut candidates: Vec<_> = references
        .model()
        .elements()
        .filter(|element| {
            element.conforms_to("ActivityEdge")
                || (element.metaclass().is_none()
                    && (element.properties().contains_key("source")
                        || element.properties().contains_key("target")))
        })
        .collect();
    candidates.sort_by(|left, right| left.id().cmp(right.id()));
    candidates
}

fn activity_reference_present(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
    metaclass: &str,
    property: &str,
) -> Result<bool, QueryError> {
    // Presence does not need an external definition. Its unresolved type has
    // a separate contract diagnostic; malformed envelopes prove no presence.
    if element
        .properties()
        .get(property)
        .is_some_and(|value| matches!(external_reference_href(value), Ok(Some(_))))
    {
        return Ok(true);
    }
    optional_activity_reference(references, element, metaclass, property)
        .map(|reference| reference.is_some())
}

// UML.xmi ObjectFlow transformation_behavior / input_and_output_parameter and
// Behavior inputParameters / outputParameters. These are the formal OCL checks;
// type compatibility and absence of side effects mentioned in prose are separate.
fn object_flow_behavior_parameters(
    references: &ReferenceIndex<'_>,
    edge: ElementRef<'_>,
    property: &str,
) -> Result<bool, QueryError> {
    let Some(identifier) =
        optional_activity_reference(references, edge, "ObjectFlow", property)?
    else {
        return Ok(true);
    };
    let (parameters, completeness) = operations::owned_parameter_evidence(references, &identifier)
        .map_err(|error| QueryError::unsupported(edge.id(), error.to_string()))?;
    let mut uncertainty = completeness
        .err()
        .map(|error| QueryError::unsupported(edge.id(), error.to_string()));
    let mut inputs = 0_usize;
    let mut outputs = 0_usize;
    let mut checks = Vec::new();
    for parameter in parameters {
        match operations::direction(parameter) {
            Ok(direction) => {
                if matches!(direction.as_str(), "in" | "inout") {
                    inputs += 1;
                    if property == "selection" {
                        let input = ElementRef::Object(parameter);
                        checks.push(
                            schema::read_effective_boolean(input, "isUnique")
                                .map(|unique| !unique)
                                .map_err(|error| QueryError::unsupported(input.id(), error)),
                        );
                        // MultiplicityElement::is compares the two bound queries
                        // independently. A known mismatch must survive an opaque
                        // opposite bound; malformed data has separate diagnostics.
                        checks.push(
                            value_specifications::lower_bound(references, input)
                                .map(|lower| lower == 0.into())
                                .map_err(|error| {
                                    QueryError::unsupported(input.id(), error.to_string())
                                }),
                        );
                        checks.push(
                            value_specifications::upper_bound(references, input)
                                .map(|upper| upper.is_unlimited())
                                .map_err(|error| {
                                    QueryError::unsupported(input.id(), error.to_string())
                                }),
                        );
                    }
                }
                if matches!(direction.as_str(), "out" | "inout" | "return") {
                    outputs += 1;
                }
            }
            Err(error) => {
                uncertainty = Some(QueryError::unsupported(parameter.id.as_str(), error));
            }
        }
    }
    // Additional unresolved members cannot remove two known inputs or outputs.
    // Conversely, a count of zero or one is conclusive only for a complete set.
    checks.push(if inputs > 1 || outputs > 1 {
        Ok(false)
    } else if let Some(error) = uncertainty {
        Err(error)
    } else {
        Ok(inputs == 1 && outputs == 1)
    });
    // A proven input restriction violation also survives incomplete counts.
    require_all(checks)
}

enum DecisionInputRule {
    ZeroInputs,
    ControlOneInput,
    ObjectOneInput,
    TwoInputs,
}

// UML.xmi DecisionNode input-parameter cardinality rules. ControlFlow uses
// exists; ObjectFlow uses forAll, including its true result on an empty set.
// Neither rule excludes decisionInputFlow from the incoming collection.
fn decision_input_parameters(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
    rule: DecisionInputRule,
) -> Result<bool, QueryError> {
    let (has_decision_input_flow, require_control, expected_inputs) = match rule {
        DecisionInputRule::ZeroInputs => (false, true, 0),
        DecisionInputRule::ControlOneInput => (true, true, 1),
        DecisionInputRule::ObjectOneInput => (false, false, 1),
        DecisionInputRule::TwoInputs => (true, false, 2),
    };
    let incoming = activity_edge_candidates(references).into_iter().map(|edge| {
        let family = if edge.metaclass().is_some() {
            Ok(edge.conforms_to(if require_control { "ControlFlow" } else { "ObjectFlow" }))
        } else {
            Err(QueryError::unsupported(edge.id(), "ActivityEdge metaclass is unresolved"))
        };
        let incident = activity_endpoint(references, edge, "target")
            .map(|target| target.id() == node.id());
        if require_control {
            require_all([incident, family])
        } else {
            accept_any([incident.map(|incident| !incident), family])
        }
    });
    let incoming_family = if require_control {
        accept_any(incoming)
    } else {
        require_all(incoming)
    };
    let guard = require_all([
        activity_reference_present(references, node, "DecisionNode", "decisionInput"),
        activity_reference_present(references, node, "DecisionNode", "decisionInputFlow")
            .map(|present| present == has_decision_input_flow),
        incoming_family,
    ]);
    let input_count = (|| {
        let identifier =
            optional_activity_reference(references, node, "DecisionNode", "decisionInput")?
                .ok_or_else(|| {
                    QueryError::unsupported(node.id(), "decisionInput Behavior is not represented")
                })?;
        let (parameters, completeness) =
            operations::owned_parameter_evidence(references, &identifier)
                .map_err(|error| QueryError::unsupported(node.id(), error.to_string()))?;
        let mut uncertainty = completeness
            .err()
            .map(|error| QueryError::unsupported(node.id(), error.to_string()));
        let mut inputs = 0_usize;
        for parameter in parameters {
            match operations::direction(parameter) {
                Ok(direction) => {
                    if matches!(direction.as_str(), "in" | "inout") {
                        inputs += 1;
                    }
                }
                Err(error) => {
                    uncertainty = Some(QueryError::unsupported(parameter.id.as_str(), error));
                }
            }
        }
        // Too many canonical inputs disprove the count despite unknown neighbors.
        // Equality or a shortfall is conclusive only for complete membership.
        if inputs > expected_inputs {
            Ok(false)
        } else if let Some(error) = uncertainty {
            Err(error)
        } else {
            Ok(inputs == expected_inputs)
        }
    })();
    accept_any([guard.map(|applies| !applies), input_count])
}

fn object_node_selection(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let present = activity_reference_present(references, node, "ObjectNode", "selection")?;
    let ordering = schema::read_effective_scalar(node, "ordering")
        .map_err(|error| QueryError::unsupported(node.id(), error))?;
    // This is an equivalence, not merely selection implies ordered. Neither
    // Boolean side alone decides equality if the other side is unknown.
    Ok(present == (ordering.as_str() == Some("ordered")))
}

fn object_node_edges(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let control = schema::read_effective_boolean(node, "isControlType")
        .map_err(|error| QueryError::unsupported(node.id(), error));
    let all_objects = require_all(
        activity_edge_candidates(references)
            .into_iter()
            .map(|edge| {
                let incident = accept_any([
                    activity_endpoint(references, edge, "source")
                        .map(|endpoint| endpoint.id() == node.id()),
                    activity_endpoint(references, edge, "target")
                        .map(|endpoint| endpoint.id() == node.id()),
                ]);
                let object_flow = if edge.metaclass().is_some() {
                    Ok(edge.conforms_to("ObjectFlow"))
                } else {
                    Err(QueryError::unsupported(
                        edge.id(),
                        "ActivityEdge metaclass is unresolved",
                    ))
                };
                // A possible ObjectFlow cannot invalidate this universal. A known
                // incident non-ObjectFlow can, independently of uncertain neighbors.
                accept_any([incident.map(|incident| !incident), object_flow])
            }),
    );
    accept_any([control, all_objects])
}

fn activity_edge_exists(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
    role: &str,
) -> Result<bool, QueryError> {
    accept_any(
        activity_edge_candidates(references)
            .into_iter()
            .map(|edge| {
                require_all([
                    activity_endpoint(references, edge, role)
                        .map(|endpoint| endpoint.id() == node.id()),
                    if edge.conforms_to("ActivityEdge") {
                        Ok(true)
                    } else {
                        Err(QueryError::unsupported(
                            edge.id(),
                            "ActivityEdge membership is unresolved",
                        ))
                    },
                ])
            }),
    )
}

fn activity_parameter_one_direction(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
) -> Result<bool, QueryError> {
    // One known edge in each direction (including a self-loop) proves failure.
    // Unresolved endpoints cannot erase witnesses or invent an absent direction.
    require_all([
        activity_edge_exists(references, node, "target"),
        activity_edge_exists(references, node, "source"),
    ])
    .map(|both| !both)
}

fn activity_parameter_membership(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let activity = references
        .model()
        .owner(node)
        .map_err(|error| QueryError::unsupported(node.id(), error))?
        .filter(|owner| owner.conforms_to("Activity"))
        .ok_or_else(|| {
            QueryError::unsupported(
                node.id(),
                "direct canonical Activity containment is unresolved",
            )
        })?;
    if !classifiers::contributes_to_owned_union(activity, node, "Activity-node")
        .map_err(|error| QueryError::unsupported(node.id(), error.to_string()))?
    {
        return Err(QueryError::unsupported(
            node.id(),
            "canonical Activity node role is unresolved",
        ));
    }
    if node.properties().contains_key("activity")
        && optional_activity_reference(references, node, "ActivityNode", "activity")?.as_deref()
            != Some(activity.id())
    {
        return Err(QueryError::unsupported(
            node.id(),
            "authored activity contradicts canonical containment",
        ));
    }
    if node
        .properties()
        .get("inStructuredNode")
        .is_some_and(|value| !value.is_null())
    {
        return Err(QueryError::unsupported(
            node.id(),
            "structured-node containment is not resolved to the direct Activity",
        ));
    }
    let identifier =
        optional_activity_reference(references, node, "ActivityParameterNode", "parameter")?
            .ok_or_else(|| {
                QueryError::unsupported(
                    node.id(),
                    "ActivityParameterNode parameter is not represented",
                )
            })?;
    let parameter = references
        .element(&identifier)
        .map_err(|error| QueryError::unsupported(node.id(), error))?;
    let owner = references
        .model()
        .owner(parameter)
        .map_err(|error| QueryError::unsupported(parameter.id(), error))?
        .filter(|owner| owner.conforms_to("Behavior") || owner.conforms_to("BehavioralFeature"))
        .ok_or_else(|| {
            QueryError::unsupported(
                parameter.id(),
                "canonical parameter containment is unresolved",
            )
        })?;
    let evidence = |owner_id| {
        operations::owned_parameter_evidence(references, owner_id)
            .map_err(|error| QueryError::unsupported(node.id(), error.to_string()))
    };
    let (parameters, completeness) = evidence(activity.id())?;
    // A known membership witness survives unrelated incomplete entries, but
    // the shared query rejects contradictory local membership/order first.
    if parameters
        .iter()
        .any(|candidate| candidate.id == parameter.id())
    {
        return Ok(true);
    }
    if owner.id() == activity.id() {
        return Err(QueryError::unsupported(
            parameter.id(),
            "canonical ownedParameter role is unresolved",
        ));
    }
    completeness.map_err(|error| QueryError::unsupported(node.id(), error.to_string()))?;
    let (other_parameters, _) = evidence(owner.id())?;
    if !other_parameters
        .iter()
        .any(|candidate| candidate.id == parameter.id())
    {
        return Err(QueryError::unsupported(
            parameter.id(),
            "other owner's parameter membership is unresolved",
        ));
    }
    Ok(false)
}

fn activity_parameter_same_type(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let identifier =
        optional_activity_reference(references, node, "ActivityParameterNode", "parameter")?
            .ok_or_else(|| {
                QueryError::unsupported(
                    node.id(),
                    "ActivityParameterNode parameter is not represented",
                )
            })?;
    let parameter = references
        .element(&identifier)
        .map_err(|error| QueryError::unsupported(node.id(), error))?;
    // TypedElement::type is optional. Absent and explicit null both denote
    // no type; resolved types compare identity, never subtype compatibility.
    let node_type = optional_activity_reference(references, node, "TypedElement", "type")?;
    let parameter_type =
        optional_activity_reference(references, parameter, "TypedElement", "type")?;
    Ok(node_type == parameter_type)
}

fn activity_parameter_direction(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
    flowing_role: &str,
    empty_role: &str,
    allowed: &[&str],
) -> Result<bool, QueryError> {
    let guard = require_all([
        activity_edge_exists(references, node, flowing_role),
        activity_edge_exists(references, node, empty_role).map(|exists| !exists),
    ]);
    let direction = (|| {
        let identifier =
            optional_activity_reference(references, node, "ActivityParameterNode", "parameter")?
                .ok_or_else(|| {
                    QueryError::unsupported(
                        node.id(),
                        "ActivityParameterNode parameter is not represented",
                    )
                })?;
        let parameter = references
            .element(&identifier)
            .map_err(|error| QueryError::unsupported(node.id(), error))?;
        let value = schema::read_effective_scalar(parameter, "direction")
            .map_err(|error| QueryError::unsupported(parameter.id(), error))?;
        Ok(value
            .as_str()
            .is_some_and(|direction| allowed.contains(&direction)))
    })();
    // Do not gate on an unrelated invalid parameter when the guard is false.
    // Likewise an allowed direction proves the implication with unknown incidence.
    accept_any([guard.map(|applies| !applies), direction])
}

fn uniform_activity_edges(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
) -> Result<bool, QueryError> {
    uniform_activity_edges_excluding(references, node, Ok(None))
}

fn optional_activity_reference(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
    metaclass: &str,
    property: &str,
) -> Result<Option<String>, QueryError> {
    let Some(value) = node
        .properties()
        .get(property)
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    let descriptor = schema::uml_property(metaclass, property)
        .map_err(|error| QueryError::unsupported(node.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(
                node.id(),
                format!("{metaclass}::{property} contract is unavailable"),
            )
        })?;
    match property_contracts::check_value(references, node, descriptor, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) | PropertyOutcome::Unverified(message) => {
            return Err(QueryError::unsupported(node.id(), message));
        }
    }
    let identifier = reference_id(value)
        .map_err(|message| QueryError::unsupported(node.id(), message))?
        .ok_or_else(|| {
            QueryError::unsupported(
                node.id(),
                format!("{metaclass}::{property} identity is unresolved"),
            )
        })?;
    references
        .element(identifier)
        .map_err(|message| QueryError::unsupported(node.id(), message))?;
    Ok(Some(identifier.to_owned()))
}

fn uniform_activity_edges_excluding(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
    exclusion: Result<Option<String>, QueryError>,
) -> Result<bool, QueryError> {
    let mut controls = Vec::new();
    let mut objects = Vec::new();
    let candidates = activity_edge_candidates(references);
    for edge in candidates {
        // Exclusion is by identity, even for outgoing edges or self-loops.
        // Membership of decisionInputFlow in incoming is a separate constraint.
        if matches!(&exclusion, Ok(Some(identifier)) if identifier == edge.id()) {
            continue;
        }
        // The union contains each edge once, even for a self-loop. Either known
        // endpoint establishes incidence despite an unresolved opposite end.
        let incidence = accept_any([
            activity_endpoint(references, edge, "source")
                .map(|endpoint| endpoint.id() == node.id()),
            activity_endpoint(references, edge, "target")
                .map(|endpoint| endpoint.id() == node.id()),
        ]);
        let incidence = require_all([
            incidence,
            exclusion.as_ref().map(|_| true).map_err(Clone::clone),
        ]);
        // An unknown exclusion can only remove edges. A homogeneous union
        // remains homogeneous, but a mixed union need not remain mixed.
        match incidence {
            Ok(false) => continue,
            Ok(true) if edge.metaclass().is_some() => {
                controls.push(Ok(edge.conforms_to("ControlFlow")));
                objects.push(Ok(edge.conforms_to("ObjectFlow")));
            }
            _ => {
                let unresolved = || {
                    QueryError::unsupported(
                        node.id(),
                        format!(
                            "ActivityEdge {} has unresolved incidence or metaclass",
                            edge.id(),
                        ),
                    )
                };
                // A possibly incident edge already known to be a ControlFlow
                // cannot invalidate an all-ControlFlow union (and vice versa).
                controls.push(if edge.conforms_to("ControlFlow") {
                    Ok(true)
                } else {
                    Err(unresolved())
                });
                objects.push(if edge.conforms_to("ObjectFlow") {
                    Ok(true)
                } else {
                    Err(unresolved())
                });
            }
        }
    }
    accept_any([require_all(controls), require_all(objects)])
}

fn join_edge_families(
    references: &ReferenceIndex<'_>,
    node: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let mut incoming_objects = Vec::new();
    let mut outgoing_objects = Vec::new();
    let mut outgoing_controls = Vec::new();
    for edge in activity_edge_candidates(references) {
        let incoming = activity_endpoint(references, edge, "target")
            .map(|endpoint| endpoint.id() == node.id());
        let outgoing = activity_endpoint(references, edge, "source")
            .map(|endpoint| endpoint.id() == node.id());
        let family = |metaclass: &str| {
            if edge.metaclass().is_some() {
                Ok(edge.conforms_to(metaclass))
            } else {
                Err(QueryError::unsupported(
                    node.id(),
                    format!("ActivityEdge {} has unresolved metaclass", edge.id(),),
                ))
            }
        };
        let object = family("ObjectFlow");
        incoming_objects.push(require_all([incoming, object.clone()]));
        outgoing_objects.push(accept_any([outgoing.clone().map(|value| !value), object]));
        outgoing_controls.push(accept_any([
            outgoing.map(|value| !value),
            family("ControlFlow"),
        ]));
    }
    let has_incoming_object = accept_any(incoming_objects);
    let all_outgoing_objects = require_all(outgoing_objects);
    let all_outgoing_controls = require_all(outgoing_controls);
    // If both consequent branches hold (notably an empty outgoing set), unknown
    // incoming incidence cannot change the result. Cardinality is checked alone.
    accept_any([
        require_all([has_incoming_object.clone(), all_outgoing_objects.clone()]),
        require_all([
            has_incoming_object.map(|value| !value),
            all_outgoing_controls.clone(),
        ]),
        require_all([all_outgoing_objects, all_outgoing_controls]),
    ])
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for node in references.model().elements() {
        let mut checks = Vec::new();
        if node.conforms_to("Pin") {
            checks.push((
                schema::read_effective_boolean(node, "isUnique")
                    .map(|unique| !unique)
                    .map_err(|error| QueryError::unsupported(node.id(), error)),
                "UML_PIN_NOT_UNIQUE",
                "MDE_PIN_NOT_UNIQUE_UNVERIFIED",
                "Pin-not_unique",
                "a Pin must have isUnique=false",
            ));
            checks.push((
                accept_any([
                    schema::read_effective_boolean(node, "isControl")
                        .map(|control| !control)
                        .map_err(|error| QueryError::unsupported(node.id(), error)),
                    schema::read_effective_boolean(node, "isControlType")
                        .map_err(|error| QueryError::unsupported(node.id(), error)),
                ]),
                "UML_PIN_CONTROL_PINS",
                "MDE_PIN_CONTROL_PINS_UNVERIFIED",
                "Pin-control_pins",
                "a Pin with isControl=true must also have isControlType=true",
            ));
        }
        if node.conforms_to("ObjectNode") {
            checks.push((
                object_node_selection(references, node),
                "UML_OBJECT_NODE_SELECTION_BEHAVIOR",
                "MDE_OBJECT_NODE_SELECTION_BEHAVIOR_UNVERIFIED",
                "ObjectNode-selection_behavior",
                "an ObjectNode has a selection Behavior if and only if its ordering is ordered",
            ));
            checks.push((
                object_node_edges(references, node),
                "UML_OBJECT_NODE_OBJECT_FLOW_EDGES",
                "MDE_OBJECT_NODE_OBJECT_FLOW_EDGES_UNVERIFIED",
                "ObjectNode-object_flow_edges",
                "an ObjectNode with isControlType=false must have only ObjectFlow incident edges",
            ));
        }
        if node.conforms_to("ObjectFlow") {
            checks.push((
                object_flow_behavior_parameters(references, node, "selection"),
                "UML_OBJECT_FLOW_INPUT_AND_OUTPUT_PARAMETER",
                "MDE_OBJECT_FLOW_INPUT_AND_OUTPUT_PARAMETER_UNVERIFIED",
                "ObjectFlow-input_and_output_parameter",
                "an ObjectFlow selection Behavior must have one input Parameter with non-unique multiplicity 0..* and one output Parameter",
            ));
            checks.push((
                object_flow_behavior_parameters(references, node, "transformation"),
                "UML_OBJECT_FLOW_TRANSFORMATION_BEHAVIOR",
                "MDE_OBJECT_FLOW_TRANSFORMATION_BEHAVIOR_UNVERIFIED",
                "ObjectFlow-transformation_behavior",
                "an ObjectFlow transformation Behavior must have exactly one input and one output Parameter",
            ));
            checks.push((
                accept_any(["source", "target"].map(|role| {
                    activity_endpoint(references, node, role)
                        .map(|endpoint| endpoint.conforms_to("ExecutableNode"))
                }))
                .map(|executable| !executable),
                "UML_OBJECT_FLOW_NO_EXECUTABLE_NODES",
                "MDE_OBJECT_FLOW_NO_EXECUTABLE_NODES_UNVERIFIED",
                "ObjectFlow-no_executable_nodes",
                "an ObjectFlow must not have an ExecutableNode at either endpoint",
            ));
            checks.push((
                accept_any([
                    activity_reference_present(references, node, "ObjectFlow", "selection")
                        .map(|present| !present),
                    activity_endpoint(references, node, "source")
                        .map(|source| source.conforms_to("ObjectNode")),
                ]),
                "UML_OBJECT_FLOW_SELECTION_BEHAVIOR",
                "MDE_OBJECT_FLOW_SELECTION_BEHAVIOR_UNVERIFIED",
                "ObjectFlow-selection_behavior",
                "an ObjectFlow with a selection Behavior must have an ObjectNode source",
            ));
            // Defaults come from the normative property descriptors. Malformed
            // scalars have their own property diagnostics, not a fabricated
            // violation of this conjunction. A known false still disproves it.
            checks.push((
                require_all(["isMulticast", "isMultireceive"].map(|property| {
                    schema::read_effective_boolean(node, property)
                        .map_err(|error| QueryError::unsupported(node.id(), error))
                }))
                .map(|both| !both),
                "UML_OBJECT_FLOW_IS_MULTICAST_OR_IS_MULTIRECEIVE",
                "MDE_OBJECT_FLOW_IS_MULTICAST_OR_IS_MULTIRECEIVE_UNVERIFIED",
                "ObjectFlow-is_multicast_or_is_multireceive",
                "ObjectFlow isMulticast and isMultireceive cannot both be true",
            ));
        }
        if node.conforms_to("ControlFlow") {
            checks.push((
                require_all([
                    control_flow_endpoint(references, node, "source"),
                    control_flow_endpoint(references, node, "target"),
                ]),
                "UML_CONTROL_FLOW_OBJECT_NODES",
                "MDE_CONTROL_FLOW_OBJECT_NODES_UNVERIFIED",
                "ControlFlow-object_nodes",
                "ObjectNode endpoints of a ControlFlow must have isControlType=true",
            ));
        }
        if node.conforms_to("ActivityParameterNode") {
            checks.push((
                activity_parameter_membership(references, node),
                "UML_ACTIVITY_PARAMETER_NODE_HAS_PARAMETERS",
                "MDE_ACTIVITY_PARAMETER_NODE_HAS_PARAMETERS_UNVERIFIED",
                "ActivityParameterNode-has_parameters",
                "an ActivityParameterNode must reference a Parameter owned by its Activity",
            ));
            checks.push((
                activity_parameter_same_type(references, node),
                "UML_ACTIVITY_PARAMETER_NODE_SAME_TYPE",
                "MDE_ACTIVITY_PARAMETER_NODE_SAME_TYPE_UNVERIFIED",
                "ActivityParameterNode-same_type",
                "an ActivityParameterNode must have the same type as its Parameter",
            ));
            checks.push((
                activity_parameter_direction(references, node, "source", "target", &["in", "inout"]),
                "UML_ACTIVITY_PARAMETER_NODE_NO_INCOMING_EDGES",
                "MDE_ACTIVITY_PARAMETER_NODE_NO_INCOMING_EDGES_UNVERIFIED",
                "ActivityParameterNode-no_incoming_edges",
                "an ActivityParameterNode with only outgoing edges requires an in or inout Parameter",
            ));
            checks.push((
                activity_parameter_direction(references, node, "target", "source", &["out", "inout", "return"]),
                "UML_ACTIVITY_PARAMETER_NODE_NO_OUTGOING_EDGES",
                "MDE_ACTIVITY_PARAMETER_NODE_NO_OUTGOING_EDGES_UNVERIFIED",
                "ActivityParameterNode-no_outgoing_edges",
                "an ActivityParameterNode with only incoming edges requires an out, inout or return Parameter",
            ));
            checks.push((
                activity_parameter_one_direction(references, node),
                "UML_ACTIVITY_PARAMETER_NODE_NO_EDGES",
                "MDE_ACTIVITY_PARAMETER_NODE_NO_EDGES_UNVERIFIED",
                "ActivityParameterNode-no_edges",
                "an ActivityParameterNode must not have both incoming and outgoing ActivityEdges",
            ));
        }
        if node.conforms_to("InitialNode") {
            checks.push((
                incident_edges(references, node, "target", 0, 0, false),
                "UML_INITIAL_NODE_NO_INCOMING_EDGES",
                "MDE_INITIAL_NODE_NO_INCOMING_EDGES_UNVERIFIED",
                "InitialNode-no_incoming_edges",
                "an InitialNode cannot have incoming ActivityEdges",
            ));
            checks.push((
                incident_edges(references, node, "source", 0, usize::MAX, true),
                "UML_INITIAL_NODE_CONTROL_EDGES",
                "MDE_INITIAL_NODE_CONTROL_EDGES_UNVERIFIED",
                "InitialNode-control_edges",
                "outgoing InitialNode edges must be ControlFlows",
            ));
        }
        if node.conforms_to("FinalNode") {
            checks.push((
                incident_edges(references, node, "source", 0, 0, false),
                "UML_FINAL_NODE_NO_OUTGOING_EDGES",
                "MDE_FINAL_NODE_NO_OUTGOING_EDGES_UNVERIFIED",
                "FinalNode-no_outgoing_edges",
                "a FinalNode cannot have outgoing ActivityEdges",
            ));
        }
        if node.conforms_to("ForkNode") {
            checks.push((
                uniform_activity_edges(references, node),
                "UML_FORK_NODE_EDGES",
                "MDE_FORK_NODE_EDGES_UNVERIFIED",
                "ForkNode-edges",
                "incoming and outgoing ForkNode edges must be all ControlFlows or all ObjectFlows",
            ));
            checks.push((
                incident_edges(references, node, "target", 1, 1, false),
                "UML_FORK_NODE_ONE_INCOMING_EDGE",
                "MDE_FORK_NODE_ONE_INCOMING_EDGE_UNVERIFIED",
                "ForkNode-one_incoming_edge",
                "a ForkNode requires exactly one incoming ActivityEdge",
            ));
        }
        if node.conforms_to("JoinNode") {
            checks.push((
                join_edge_families(references, node),
                "UML_JOIN_NODE_INCOMING_OBJECT_FLOW",
                "MDE_JOIN_NODE_INCOMING_OBJECT_FLOW_UNVERIFIED",
                "JoinNode-incoming_object_flow",
                "outgoing JoinNode edges must be ObjectFlows when an incoming ObjectFlow exists, and ControlFlows otherwise",
            ));
            checks.push((
                incident_edges(references, node, "source", 1, 1, false),
                "UML_JOIN_NODE_ONE_OUTGOING_EDGE",
                "MDE_JOIN_NODE_ONE_OUTGOING_EDGE_UNVERIFIED",
                "JoinNode-one_outgoing_edge",
                "a JoinNode requires exactly one outgoing ActivityEdge",
            ));
        }
        if node.conforms_to("MergeNode") {
            checks.push((
                uniform_activity_edges(references, node),
                "UML_MERGE_NODE_EDGES",
                "MDE_MERGE_NODE_EDGES_UNVERIFIED",
                "MergeNode-edges",
                "incoming and outgoing MergeNode edges must be all ControlFlows or all ObjectFlows",
            ));
            checks.push((
                incident_edges(references, node, "source", 1, 1, false),
                "UML_MERGE_NODE_ONE_OUTGOING_EDGE",
                "MDE_MERGE_NODE_ONE_OUTGOING_EDGE_UNVERIFIED",
                "MergeNode-one_outgoing_edge",
                "a MergeNode requires exactly one outgoing ActivityEdge",
            ));
        }
        if node.conforms_to("DecisionNode") {
            checks.push((
                decision_input_parameters(references, node, DecisionInputRule::ZeroInputs),
                "UML_DECISION_NODE_ZERO_INPUT_PARAMETERS",
                "MDE_DECISION_NODE_ZERO_INPUT_PARAMETERS_UNVERIFIED",
                "DecisionNode-zero_input_parameters",
                "a DecisionNode with incoming ControlFlow and no decisionInputFlow must have no input Parameters on its decisionInput Behavior",
            ));
            checks.push((
                decision_input_parameters(references, node, DecisionInputRule::ControlOneInput),
                "UML_DECISION_NODE_INCOMING_CONTROL_ONE_INPUT_PARAMETER",
                "MDE_DECISION_NODE_INCOMING_CONTROL_ONE_INPUT_PARAMETER_UNVERIFIED",
                "DecisionNode-incoming_control_one_input_parameter",
                "a DecisionNode with incoming ControlFlow and decisionInputFlow must have one input Parameter on its decisionInput Behavior",
            ));
            checks.push((
                decision_input_parameters(references, node, DecisionInputRule::ObjectOneInput),
                "UML_DECISION_NODE_INCOMING_OBJECT_ONE_INPUT_PARAMETER",
                "MDE_DECISION_NODE_INCOMING_OBJECT_ONE_INPUT_PARAMETER_UNVERIFIED",
                "DecisionNode-incoming_object_one_input_parameter",
                "a DecisionNode whose incoming edges are all ObjectFlows and which has no decisionInputFlow must have one input Parameter on its decisionInput Behavior",
            ));
            checks.push((
                decision_input_parameters(references, node, DecisionInputRule::TwoInputs),
                "UML_DECISION_NODE_TWO_INPUT_PARAMETERS",
                "MDE_DECISION_NODE_TWO_INPUT_PARAMETERS_UNVERIFIED",
                "DecisionNode-two_input_parameters",
                "a DecisionNode whose incoming edges are all ObjectFlows and which has decisionInputFlow must have two input Parameters on its decisionInput Behavior",
            ));
            checks.push((
                uniform_activity_edges_excluding(references, node, optional_activity_reference(references, node, "DecisionNode", "decisionInputFlow")),
                "UML_DECISION_NODE_EDGES",
                "MDE_DECISION_NODE_EDGES_UNVERIFIED",
                "DecisionNode-edges",
                "DecisionNode edges other than decisionInputFlow must be all ControlFlows or all ObjectFlows",
            ));
            checks.push((require_all([
                incident_edges(references, node, "target", 1, 2, false),
                incident_edges(references, node, "source", 1, usize::MAX, false),
            ]), "UML_DECISION_NODE_INCOMING_OUTGOING_EDGES", "MDE_DECISION_NODE_INCOMING_OUTGOING_EDGES_UNVERIFIED",
                "DecisionNode-incoming_outgoing_edges", "a DecisionNode requires one or two incoming and at least one outgoing ActivityEdge"));
        }
        for (result, code, unknown, constraint, violation) in checks {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (StructuralSeverity::Error, code, violation.to_owned()),
                Err(QueryError::Invalid { message, .. }) => {
                    (StructuralSeverity::Error, code, message)
                }
                Err(QueryError::Unsupported { message, .. }) => {
                    (StructuralSeverity::Warning, unknown, message)
                }
            };
            issues.push(StructuralDiagnostic {
                severity,
                code,
                subject_id: node.id().to_owned(),
                message: format!("{constraint}: {message}"),
            });
        }
    }
    issues
}
