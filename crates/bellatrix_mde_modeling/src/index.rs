//! Ephemeral directed index: persisted Bellatrix IDs never depend on NodeIndex.
//! This is a query accelerator, not a second source of truth.
use bellatrix_mde_model::{Project, Relation};
use petgraph::{
    Direction,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphLookupError {
    #[error("unknown graph element: {0}")]
    Unknown(String),
}

pub struct GraphIndex<'a> {
    graph: DiGraph<&'a str, &'a Relation>,
    ids: HashMap<&'a str, NodeIndex>,
}

impl<'a> GraphIndex<'a> {
    /// Caller must validate the bellatrix-project first when integrity matters.
    pub fn new(project: &'a Project) -> Self {
        let mut graph = DiGraph::new();
        let mut ids = HashMap::new();
        for element in &project.elements {
            ids.insert(element.id.as_str(), graph.add_node(element.id.as_str()));
        }
        for relation in &project.relations {
            if let (Some(&from), Some(&to)) = (
                ids.get(relation.from.as_str()),
                ids.get(relation.to.as_str()),
            ) {
                graph.add_edge(from, to, relation);
            }
        }
        Self { graph, ids }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains_key(id)
    }
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn kinds(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for edge in self.graph.edge_weights() {
            *counts.entry(edge.kind.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub fn outgoing(
        &self,
        id: &str,
        kind: Option<&str>,
    ) -> Result<Vec<&'a Relation>, GraphLookupError> {
        self.links(id, kind, Direction::Outgoing)
    }

    pub fn incoming(
        &self,
        id: &str,
        kind: Option<&str>,
    ) -> Result<Vec<&'a Relation>, GraphLookupError> {
        self.links(id, kind, Direction::Incoming)
    }

    fn links(
        &self,
        id: &str,
        kind: Option<&str>,
        direction: Direction,
    ) -> Result<Vec<&'a Relation>, GraphLookupError> {
        let at = self.node(id)?;
        let mut links: Vec<_> = self
            .graph
            .edges_directed(at, direction)
            .filter(|edge| kind.is_none_or(|k| edge.weight().kind == k))
            .map(|edge| *edge.weight())
            .collect();
        links.sort_by(|a, b| {
            (&a.kind, &a.from, &a.to, &a.id).cmp(&(&b.kind, &b.from, &b.to, &b.id))
        });
        Ok(links)
    }

    /// BFS follows selected directed relations only; it never follows dependencies
    /// unless the caller explicitly asks to do so.
    pub fn reachable(&self, roots: &[&str], kinds: &[&str]) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut pending: VecDeque<_> = roots
            .iter()
            .filter_map(|id| self.ids.get(*id).copied())
            .collect();
        while let Some(at) = pending.pop_front() {
            if !visited.insert(self.graph[at].to_owned()) {
                continue;
            }
            let mut next = self
                .graph
                .edges_directed(at, Direction::Outgoing)
                .filter(|edge| kinds.contains(&edge.weight().kind.as_str()))
                .map(|edge| edge.target())
                .collect::<Vec<_>>();
            next.sort_by_key(|index| self.graph[*index]);
            pending.extend(next);
        }
        visited
    }

    /// Returns the shortest directed path, preserving its original relation IDs.
    pub fn shortest_path(
        &self,
        from: &str,
        to: &str,
        kind: Option<&str>,
    ) -> Result<Option<Vec<&'a Relation>>, GraphLookupError> {
        let start = self.node(from)?;
        let target = self.node(to)?;
        if start == target {
            return Ok(Some(Vec::new()));
        }
        let mut visited = HashSet::from([start]);
        let mut parent: HashMap<NodeIndex, (NodeIndex, &'a Relation)> = HashMap::new();
        let mut queue = VecDeque::from([start]);
        while let Some(at) = queue.pop_front() {
            let mut edges = self
                .graph
                .edges_directed(at, Direction::Outgoing)
                .filter(|edge| kind.is_none_or(|k| edge.weight().kind == k))
                .map(|edge| (edge.target(), *edge.weight()))
                .collect::<Vec<_>>();
            edges.sort_by(|a, b| {
                (self.graph[a.0], &a.1.kind, &a.1.id).cmp(&(self.graph[b.0], &b.1.kind, &b.1.id))
            });
            for (next, relation) in edges {
                if visited.insert(next) {
                    parent.insert(next, (at, relation));
                    if next == target {
                        return Ok(Some(reconstruct(start, target, &parent)));
                    }
                    queue.push_back(next);
                }
            }
        }
        Ok(None)
    }

    /// Tarjan's SCC algorithm is applied to one relation kind, not to the entire
    /// bellatrix-mde-model: a dependency cycle is valid unless a target profile forbids it.
    pub fn cyclic_components(&self, kind: &str) -> Vec<Vec<String>> {
        let mut filtered = DiGraph::<&str, ()>::new();
        for id in self.graph.node_weights() {
            filtered.add_node(*id);
        }
        for edge in self
            .graph
            .edge_references()
            .filter(|e| e.weight().kind == kind)
        {
            filtered.add_edge(edge.source(), edge.target(), ());
        }
        let mut groups = petgraph::algo::tarjan_scc(&filtered)
            .into_iter()
            .filter(|component| {
                component.len() > 1 || filtered.find_edge(component[0], component[0]).is_some()
            })
            .map(|component| {
                let mut ids = component
                    .into_iter()
                    .map(|index| filtered[index].to_owned())
                    .collect::<Vec<_>>();
                ids.sort();
                ids
            })
            .collect::<Vec<_>>();
        groups.sort();
        groups
    }

    fn node(&self, id: &str) -> Result<NodeIndex, GraphLookupError> {
        self.ids
            .get(id)
            .copied()
            .ok_or_else(|| GraphLookupError::Unknown(id.to_owned()))
    }
}

fn reconstruct<'a>(
    start: NodeIndex,
    mut at: NodeIndex,
    parent: &HashMap<NodeIndex, (NodeIndex, &'a Relation)>,
) -> Vec<&'a Relation> {
    let mut path = Vec::new();
    while at != start {
        let (previous, edge) = parent[&at];
        path.push(edge);
        at = previous;
    }
    path.reverse();
    path
}
