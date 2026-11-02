use bellatrix_mde::commands::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

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
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision, Command::ResizeView {
        view_id: request.view_id, width: request.width, height: request.height,
    }).await
}
