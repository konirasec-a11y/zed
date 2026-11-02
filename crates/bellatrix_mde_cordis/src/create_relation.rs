//! Semantic relationships own endpoint identities; notation supplies the endpoint geometry.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::{Geometry, ModelingSection, PathSegment, Point, Relation, Style, View};
use bellatrix_mde_modeling::{Graph, ModelingRelationKind, relation_kind_for_creation};
pub(crate) use bellatrix_mde_modeling::{route_relationship as route, reroute_relationships as reroute};
#[cfg(test)]
use bellatrix_mde_modeling::projected_relationship_paths;
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Debug)]
pub struct ModelingCreateRelationRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub kind: ModelingRelationKind,
    pub from: String,
    pub to: String,
    pub expected_revision: u64,
}
pub struct ModelingCreateRelation;
impl Event for ModelingCreateRelation {
    const NAME: &'static str = "bellatrix.mde.create-relation.v1";
    type Args = ModelingCreateRelationRequested;
    type Output = Result<u64, String>;
}

pub(crate) async fn create_relation(context: &Context, projects: &ProjectService,
    request: ModelingCreateRelationRequested) -> Result<u64, String> {
    let document = projects.snapshot(&request.project_id).map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision { return Err("stale relationship creation; refresh project snapshot".into()); }
    let mut section = from_document(&document)?;
    let from = section.elements.iter().find(|element| element.id == request.from).ok_or("source element not found")?;
    let to = section.elements.iter().find(|element| element.id == request.to).ok_or("target element not found")?;
    request.kind.validate_endpoints(&from.kind, &to.kind)?;
    let geometry = route(&section, &request.diagram_id, &request.from, &request.to)?;
    let mut sequence = request.expected_revision.checked_add(1).ok_or("ID overflow")?;
    let (id, view_id) = loop {
        let id = format!("mde-uml-relation-{sequence}");
        let view_id = format!("mde-uml-relation-view-{sequence}");
        if section.elements.iter().all(|element| element.id != id && element.id != view_id)
            && section.relations.iter().all(|relation| relation.id != id && relation.id != view_id)
            && section.diagrams.iter().all(|diagram| diagram.id != id && diagram.id != view_id
                && diagram.views.iter().all(|view| view.id != id && view.id != view_id)) { break (id, view_id); }
        sequence = sequence.checked_add(1).ok_or("ID overflow")?;
    };
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("routing".into(), serde_json::json!("automatic"));
    // Aggregation/composition specialize the *end* of a UML Association.
    // The Toolbox and Class Quick Edit must persist the same model shape.
    let (semantic_kind, aggregation) = relation_kind_for_creation(request.kind);
    if let Some(aggregation) = aggregation {
        properties.insert("end1Aggregation".into(), serde_json::json!(aggregation));
    }
    section.relations.push(Relation { id: id.clone(), kind: semantic_kind.into(),
        from: request.from, to: request.to, properties });
    let diagram = section.diagrams.iter_mut().find(|diagram| diagram.id == request.diagram_id).ok_or("diagram not found")?;
    let z_index = diagram.views.iter().map(|view| view.z_index).max().unwrap_or(0)
        .checked_add(1).ok_or("z-index overflow")?;
    diagram.views.push(View { id: view_id, subject: Some(id), label: None, geometry, style: Style::default(), z_index });
    // Recompute every automatic sibling after the new Relation and View
    // exist. Otherwise all tools keep the original straight-line geometry.
    reroute(&mut section, &request.diagram_id);
    Graph::new(&section.to_model(document.id().as_str(), document.name())).validate().map_err(|error| error.to_string())?;
    let state = serde_json::to_value(section).map_err(|error| error.to_string())?;
    match context.query::<ProjectReplace>(Routing::Unscoped, ProjectReplaceRequested {
        id: request.project_id, namespace: namespace_for(&document).into(), expected_revision: request.expected_revision, state,
    }).await.map_err(|error| error.to_string())? {
        QueryOutcome::Answer(Ok(document)) => Ok(document.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relationship_tool_routes_between_two_visible_class_views() {
        let model = serde_json::from_str(
            include_str!("../../bellatrix_mde_model/fixtures/todo.json")
        ).unwrap();
        let section = ModelingSection::from_model(&model);
        let diagram_id = section.diagrams[0].id.clone();
        let geometry = route(&section, &diagram_id, "type-list", "type-task").unwrap();
        let Geometry::Path { start, segments } = geometry else {
            panic!("a UML relationship must be represented as a Path");
        };
        assert!(!segments.is_empty());
        let end = match segments.last().unwrap() {
            PathSegment::Line { to } | PathSegment::Cubic { to, .. } => *to,
        };
        assert!(start.x.is_finite() && start.y.is_finite());
        assert!(end.x.is_finite() && end.y.is_finite());
        assert!(start.x.hypot(start.y) > 0.0);
        assert!(start.x != end.x || start.y != end.y,
            "two different classes must not produce an invisible zero-length edge");
    }

    #[test]
    fn different_relations_between_same_nodes_occupy_different_lanes() {
        let model = serde_json::from_str(
            include_str!("../../bellatrix_mde_model/fixtures/todo.json")
        ).unwrap();
        let mut section = ModelingSection::from_model(&model);
        let diagram_id = section.diagrams[0].id.clone();
        let original = section.diagrams[0].views.iter()
            .find(|view| view.subject.as_deref() == Some("ref-task"))
            .unwrap().geometry.clone();
        for (id, kind) in [("new-dependency", "uml.dependency"),
                           ("new-composition", "uml.composition")] {
            let mut properties = std::collections::BTreeMap::new();
            properties.insert("routing".into(), serde_json::json!("automatic"));
            section.relations.push(Relation {
                id: id.into(), kind: kind.into(),
                from: "type-list".into(), to: "type-task".into(),
                properties,
            });
            let geometry = route(&section, &diagram_id, "type-list", "type-task").unwrap();
            section.diagrams[0].views.push(View {
                id: format!("view-{id}"), subject: Some(id.into()), label: None,
                geometry, style: Style::default(), z_index: 5,
            });
        }
        let before_projection = section.clone();
        let projected = projected_relationship_paths(&section, &diagram_id);
        assert_eq!(section, before_projection,
            "projection must not mutate the persisted project");
        assert_eq!(projected.len(), 3);
        assert_ne!(projected["new-dependency"], projected["new-composition"]);
        assert!(matches!(&projected["ref-task"],
            Geometry::Path { segments, .. } if matches!(segments.first(), Some(PathSegment::Cubic { .. })),
            "legacy dependency retains its cubic bend while attaching to nodes"));
        reroute(&mut section, &diagram_id);
        let relation_view = |id: &str| section.diagrams[0].views.iter()
            .find(|view| view.subject.as_deref() == Some(id))
            .unwrap().geometry.clone();
        let first = relation_view("new-dependency");
        let second = relation_view("new-composition");
        assert_ne!(first, second, "two connections must never paint over the same path");
        let Geometry::Path { segments, .. } = relation_view("ref-task") else {
            panic!("legacy relationship remains a Path")
        };
        assert!(matches!(segments.first(), Some(PathSegment::Cubic { .. })));
        assert!(matches!(original, Geometry::Path { .. }));
        assert!(matches!(first, Geometry::Path { segments, .. }
            if matches!(&segments[..], [PathSegment::Line {..}] | [PathSegment::Cubic {..}])));
        Graph::new(&section.to_model("project-todo", "Task List")).validate().unwrap();
    }

    #[test]
    fn resized_interface_anchors_use_its_actual_circle() {
        let model = serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let mut section = ModelingSection::from_model(&model);
        section.elements.iter_mut().find(|element| element.id == "type-task").unwrap().kind = "rust.trait".into();
        let diagram_id = section.diagrams[0].id.clone();
        let contents = classifier_contents(&section, "type-task");
        for (width, height) in [(120.0, 160.0), (240.0, 280.0)] {
            let view = section.diagrams[0].views.iter_mut().find(|view| view.subject.as_deref() == Some("type-task")).unwrap();
            view.geometry = Geometry::Rect { x: 400.0, y: 100.0, width, height };
            let geometry = route(&section, &diagram_id, "type-list", "type-task").unwrap();
            let Geometry::Path { segments, .. } = geometry else { panic!("expected connector") };
            let PathSegment::Line { to } = segments[0] else { panic!("expected straight connector") };
            let circle = interface_layout(width, height, contents);
            let distance = (to.x - 400.0 - circle.center.x).hypot(to.y - 100.0 - circle.center.y);
            assert!((distance - circle.radius).abs() < 1e-8);
        }
    }
}
