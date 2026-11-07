use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingSetProperty, ModelingSetPropertyRequested};
use crate::project::ProjectId;

pub(crate) fn set_property_command(
    request: ModelingSetPropertyRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetProperty {
            element_id: request.element_id,
            path: request.path,
            value: request.value,
        },
    )
}
