//! Rectangle resizing is committed by MDE, not by GPUI.
//! The current model supports other geometry types; they are rejected here
//! rather than silently converted to rectangles.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::Geometry;
use bellatrix_mde_modeling::Graph;
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

pub(crate) async fn resize_view(
    ctx: &Context,
    projects: &ProjectService,
    request: ModelingResizeViewRequested,
) -> Result<u64, String> {
    if !request.width.is_finite() || !request.height.is_finite()
        || request.width < 80.0 || request.height < 48.0
    {
        return Err("view dimensions must be finite and at least 80x48".into());
    }
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision {
        return Err("stale view resize; refresh project snapshot".into());
    }
    let namespace = namespace_for(&document);
    let mut section = from_document(&document)?;
    let diagram_id = section.diagrams.iter()
        .find(|d| d.views.iter().any(|v| v.id == request.view_id))
        .map(|d| d.id.clone()).ok_or("view not found")?;
    let view = section.diagrams.iter_mut().flat_map(|diagram| &mut diagram.views)
        .find(|view| view.id == request.view_id).ok_or("view not found")?;
    match &mut view.geometry {
        Geometry::Rect { width, height, .. } => {
            *width = request.width;
            *height = request.height;
        }
        _ => return Err("only rectangle views can be resized with this command".into()),
    }
    crate::create_relation::reroute(&mut section, &diagram_id);
    Graph::new(&section.to_model(document.id().as_str(), document.name()))
        .validate().map_err(|error| error.to_string())?;
    let state = serde_json::to_value(section).map_err(|error| error.to_string())?;
    match ctx.query::<ProjectReplace>(Routing::Unscoped, ProjectReplaceRequested {
        id: request.project_id,
        namespace: namespace.into(),
        expected_revision: request.expected_revision,
        state,
    }).await.map_err(|error| error.to_string())? {
        QueryOutcome::Answer(Ok(updated)) => Ok(updated.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService unavailable".into()),
    }
}
