use crate::Command;
pub use bellatrix_mde_contracts::events::{ModelingQuickEdit, ModelingQuickEditRequested};
use bellatrix_project::ProjectService;
use cordis::Context;

pub(crate) async fn execute_quick_edit(
    context: &Context,
    projects: &ProjectService,
    request: ModelingQuickEditRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::QuickEdit {
            diagram_id: request.diagram_id,
            element_id: request.element_id,
            view_id: request.view_id,
            action: request.action,
        },
    )
    .await
}
