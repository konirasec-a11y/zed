use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingMoveView, ModelingMoveViewRequested};
use crate::project::ProjectId;

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
