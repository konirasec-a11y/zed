use crate::Command;
pub use bellatrix_mde_contracts::events::{ModelingMoveView, ModelingMoveViewRequested};
use bellatrix_project::ProjectService;
use cordis::Context;

pub(crate) async fn move_view(
    context: &Context,
    projects: &ProjectService,
    request: ModelingMoveViewRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::MoveView {
            view_id: request.view_id,
            dx: request.dx,
            dy: request.dy,
        },
    )
    .await
}
