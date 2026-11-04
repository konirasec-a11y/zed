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
    #[error("generation dependency {relation} targets {target}, which is not a source element")]
    UnsupportedDependencyTarget { relation: String, target: String },
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
    // The current generation IR describes source Elements, not reified UML
    // relationships. Reject unsupported targets rather than dropping references
    // in the IR projection below, including dependencies outside the selection.
    let mut unsupported = project.relations.iter().filter(|relation|
        selected.contains(&relation.from) && ordering_edge(&relation.kind)
            && project.element(&relation.to).is_none()
    ).collect::<Vec<_>>();
    unsupported.sort_by(|left, right| (&left.id, &left.to).cmp(&(&right.id, &right.to)));
    if let Some(relation) = unsupported.first() {
        return Err(GenerationGraphError::UnsupportedDependencyTarget {
            relation: relation.id.clone(), target: relation.to.clone(),
        });
    }
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
