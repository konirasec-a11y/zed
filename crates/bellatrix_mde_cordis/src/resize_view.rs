//! View resizing preserves the notation's content bounds and re-routes its connectors.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::Geometry;
use bellatrix_mde_modeling::{Graph, UmlType, classifier_contents};
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Debug)]
pub struct ModelingResizeViewRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub width: f64,
    pub height: f64,
    pub expected_revision: u64,
}
pub struct ModelingResizeView;
impl Event for ModelingResizeView {
    const NAME: &'static str = "bellatrix.mde.resize-view.v1";
    type Args = ModelingResizeViewRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn resize_view(context: &Context, projects: &ProjectService,
    request: ModelingResizeViewRequested) -> Result<u64, String> {
    if !request.width.is_finite() || !request.height.is_finite() || request.width < 80.0 || request.height < 48.0 {
        return Err("view dimensions must be finite and at least 80x48".into());
    }
    let document = projects.snapshot(&request.project_id).map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision { return Err("stale view resize; refresh project snapshot".into()); }
    let mut section = from_document(&document)?;
    let diagram_id = section.diagrams.iter().find(|diagram| diagram.views.iter().any(|view| view.id == request.view_id))
        .map(|diagram| diagram.id.clone()).ok_or("view not found")?;
    let subject = section.diagrams.iter().flat_map(|diagram| &diagram.views)
        .find(|view| view.id == request.view_id).and_then(|view| view.subject.as_deref());
    let minimum_height = subject.and_then(|id| section.elements.iter().find(|element| element.id == id))
        .filter(|element| UmlType::from_kind(&element.kind) == Some(UmlType::Interface))
        .map(|element| classifier_contents(&section, &element.id).minimum_interface_height())
        .unwrap_or(48.0);
    let view = section.diagrams.iter_mut().flat_map(|diagram| &mut diagram.views)
        .find(|view| view.id == request.view_id).ok_or("view not found")?;
    match &mut view.geometry {
        Geometry::Rect { width, height, .. } => { *width = request.width; *height = request.height.max(minimum_height); }
        _ => return Err("only node Views can be resized with this command".into()),
    }
    crate::create_relation::reroute(&mut section, &diagram_id);
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
