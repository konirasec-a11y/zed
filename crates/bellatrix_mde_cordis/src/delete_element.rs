use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingDeleteElement, ModelingDeleteElementRequested};
use crate::project::ProjectId;

pub(crate) fn delete_element_command(
    request: ModelingDeleteElementRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::DeleteElement {
            element_id: request.element_id,
        },
    )
}
