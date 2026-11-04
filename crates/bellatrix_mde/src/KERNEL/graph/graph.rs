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
    #[error("view {view} references endpoint view {endpoint} outside diagram {diagram}")]
    MissingViewEndpoint { view: String, endpoint: String, diagram: String },
    #[error("view {view} has an invalid presentation reference {role}: {target}")]
    PresentationReference { view: String, role: String, target: String },
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
    #[error("invalid UML root: {0}")]
    InvalidRoot(String),
    #[error("invalid hierarchy: {0}")]
    InvalidHierarchy(String),
    #[error("invalid geometry in view {0}")]
    Geometry(String),
}

pub struct Graph<'a> { pub project: &'a Project }

impl<'a> Graph<'a> {
    pub fn new(project: &'a Project) -> Self { Self { project } }

    pub fn validate_identifiers(&self) -> Result<(), GraphError> {
        let mut all = HashSet::new();
        for id in std::iter::once(&self.project.id)
            .chain(self.project.elements.iter().map(|element| &element.id))
            .chain(self.project.relations.iter().map(|relation| &relation.id))
            .chain(self.project.diagrams.iter().map(|diagram| &diagram.id))
            .chain(self.project.diagrams.iter().flat_map(|diagram| diagram.all_views().map(|view| &view.id)))
        {
            if id.trim().is_empty() || !all.insert(id.as_str()) {
                return Err(GraphError::Duplicate(id.clone()));
            }
        }
        Ok(())
    }

    pub fn validate_presentation(&self) -> Result<(), GraphError> {
        crate::validate_model_hierarchy(self.project).map_err(GraphError::InvalidHierarchy)?;
        self.validate_identifiers()?;
        let nodes: HashSet<&str> = self.project.elements.iter().map(|element| element.id.as_str()).collect();
        let relations: HashSet<&str> = self.project.relations.iter().map(|relation| relation.id.as_str()).collect();
        for diagram in &self.project.diagrams {
            if !(diagram.width.is_finite() && diagram.height.is_finite() && diagram.width > 0.0 && diagram.height > 0.0) {
                return Err(GraphError::Geometry(diagram.id.clone()));
            }
            validate_view_endpoints(diagram)?;
            validate_presentation_references(diagram, &nodes, &relations, false)?;
            for view in diagram.all_views() {
                if !valid_geometry(&view.geometry) || !valid_style(&view.style)
                    || view.timing.as_ref().is_some_and(|timing| timing.validate().is_err())
                {
                    return Err(GraphError::Geometry(view.id.clone()));
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        self.validate_with_presentation(true)
    }

    // Topological queries must reject ambiguous identities and broken references,
    // but a visual size or style error does not change graph connectivity.
    pub(crate) fn validate_integrity(&self) -> Result<(), GraphError> {
        self.validate_with_presentation(false)
    }

    fn validate_with_presentation(&self, check_presentation: bool) -> Result<(), GraphError> {
        self.validate_identifiers()?;
        let semantic = crate::uml::root::Model::from_project(self.project);
        semantic.root().map_err(GraphError::InvalidRoot)?;
        let nodes: HashSet<&str> = semantic.elements().map(|element| element.id()).collect();
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
            let parent = semantic.element(&relation.from).ok_or_else(|| GraphError::MissingEndpoint { relation: relation.id.clone(), endpoint: relation.from.clone() })?;
            let child = semantic.element(&relation.to).ok_or_else(|| GraphError::MissingEndpoint { relation: relation.id.clone(), endpoint: relation.to.clone() })?;
            if relation.kind == "owns" {
                if !semantic_owners.insert(relation.to.as_str()) {
                    return Err(GraphError::MultipleOwners(relation.to.clone()));
                }
                if parent.kind().starts_with("rust.") && child.kind().starts_with("fs.") {
                    return Err(GraphError::InvalidRelation(relation.id.clone()));
                }
            }
            if relation.kind == "declares" && child.kind().starts_with("rust.") {
                if parent.kind() != "fs.file" { return Err(GraphError::InvalidRelation(relation.id.clone())); }
                if !declarations.insert(relation.to.as_str()) { return Err(GraphError::MultipleDeclarations(relation.to.clone())); }
            }
            if relation.kind == "physical_parent" {
                if parent.kind() != "fs.directory" || !matches!(child.kind(), "fs.directory" | "fs.file" | "cargo.package" | "cargo.workspace") {
                    return Err(GraphError::InvalidPhysicalParent(relation.id.clone()));
                }
                if !physical_parents.insert(relation.to.as_str()) { return Err(GraphError::MultipleParents(relation.to.clone())); }
            }
        }
        for diagram in &self.project.diagrams {
            if check_presentation && !(diagram.width.is_finite() && diagram.height.is_finite() && diagram.width > 0.0 && diagram.height > 0.0) {
                return Err(GraphError::Geometry(diagram.id.clone()));
            }
            if check_presentation {
                validate_view_endpoints(diagram)?;
                validate_presentation_references(diagram, &nodes, &relations, true)?;
            }
            for view in diagram.all_views() {
                if let Some(subject) = &view.subject
                    && !nodes.contains(subject.as_str()) && !relations.contains(subject.as_str())
                {
                    return Err(GraphError::MissingView { view: view.id.clone(), subject: subject.clone() });
                }
                if check_presentation && (!valid_geometry(&view.geometry) || !valid_style(&view.style)
                    || view.timing.as_ref().is_some_and(|timing| timing.validate().is_err()))
                {
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

fn validate_view_endpoints(diagram: &crate::Diagram) -> Result<(), GraphError> {
    let view_ids: HashSet<&str> = diagram.all_views().map(|view| view.id.as_str()).collect();
    for view in diagram.all_views() {
        // Drafts intentionally retain orphaned attachments for repair; their validator
        // reports warnings without treating the drawing as a semantic relationship.
        if view.draft.is_some() {
            continue;
        }
        if let Some(endpoints) = &view.endpoints {
            for endpoint in endpoints.source_view_id.iter().chain(endpoints.target_view_id.iter()) {
                if !view_ids.contains(endpoint.as_str()) {
                    return Err(GraphError::MissingViewEndpoint {
                        view: view.id.clone(),
                        endpoint: endpoint.clone(),
                        diagram: diagram.id.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_presentation_references(
    diagram: &crate::Diagram,
    nodes: &HashSet<&str>,
    relations: &HashSet<&str>,
    check_model_references: bool,
) -> Result<(), GraphError> {
    let views: HashSet<&str> = diagram.all_views().map(|view| view.id.as_str()).collect();
    for view in diagram.all_views() {
        for (role, reference) in &view.presentation.references {
            let Some(reference) = reference.value() else { continue };
            let (target, valid) = match reference {
                crate::PresentationReference::View(id) => (id, views.contains(id.as_str())),
                crate::PresentationReference::Model(id) => (id, !check_model_references || nodes.contains(id.as_str()) || relations.contains(id.as_str())),
                crate::PresentationReference::Resource(id) => (id, view.presentation.resources.contains_key(id)),
            };
            if !valid {
                return Err(GraphError::PresentationReference { view: view.id.clone(), role: role.clone(), target: target.clone() });
            }
        }
        if let Some(endpoints) = &view.presentation.relationship_endpoints {
            for (side, binding) in [("source", &endpoints.source), ("target", &endpoints.target)] {
                let (target, valid) = match &binding.target {
                    crate::EndpointMarkerTarget::ModelEnd { id } => (
                        id,
                        !id.trim().is_empty()
                            && (!check_model_references || nodes.contains(id.as_str()) || relations.contains(id.as_str())),
                    ),
                    crate::EndpointMarkerTarget::View { id } => (
                        id,
                        !id.trim().is_empty() && (!check_model_references || views.contains(id.as_str())),
                    ),
                    crate::EndpointMarkerTarget::RelationshipRole { role } => (
                        role,
                        !role.trim().is_empty() && (!check_model_references || view.subject.is_some() || view.draft.is_some()),
                    ),
                };
                if !valid {
                    return Err(GraphError::PresentationReference {
                        view: view.id.clone(),
                        role: format!("relationship_endpoints.{side}"),
                        target: target.clone(),
                    });
                }
            }
        }
        if let Some(placement) = &view.presentation.label_placement {
            if [&placement.angle, &placement.distance].iter().any(|value| value.value().is_some_and(|number| !number.is_finite())) {
                return Err(GraphError::Geometry(view.id.clone()));
            }
        }
    }
    Ok(())
}

fn valid_geometry(geometry: &crate::Geometry) -> bool {
    use crate::{Geometry, PathSegment};
    let finite = |point: &crate::Point| point.x.is_finite() && point.y.is_finite();
    match geometry {
        Geometry::Layout { bounds } => {
            [&bounds.x, &bounds.y, &bounds.width, &bounds.height].iter()
                .all(|value| value.value().is_none_or(|number| number.is_finite()))
                && [&bounds.width, &bounds.height].iter()
                    .all(|value| value.value().is_none_or(|number| *number >= 0.0))
        }
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
    style.font_weight.is_none_or(|weight| (1..=1000).contains(&weight))
        && bounded(style.stroke_width, 0.0, 100.0) && bounded(style.font_size, 1.0, 400.0)
        && bounded(style.opacity, 0.0, 1.0) && bounded(style.corner_radius, 0.0, 1000.0)
        && style.dash_pattern.iter().all(|value| value.is_finite() && *value >= 0.0)
}
