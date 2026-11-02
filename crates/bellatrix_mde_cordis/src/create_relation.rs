//! Versioned UML relationship creation and automatic route maintenance.
//! Relationships are semantic Relation objects; their geometry belongs to Views.
use crate::{class_diagram::ModelingRelationKind, project_integration::{from_document, namespace_for}};
use bellatrix_mde_model::{Geometry, ModelingSection, PathSegment, Point, Relation, Style, View};
use bellatrix_mde_modeling::Graph;
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

fn rect_center(g: &Geometry) -> Result<(Point, f64, f64), String> {
    match g {
        Geometry::Rect { x, y, width, height } if *width > 0.0 && *height > 0.0 =>
            Ok((Point { x: x + width / 2.0, y: y + height / 2.0 }, *width, *height)),
        _ => Err("relationship endpoint has no valid rectangular View".into()),
    }
}
fn anchor(from: Point, to: Point, width: f64, height: f64) -> Point {
    let (dx,dy) = (to.x - from.x, to.y - from.y);
    let tx = if dx.abs() > 0.00001 { width / 2.0 / dx.abs() } else { f64::MAX };
    let ty = if dy.abs() > 0.00001 { height / 2.0 / dy.abs() } else { f64::MAX };
    let t = tx.min(ty);
    Point { x: from.x + dx * t, y: from.y + dy * t }
}

/// Auto-route straight UML connectors against the actual node boundaries.
pub(crate) fn route(section: &ModelingSection, diagram_id: &str, from: &str, to: &str) -> Result<Geometry, String> {
    let diagram = section.diagrams.iter().find(|d| d.id == diagram_id)
        .ok_or("diagram not found")?;
    let rect = |subject: &str| {
        let view = diagram.views.iter().find(|v| v.subject.as_deref() == Some(subject))
            .ok_or_else(|| format!("endpoint {subject} has no View in this diagram"))?;
        rect_center(&view.geometry)
    };
    let (a, wa, ha) = rect(from)?;
    let (b, wb, hb) = rect(to)?;
    if from == to {
        let start = Point { x:a.x + wa / 2.0, y:a.y - ha / 4.0 };
        let end = Point { x:a.x + wa / 2.0, y:a.y + ha / 4.0 };
        return Ok(Geometry::Path { start, segments: vec![PathSegment::Cubic {
            control1: Point { x:start.x + 62.0, y:start.y - 56.0 },
            control2: Point { x:end.x + 62.0, y:end.y + 56.0 }, to:end,
        }]});
    }
    let start = anchor(a,b,wa,ha);
    let to = anchor(b,a,wb,hb);
    Ok(Geometry::Path { start, segments: vec![PathSegment::Line { to }] })
}

/// Called after move/resize. Preserve manually controlled paths: only
/// relationships explicitly marked `routing: "automatic"` are touched.
pub(crate) fn reroute(section: &mut ModelingSection, diagram_id: &str) {
    let routes = section.relations.iter()
        .filter(|rel| rel.kind.starts_with("uml.")
            && rel.properties.get("routing").and_then(|v| v.as_str()) == Some("automatic"))
        .filter_map(|rel| route(section,diagram_id,&rel.from,&rel.to).ok().map(|g|(rel.id.clone(),g)))
        .collect::<Vec<_>>();
    if let Some(diagram) = section.diagrams.iter_mut().find(|d| d.id == diagram_id) {
        for (subject,geometry) in routes {
            if let Some(view) = diagram.views.iter_mut().find(|v| v.subject.as_deref()==Some(&subject)) {
                view.geometry=geometry;
            }
        }
    }
}

pub(crate) async fn create_relation(
    ctx: &Context, projects: &ProjectService, request: ModelingCreateRelationRequested,
) -> Result<u64, String> {
    let document = projects.snapshot(&request.project_id).map_err(|e|e.to_string())?;
    if document.revision()!=request.expected_revision {
        return Err("stale relationship creation; refresh project snapshot".into());
    }
    let mut section = from_document(&document)?;
    let from = section.elements.iter().find(|e| e.id==request.from)
        .ok_or("source element not found")?;
    let to = section.elements.iter().find(|e| e.id==request.to)
        .ok_or("target element not found")?;
    request.kind.validate_endpoints(&from.kind,&to.kind)?;
    let geometry = route(&section,&request.diagram_id,&request.from,&request.to)?;
    let mut index = request.expected_revision.checked_add(1).ok_or("ID overflow")?;
    let (id,view_id) = loop {
        let id=format!("mde-uml-relation-{index}");
        let view_id=format!("mde-uml-relation-view-{index}");
        if section.elements.iter().all(|e|e.id!=id && e.id!=view_id)
            && section.relations.iter().all(|r|r.id!=id && r.id!=view_id)
            && section.diagrams.iter().all(|d|d.id!=id && d.id!=view_id
                && d.views.iter().all(|v|v.id!=id && v.id!=view_id)) {
            break (id,view_id);
        }
        index=index.checked_add(1).ok_or("ID overflow")?;
    };
    let mut properties=std::collections::BTreeMap::new();
    properties.insert("routing".into(), serde_json::json!("automatic"));
    section.relations.push(Relation {
        id:id.clone(),kind:request.kind.model_kind().into(),
        from:request.from,to:request.to,properties,
    });
    let diagram=section.diagrams.iter_mut().find(|d|d.id==request.diagram_id)
        .ok_or("diagram not found")?;
    let z_index=diagram.views.iter().map(|v|v.z_index).max().unwrap_or(0)
        .checked_add(1).ok_or("z-index overflow")?;
    diagram.views.push(View {
        id:view_id,subject:Some(id),label:None,geometry,
        style:Style::default(),z_index,
    });
    Graph::new(&section.to_model(document.id().as_str(),document.name()))
        .validate().map_err(|e|e.to_string())?;
    let state=serde_json::to_value(section).map_err(|e|e.to_string())?;
    let namespace=namespace_for(&document);
    match ctx.query::<ProjectReplace>(Routing::Unscoped,ProjectReplaceRequested {
        id:request.project_id,namespace:namespace.into(),
        expected_revision:request.expected_revision,state,
    }).await.map_err(|e|e.to_string())? {
        QueryOutcome::Answer(Ok(doc))=>Ok(doc.revision()),
        QueryOutcome::Answer(Err(e))=>Err(e.to_string()),
        QueryOutcome::Miss=>Err("ProjectService unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relationship_uses_existing_view_boundaries() {
        let model=serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let section=ModelingSection::from_model(&model);
        let diagram_id=section.diagrams.first().unwrap().id.clone();
        let geometry=route(&section,&diagram_id,"type-list","type-task").unwrap();
        assert!(matches!(geometry,Geometry::Path{segments,..} if segments.len()==1));
    }
}
