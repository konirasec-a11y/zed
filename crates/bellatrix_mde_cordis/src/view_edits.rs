//! View transforms are domain commands, never local-only GPUI mutations.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_modeling::Graph;
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Debug)]
pub struct ModelingMoveViewRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub dx: f64,
    pub dy: f64,
    pub expected_revision: u64,
}

pub struct ModelingMoveView;
impl Event for ModelingMoveView {
    const NAME: &'static str = "bellatrix.bellatrix-mde-modeling.move-view.v1";
    type Args = ModelingMoveViewRequested;
    type Output = Result<u64, String>;
}

pub(crate) async fn move_view(
    ctx: &Context,
    projects: &ProjectService,
    request: ModelingMoveViewRequested,
) -> Result<u64, String> {
    if !request.dx.is_finite() || !request.dy.is_finite() {
        return Err("view displacement must be finite".into());
    }
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|e| e.to_string())?;
    if document.revision() != request.expected_revision {
        return Err("stale view edit; refresh bellatrix-project snapshot".into());
    }
    let mut section = from_document(&document)?;
    let namespace = namespace_for(&document);
    let view = section
        .diagrams
        .iter_mut()
        .flat_map(|diagram| &mut diagram.views)
        .find(|view| view.id == request.view_id)
        .ok_or("view not found")?;
    view.geometry.translate(request.dx, request.dy);
    Graph::new(&section.to_model(document.id().as_str(), document.name()))
        .validate()
        .map_err(|e| e.to_string())?;
    let value = serde_json::to_value(section).map_err(|e| e.to_string())?;
    let answer = ctx
        .query::<ProjectReplace>(
            Routing::Unscoped,
            ProjectReplaceRequested {
                id: request.project_id,
                namespace: namespace.into(),
                expected_revision: request.expected_revision,
                state: value,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    match answer {
        QueryOutcome::Answer(Ok(document)) => Ok(document.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService is unavailable".into()),
    }
}
