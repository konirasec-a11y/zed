use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingQuickEdit, ModelingQuickEditRequested};
use crate::project::{ProjectId, ProjectService};
use cordis::Context;

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

pub(crate) async fn execute_quick_edit(
    context: &Context,
    projects: &ProjectService,
    request: ModelingQuickEditRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = execute_quick_edit_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
