use crate::{Graph, GraphIndex};
use crate::api_contract::{API_VERSION, decode_model, error_response, invalid_request};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReachabilityRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    roots: Vec<String>,
    relation_kinds: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    from: String,
    to: String,
    relation_kind: Option<String>,
}

/// Return outgoing graph reachability as versioned JSON containing sorted `node_ids`.
///
/// Input fields: `api_version: 1`, `model`, `roots` (IDs), `relation_kinds` (strings).
/// Roots are included, duplicates collapse, and an empty kind list traverses no
/// edges. Empty roots yield an empty result. Unknown roots are errors, never ignored.
/// Graph integrity is checked before querying; this is not UML conformance validation.
/// Node IDs include semantic UML relationships represented as graph vertices.
///
/// # Errors
/// Returns versioned JSON errors for invalid requests, unsupported API versions,
/// invalid model documents, `GRAPH_INTEGRITY`, or `UNKNOWN_NODE`.
///
/// # Effects and determinism
/// No external effects, clock, sink or callbacks. Does not mutate input or retain
/// state. Identical input yields identical output independently of preceding calls.
pub fn reachable_graph_nodes(input_json: &str) -> Result<String, String> {
    let request: ReachabilityRequest = serde_json::from_str(input_json)
        .map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    Graph::new(&model).validate_integrity()
        .map_err(|error| error_response("GRAPH_INTEGRITY", error.to_string()))?;
    let index = GraphIndex::new(&model);
    for root in &request.roots {
        if !index.contains(root) {
            return Err(error_response("UNKNOWN_NODE", format!("unknown graph element: {root}")));
        }
    }
    let roots = request.roots.iter().map(String::as_str).collect::<Vec<_>>();
    let kinds = request.relation_kinds.iter().map(String::as_str).collect::<Vec<_>>();
    let mut node_ids = index.reachable(&roots, &kinds).into_iter().collect::<Vec<_>>();
    node_ids.sort();
    Ok(serde_json::json!({"api_version": API_VERSION, "node_ids": node_ids}).to_string())
}

/// Return a shortest directed path as versioned JSON containing `relation_ids`.
///
/// Input fields: `api_version: 1`, `model`, `from`, `to`, and optional
/// `relation_kind` (string or null; absent/null means all kinds). Output is an
/// ordered array of relation IDs, null when no path exists, or [] for identical
/// existing endpoints. Ties follow the existing GraphIndex ordering by next node
/// ID, relation kind and relation ID. Graph integrity is required, not full UML
/// conformance. Unknown kinds simply match no edges.
///
/// # Errors
/// Returns versioned JSON errors for invalid requests, unsupported API versions,
/// invalid model documents, `GRAPH_INTEGRITY`, or `UNKNOWN_NODE`.
///
/// # Effects and determinism
/// No external effects, clock, sink or callbacks. Does not mutate input or retain
/// state. Identical input yields identical output independently of preceding calls.
pub fn shortest_graph_path(input_json: &str) -> Result<String, String> {
    let request: PathRequest = serde_json::from_str(input_json)
        .map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    Graph::new(&model).validate_integrity()
        .map_err(|error| error_response("GRAPH_INTEGRITY", error.to_string()))?;
    let index = GraphIndex::new(&model);
    let relation_ids = index.shortest_path(&request.from, &request.to, request.relation_kind.as_deref())
        .map_err(|error| error_response("UNKNOWN_NODE", error.to_string()))?
        .map(|relations| relations.into_iter().map(|relation| relation.id.as_str()).collect::<Vec<_>>());
    Ok(serde_json::json!({"api_version": API_VERSION, "relation_ids": relation_ids}).to_string())
}
