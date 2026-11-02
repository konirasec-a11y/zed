use crate::{GraphIndex, Project};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("duplicate or empty identifier: {0}")]
    Duplicate(String),
    #[error("relation {relation} has a missing endpoint: {endpoint}")]
    MissingEndpoint { relation: String, endpoint: String },
    #[error("view {view} references unknown subject {subject}")]
    MissingView { view: String, subject: String },
    #[error("invalid physical parent relation {0} (expected directory -> directory/file/package)")]
    InvalidPhysicalParent(String),
    #[error("node {0} has two physical parents")]
    MultipleParents(String),
    #[error("element {0} has more than one semantic owner")]
    MultipleOwners(String),
    #[error("element {0} has more than one declaration file")]
    MultipleDeclarations(String),
    #[error("invalid relationship {0} for the element types")]
    InvalidRelation(String),
    #[error("cycle in {0} hierarchy")]
    Cycle(String),
    #[error("invalid geometry in view {0}")]
    Geometry(String),
}

pub struct Graph<'a> { pub project: &'a Project }

impl<'a> Graph<'a> {
    pub fn new(project: &'a Project) -> Self { Self { project } }

    pub fn validate(&self) -> Result<(), GraphError> {
        let mut all = HashSet::new();
        for id in std::iter::once(&self.project.id)
            .chain(self.project.elements.iter().map(|element| &element.id))
            .chain(self.project.relations.iter().map(|relation| &relation.id))
            .chain(self.project.diagrams.iter().map(|diagram| &diagram.id))
            .chain(self.project.diagrams.iter().flat_map(|diagram| diagram.views.iter().map(|view| &view.id)))
        {
            if id.trim().is_empty() || !all.insert(id.as_str()) {
                return Err(GraphError::Duplicate(id.clone()));
            }
        }
        let nodes: HashSet<&str> = self.project.elements.iter().map(|element| element.id.as_str()).collect();
        let relations: HashSet<&str> = self.project.relations.iter().map(|relation| relation.id.as_str()).collect();
        let mut physical_parents = HashSet::new();
        let mut semantic_owners = HashSet::new();
        let mut declarations = HashSet::new();
        for relation in &self.project.relations {
            for end in [&relation.from, &relation.to] {
                if !nodes.contains(end.as_str()) {
                    return Err(GraphError::MissingEndpoint { relation: relation.id.clone(), endpoint: end.clone() });
                }
            }
            let parent = self.project.element(&relation.from).expect("checked above");
            let child = self.project.element(&relation.to).expect("checked above");
            if relation.kind == "owns" {
                if !semantic_owners.insert(relation.to.as_str()) {
                    return Err(GraphError::MultipleOwners(relation.to.clone()));
                }
                if parent.kind.starts_with("rust.") && child.kind.starts_with("fs.") {
                    return Err(GraphError::InvalidRelation(relation.id.clone()));
                }
            }
            if relation.kind == "declares" && child.kind.starts_with("rust.") {
                if parent.kind != "fs.file" { return Err(GraphError::InvalidRelation(relation.id.clone())); }
                if !declarations.insert(relation.to.as_str()) { return Err(GraphError::MultipleDeclarations(relation.to.clone())); }
            }
            if relation.kind == "physical_parent" {
                if parent.kind != "fs.directory" || !matches!(child.kind.as_str(), "fs.directory" | "fs.file" | "cargo.package" | "cargo.workspace") {
                    return Err(GraphError::InvalidPhysicalParent(relation.id.clone()));
                }
                if !physical_parents.insert(relation.to.as_str()) { return Err(GraphError::MultipleParents(relation.to.clone())); }
            }
        }
        for diagram in &self.project.diagrams {
            if !(diagram.width.is_finite() && diagram.height.is_finite() && diagram.width > 0.0 && diagram.height > 0.0) {
                return Err(GraphError::Geometry(diagram.id.clone()));
            }
            for view in &diagram.views {
                if let Some(subject) = &view.subject
                    && !nodes.contains(subject.as_str()) && !relations.contains(subject.as_str())
                {
                    return Err(GraphError::MissingView { view: view.id.clone(), subject: subject.clone() });
                }
                if !valid_geometry(&view.geometry) || !valid_style(&view.style) {
                    return Err(GraphError::Geometry(view.id.clone()));
                }
            }
        }
        let index = GraphIndex::new(self.project);
        for kind in ["physical_parent", "owns", "declares"] {
            if !index.cyclic_components(kind).is_empty() { return Err(GraphError::Cycle(kind.to_string())); }
        }
        Ok(())
    }
}

fn valid_geometry(geometry: &crate::Geometry) -> bool {
    use crate::{Geometry, PathSegment};
    let finite = |point: &crate::Point| point.x.is_finite() && point.y.is_finite();
    match geometry {
        Geometry::Rect { x, y, width, height } => [*x, *y, *width, *height].iter().all(|number| number.is_finite()) && *width >= 0.0 && *height >= 0.0,
        Geometry::Ellipse { cx, cy, rx, ry } => [*cx, *cy, *rx, *ry].iter().all(|number| number.is_finite()) && *rx >= 0.0 && *ry >= 0.0,
        Geometry::Polygon { points } => points.len() >= 3 && points.iter().all(finite),
        Geometry::Path { start, segments } => finite(start) && segments.iter().all(|segment| match segment {
            PathSegment::Line { to } => finite(to),
            PathSegment::Cubic { control1, control2, to } => finite(control1) && finite(control2) && finite(to),
        }),
    }
}

fn valid_style(style: &crate::Style) -> bool {
    let bounded = |value: Option<f64>, lower: f64, upper: f64| value.is_none_or(|number| number.is_finite() && number >= lower && number <= upper);
    bounded(style.stroke_width, 0.0, 100.0) && bounded(style.font_size, 1.0, 400.0)
        && bounded(style.opacity, 0.0, 1.0) && bounded(style.corner_radius, 0.0, 1000.0)
        && style.dash_pattern.iter().all(|value| value.is_finite() && *value >= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Relation;
    fn fixture() -> Project { serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap() }
    #[test]
    fn validates_todo_fixture() { Graph::new(&fixture()).validate().unwrap(); }
    #[test]
    fn broken_edge_is_not_absent_edge() {
        let mut project = fixture();
        project.relations.push(Relation { id: "broken".into(), kind: "depends_on".into(), from: "type-task".into(), to: "missing".into(), properties: Default::default() });
        assert!(matches!(Graph::new(&project).validate(), Err(GraphError::MissingEndpoint { .. })));
    }
    #[test]
    fn physical_cycle_is_rejected() {
        let mut project = fixture();
        project.relations.push(Relation { id: "cycle".into(), kind: "physical_parent".into(), from: "dir-src".into(), to: "dir-src".into(), properties: Default::default() });
        assert!(matches!(Graph::new(&project).validate(), Err(GraphError::Cycle(_))));
    }
}
