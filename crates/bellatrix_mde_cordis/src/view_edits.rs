use bellatrix_mde::commands::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

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
pub(crate) async fn move_view(context: &Context, projects: &ProjectService,
    request: ModelingMoveViewRequested) -> Result<u64, String> {
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision, Command::MoveView {
        view_id: request.view_id, dx: request.dx, dy: request.dy,
    }).await
}
