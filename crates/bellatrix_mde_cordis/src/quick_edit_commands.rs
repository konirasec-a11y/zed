use crate::Command;
use crate::UmlQuickCreate;
pub use bellatrix_mde_contracts::events::{ModelingQuickEdit, ModelingQuickEditRequested};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

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
            action: request.action,
        },
    )
    .await
}
