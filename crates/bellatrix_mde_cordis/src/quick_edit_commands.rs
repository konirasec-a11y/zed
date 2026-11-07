use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingQuickEdit, ModelingQuickEditRequested};
use crate::project::ProjectId;

pub(crate) fn execute_quick_edit_command(
    request: ModelingQuickEditRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::QuickEdit {
            diagram_id: request.diagram_id,
            element_id: request.element_id,
            view_id: request.view_id,
            action: request.action,
        },
    )
}
