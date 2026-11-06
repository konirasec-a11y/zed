use crate::Command;
pub use bellatrix_mde_contracts::events::{ModelingDeleteElement, ModelingDeleteElementRequested};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::Context;

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

pub(crate) async fn delete_element(
    context: &Context,
    projects: &ProjectService,
    request: ModelingDeleteElementRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = delete_element_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
