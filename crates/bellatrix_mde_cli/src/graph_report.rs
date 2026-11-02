//! Human-readable diagnostics over the canonical domain graph.
use bellatrix_mde::{Project, GraphIndex};

pub fn report(project: &Project, from: Option<&str>, to: Option<&str>, relation: Option<&str>) -> Result<String, String> {
    let index = GraphIndex::new(project);
    let mut result = vec![format!("Graph: {} elements, {} directed relations", index.node_count(), index.edge_count())];
    for (kind, count) in index.kinds() {
        result.push(format!("  {kind}: {count}"));
        if relation.is_none_or(|filter| filter == kind.as_str()) {
            for group in index.cyclic_components(&kind) { result.push(format!("  cycle [{kind}]: {}", group.join(" -> "))); }
        }
    }
    if let Some(from) = from {
        let outgoing = index.outgoing(from, relation).map_err(|error| error.to_string())?;
        let incoming = index.incoming(from, relation).map_err(|error| error.to_string())?;
        result.push(format!("Element: {from}"));
        for edge in outgoing { result.push(format!("  outgoing {} --[{}]--> {} ({})", edge.from, edge.kind, edge.to, edge.id)); }
        for edge in incoming { result.push(format!("  incoming {} --[{}]--> {} ({})", edge.from, edge.kind, edge.to, edge.id)); }
        if let Some(to) = to {
            match index.shortest_path(from, to, relation).map_err(|error| error.to_string())? {
                Some(path) if path.is_empty() => result.push(format!("Path: {from} (same element)")),
                Some(path) => {
                    result.push(format!("Shortest directed path: {from} -> {to} ({} edges)", path.len()));
                    for edge in path { result.push(format!("  {} --[{}]--> {} ({})", edge.from, edge.kind, edge.to, edge.id)); }
                }
                None => result.push(format!("No directed path: {from} -> {to}")),
            }
        }
    }
    Ok(result.join("\n"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_finds_directed_path_and_preserves_relations() {
        let project: Project = serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
        let response = report(&project, Some("type-list"), Some("fn-add"), Some("owns")).unwrap();
        assert!(response.contains("type-list --[owns]--> fn-add")); assert!(response.contains("Shortest directed path"));
        assert!(report(&project, Some("fn-add"), Some("type-list"), Some("owns")).unwrap().contains("No directed path"));
    }
}
