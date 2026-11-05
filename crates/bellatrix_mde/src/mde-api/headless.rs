use crate::api_contract::{API_VERSION, decode_model, error_response, invalid_request};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    operation: String,
    from: Option<String>,
    to: Option<String>,
    relation: Option<String>,
}

/// Produce the existing headless Check, Inspect or Analyze report.
/// Input is `{api_version:1,model,operation,from?,to?,relation?}`. Operation is
/// `check`, `inspect` or `analyze`; traversal remains in the shared model.
/// Returns `{api_version:1,report}`. Graph integrity is required, not a claim of
/// complete UML validity. No filesystem access, mutation or retained state.
/// Errors are versioned document errors, `GRAPH_INTEGRITY` or `GRAPH_QUERY_FAILED`.
pub fn report_model_document(input_json: &str) -> Result<String, String> {
    let request: Request = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    crate::Graph::new(&model)
        .validate()
        .map_err(|error| error_response("GRAPH_INTEGRITY", error.to_string()))?;
    let report = match request.operation.as_str() {
        "check" => format!(
            "Graph integrity verified: {} elements, {} relations, {} diagrams (UML semantics not evaluated)",
            model.elements.len(),
            model.relations.len(),
            model.diagrams.len()
        ),
        "inspect" => {
            let mut lines = vec![format!("Project: {} ({})", model.name, model.id)];
            for element in &model.elements {
                lines.push(format!(
                    "{} | {} | {}",
                    element.id, element.kind, element.name
                ));
            }
            for relation in &model.relations {
                lines.push(format!(
                    "{} --[{}]--> {}",
                    relation.from, relation.kind, relation.to
                ));
            }
            lines.join("\n")
        }
        "analyze" => graph_report(
            &model,
            request.from.as_deref(),
            request.to.as_deref(),
            request.relation.as_deref(),
        )
        .map_err(|error| error_response("GRAPH_QUERY_FAILED", error))?,
        _ => {
            return Err(error_response(
                "INVALID_REQUEST",
                "operation must be check, inspect or analyze",
            ));
        }
    };
    Ok(serde_json::json!({"api_version": API_VERSION, "report": report}).to_string())
}

use crate::{GraphIndex, Project};

fn graph_report(
    project: &Project,
    from: Option<&str>,
    to: Option<&str>,
    relation: Option<&str>,
) -> Result<String, String> {
    let index = GraphIndex::new(project);
    let mut result = vec![format!(
        "Graph: {} elements, {} directed relations",
        index.node_count(),
        index.edge_count()
    )];
    for (kind, count) in index.kinds() {
        result.push(format!("  {kind}: {count}"));
        if relation.is_none_or(|filter| filter == kind.as_str()) {
            for group in index.cyclic_components(&kind) {
                result.push(format!("  cycle [{kind}]: {}", group.join(" -> ")));
            }
        }
    }
    if let Some(from) = from {
        let outgoing = index
            .outgoing(from, relation)
            .map_err(|error| error.to_string())?;
        let incoming = index
            .incoming(from, relation)
            .map_err(|error| error.to_string())?;
        result.push(format!("Element: {from}"));
        for edge in outgoing {
            result.push(format!(
                "  outgoing {} --[{}]--> {} ({})",
                edge.from, edge.kind, edge.to, edge.id
            ));
        }
        for edge in incoming {
            result.push(format!(
                "  incoming {} --[{}]--> {} ({})",
                edge.from, edge.kind, edge.to, edge.id
            ));
        }
        if let Some(to) = to {
            match index
                .shortest_path(from, to, relation)
                .map_err(|error| error.to_string())?
            {
                Some(path) if path.is_empty() => {
                    result.push(format!("Path: {from} (same element)"))
                }
                Some(path) => {
                    result.push(format!(
                        "Shortest directed path: {from} -> {to} ({} edges)",
                        path.len()
                    ));
                    for edge in path {
                        result.push(format!(
                            "  {} --[{}]--> {} ({})",
                            edge.from, edge.kind, edge.to, edge.id
                        ));
                    }
                }
                None => result.push(format!("No directed path: {from} -> {to}")),
            }
        }
    }
    Ok(result.join("\n"))
}
