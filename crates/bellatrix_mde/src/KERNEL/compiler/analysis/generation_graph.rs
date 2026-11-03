//! Pure graph analysis for code generation (source-graph -> generation IR).
//!
//! Not a complete target metamodel or DPO/TGG implementation. The source
//! Project owns stable IDs; petgraph is an ephemeral execution index.
//! Edges are dependent -> prerequisite, so the scheduling graph reverses
//! them (prerequisite -> dependent) before topological sorting.
use crate::Project;
use petgraph::{Direction, algo::toposort, graphmap::DiGraphMap};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GenerationReference {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub relation_kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GenerationUnit {
    pub id: String,
    pub kind: String,
    pub name: String,
    /// Direct semantic dependencies; language adapters resolve imports.
    pub requires: Vec<GenerationReference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GenerationModel {
    /// Stable topological order for targets that require compilation order.
    /// Other targets may ignore this ordering.
    pub units: Vec<GenerationUnit>,
}

#[derive(Debug, Error)]
pub enum GenerationGraphError {
    #[error("generation dependency cycle involving {0}")]
    Cycle(String),
}

/// Only explicit compilation/typing dependencies impose generation order.
/// An association can be navigable in both directions without making code
/// generation cyclic. Ownership and filesystem edges are not build edges.
fn ordering_edge(kind: &str) -> bool {
    matches!(kind,
        "depends_on" | "uml.dependency" | "uml.usage" | "uml.generalization" |
        "uml.interface_realization" | "uml.realization" |
        "uml.template_binding" | "build.depends_on"
    )
}

pub fn analyze_generation_graph(
    project: &Project,
    selected: &HashSet<String>,
) -> Result<GenerationModel, GenerationGraphError> {
    // Deterministic vertex creation makes independent-node ordering stable.
    let mut selected_ids = selected.iter().map(String::as_str).collect::<Vec<_>>();
    selected_ids.sort_unstable();
    let mut graph = DiGraphMap::<&str, ()>::new();
    for id in &selected_ids {
        graph.add_node(*id);
    }
    for relation in &project.relations {
        if ordering_edge(&relation.kind)
            && selected.contains(&relation.from)
            && selected.contains(&relation.to)
        {
            // Graph links point from a client to its supplier; generation
            // order points from the supplier to the client.
            graph.add_edge(relation.to.as_str(), relation.from.as_str(), ());
        }
    }
    // petgraph validates cycles; the stable Kahn frontier resolves ties by
    // source ID, independent of HashMap iteration order or execution platform.
    toposort(&graph, None)
        .map_err(|cycle| GenerationGraphError::Cycle(cycle.node_id().to_owned()))?;
    let mut indegree = selected_ids.iter().map(|id|
        (*id, graph.neighbors_directed(*id, Direction::Incoming).count())
    ).collect::<BTreeMap<&str, usize>>();
    let mut ready = indegree.iter().filter_map(|(id, count)|
        (*count == 0).then_some(*id)
    ).collect::<BTreeSet<&str>>();
    let mut ordered = Vec::with_capacity(selected_ids.len());
    while let Some(id) = ready.pop_first() {
        ordered.push(id);
        for dependent in graph.neighbors(id) {
            let count = indegree.get_mut(dependent)
                .expect("every scheduled dependency is a selected vertex");
            *count -= 1;
            if *count == 0 { ready.insert(dependent); }
        }
    }
    let units = ordered.into_iter().filter_map(|id| {
        let element = project.element(id)?;
        let mut requires = project.relations.iter()
            .filter(|r| r.from == id && ordering_edge(&r.kind))
            .filter_map(|relation| {
                let dependency = project.element(&relation.to)?;
                Some(GenerationReference {
                    id: dependency.id.clone(),
                    kind: dependency.kind.clone(),
                    name: dependency.name.clone(),
                    relation_kind: relation.kind.clone(),
                })
            })
            .collect::<Vec<_>>();
        requires.sort_by(|a, b| (&a.id, &a.relation_kind).cmp(&(&b.id, &b.relation_kind)));
        requires.dedup_by(|a, b| a.id == b.id && a.relation_kind == b.relation_kind);
        Some(GenerationUnit {
            id: element.id.clone(),
            kind: element.kind.clone(),
            name: element.name.clone(),
            requires,
        })
    }).collect();
    Ok(GenerationModel { units })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Relation;

    fn fixture() -> Project {
        serde_json::from_str(include_str!("../../../../fixtures/todo.json")).unwrap()
    }
    #[test]
    fn dependency_precedes_client_and_preserves_reference_identity() {
        let project = fixture();
        let selected = project.elements.iter().map(|e| e.id.clone()).collect();
        let model = analyze_generation_graph(&project, &selected).unwrap();
        let position = |id: &str| model.units.iter().position(|unit| unit.id == id).unwrap();
        assert!(position("type-task") < position("type-list"));
        let client = &model.units[position("type-list")];
        assert_eq!(client.requires.len(), 1);
        assert_eq!(client.requires[0].id, "type-task");
        assert_eq!(client.requires[0].relation_kind, "depends_on");
        assert_eq!(model, analyze_generation_graph(&project, &selected).unwrap());
    }
    #[test]
    fn dependency_cycle_is_error_but_association_cycle_is_legal() {
        let mut project = fixture();
        let selected = project.elements.iter().map(|e| e.id.clone()).collect();
        project.relations.push(Relation {
            id: "back-association".into(), kind: "uml.association".into(),
            from: "type-task".into(), to: "type-list".into(), properties: Default::default(),
        });
        assert!(analyze_generation_graph(&project, &selected).is_ok());
        project.relations.push(Relation {
            id: "back-dependency".into(), kind: "depends_on".into(),
            from: "type-task".into(), to: "type-list".into(), properties: Default::default(),
        });
        assert!(matches!(
            analyze_generation_graph(&project, &selected),
            Err(GenerationGraphError::Cycle(_))
        ));
    }
    #[test]
    fn scoped_generation_keeps_external_import_metadata() {
        let project = fixture();
        let selected = HashSet::from(["type-list".to_string()]);
        let model = analyze_generation_graph(&project, &selected).unwrap();
        assert_eq!(model.units.len(), 1);
        assert_eq!(model.units[0].requires[0].id, "type-task");
    }
}
