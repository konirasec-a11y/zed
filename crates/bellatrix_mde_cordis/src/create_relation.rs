//! Semantic relationships own endpoint identities; notation supplies the endpoint geometry.
use crate::{class_diagram::ModelingRelationKind, project_integration::{from_document, namespace_for}};
use bellatrix_mde_model::{Geometry, ModelingSection, PathSegment, Point, Relation, Style, View};
use bellatrix_mde_modeling::{Graph, InterfaceLayout, UmlType, classifier_contents, interface_layout};
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

#[derive(Clone, Copy)]
struct Endpoint {
    center: Point,
    width: f64,
    height: f64,
    circle: Option<InterfaceLayout>,
}
impl Endpoint {
    fn anchor_toward(self, target: Point) -> Point {
        if let Some(circle) = self.circle { return circle.anchor_toward(target); }
        let delta_x = target.x - self.center.x;
        let delta_y = target.y - self.center.y;
        if delta_x.abs() < f64::EPSILON && delta_y.abs() < f64::EPSILON {
            return Point { x: self.center.x + self.width / 2.0, y: self.center.y };
        }
        let horizontal = if delta_x.abs() > f64::EPSILON { self.width / 2.0 / delta_x.abs() } else { f64::MAX };
        let vertical = if delta_y.abs() > f64::EPSILON { self.height / 2.0 / delta_y.abs() } else { f64::MAX };
        let distance = horizontal.min(vertical);
        Point { x: self.center.x + delta_x * distance, y: self.center.y + delta_y * distance }
    }
}

pub(crate) fn route(section: &ModelingSection, diagram_id: &str, from: &str, to: &str) -> Result<Geometry, String> {
    let diagram = section.diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let endpoint = |subject: &str| -> Result<Endpoint, String> {
        let view = diagram.views.iter().find(|view| view.subject.as_deref() == Some(subject))
            .ok_or_else(|| format!("endpoint {subject} has no View in this diagram"))?;
        let (left, top, width, height) = match &view.geometry {
            Geometry::Rect { x, y, width, height } if *width > 0.0 && *height > 0.0 => (*x, *y, *width, *height),
            _ => return Err("relationship endpoint has no valid node View".to_owned()),
        };
        let kind = section.elements.iter().find(|element| element.id == subject)
            .and_then(|element| UmlType::from_kind(&element.kind));
        if kind == Some(UmlType::Interface) {
            let mut layout = interface_layout(width, height, classifier_contents(section, subject));
            layout.center.x += left;
            layout.center.y += top;
            return Ok(Endpoint { center: layout.center, width: layout.radius * 2.0,
                height: layout.radius * 2.0, circle: Some(layout) });
        }
        Ok(Endpoint { center: Point { x: left + width / 2.0, y: top + height / 2.0 }, width, height, circle: None })
    };
    let source = endpoint(from)?;
    let target = endpoint(to)?;
    if from == to {
        let start = source.anchor_toward(Point { x: source.center.x + source.width, y: source.center.y - source.height });
        let end = source.anchor_toward(Point { x: source.center.x + source.width, y: source.center.y + source.height });
        return Ok(Geometry::Path { start, segments: vec![PathSegment::Cubic {
            control1: Point { x: start.x + 62.0, y: start.y - 56.0 },
            control2: Point { x: end.x + 62.0, y: end.y + 56.0 }, to: end,
        }] });
    }
    let same_center = (source.center.x - target.center.x).abs() < f64::EPSILON
        && (source.center.y - target.center.y).abs() < f64::EPSILON;
    let start = source.anchor_toward(if same_center {
        Point { x: source.center.x + source.width, y: source.center.y }
    } else { target.center });
    let to = target.anchor_toward(if same_center {
        Point { x: target.center.x - target.width, y: target.center.y }
    } else { source.center });
    Ok(Geometry::Path { start, segments: vec![PathSegment::Line { to }] })
}

pub(crate) fn reroute(section: &mut ModelingSection, diagram_id: &str) {
    let routes = section.relations.iter()
        .filter(|relation| relation.kind.starts_with("uml.")
            && relation.properties.get("routing").and_then(serde_json::Value::as_str) == Some("automatic"))
        .filter_map(|relation| route(section, diagram_id, &relation.from, &relation.to).ok()
            .map(|geometry| (relation.id.clone(), geometry))).collect::<Vec<_>>();
    if let Some(diagram) = section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id) {
        for (subject, geometry) in routes {
            if let Some(view) = diagram.views.iter_mut().find(|view| view.subject.as_deref() == Some(subject.as_str())) {
                view.geometry = geometry;
            }
        }
    }
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
    section.relations.push(Relation { id: id.clone(), kind: request.kind.model_kind().into(),
        from: request.from, to: request.to, properties });
    let diagram = section.diagrams.iter_mut().find(|diagram| diagram.id == request.diagram_id).ok_or("diagram not found")?;
    let z_index = diagram.views.iter().map(|view| view.z_index).max().unwrap_or(0)
        .checked_add(1).ok_or("z-index overflow")?;
    diagram.views.push(View { id: view_id, subject: Some(id), label: None, geometry, style: Style::default(), z_index });
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
