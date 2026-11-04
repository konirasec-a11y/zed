use super::{
    classifiers::QueryError,
    feature_compatibility::require_all,
    property_contracts::{self, PropertyOutcome},
    references::{ReferenceIndex, reference_id},
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

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for node in references.model().elements() {
        let mut checks = Vec::new();
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
                incident_edges(references, node, "target", 1, 1, false),
                "UML_FORK_NODE_ONE_INCOMING_EDGE",
                "MDE_FORK_NODE_ONE_INCOMING_EDGE_UNVERIFIED",
                "ForkNode-one_incoming_edge",
                "a ForkNode requires exactly one incoming ActivityEdge",
            ));
        }
        if node.conforms_to("JoinNode") {
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
                incident_edges(references, node, "source", 1, 1, false),
                "UML_MERGE_NODE_ONE_OUTGOING_EDGE",
                "MDE_MERGE_NODE_ONE_OUTGOING_EDGE_UNVERIFIED",
                "MergeNode-one_outgoing_edge",
                "a MergeNode requires exactly one outgoing ActivityEdge",
            ));
        }
        if node.conforms_to("DecisionNode") {
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
