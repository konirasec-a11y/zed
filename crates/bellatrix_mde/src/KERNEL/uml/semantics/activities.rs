use super::{
    classifiers::QueryError,
    feature_compatibility::{accept_any, require_all},
    property_contracts::{self, PropertyOutcome},
    references::{ReferenceIndex, external_reference_href, reference_id},
    root::ElementRef,
    schema,
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
        if node.conforms_to("ObjectFlow") {
            checks.push((
                accept_any([
                    // A well-formed external reference proves presence without
                    // needing the external Behavior definition. Its unresolved
                    // type still has a separate property-contract diagnostic.
                    if node
                        .properties()
                        .get("selection")
                        .is_some_and(|value| matches!(external_reference_href(value), Ok(Some(_))))
                    {
                        Ok(false)
                    } else {
                        optional_activity_reference(references, node, "ObjectFlow", "selection")
                            .map(|selection| selection.is_none())
                    },
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
