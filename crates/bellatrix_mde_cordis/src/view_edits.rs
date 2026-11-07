use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingMoveView, ModelingMoveViewRequested};
use crate::project::{ProjectId, ProjectService};
use cordis::Context;

pub(crate) fn move_view_command(request: ModelingMoveViewRequested) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::MoveView {
            view_id: request.view_id,
            dx: request.dx,
            dy: request.dy,
        },
    )
}

pub(crate) async fn move_view(
    context: &Context,
    projects: &ProjectService,
    request: ModelingMoveViewRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = move_view_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
