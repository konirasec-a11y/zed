use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingResizeView, ModelingResizeViewRequested};
use crate::project::ProjectId;

pub(crate) fn resize_view_command(
    request: ModelingResizeViewRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::ResizeView {
            view_id: request.view_id,
            width: request.width,
            height: request.height,
        },
    )
}
