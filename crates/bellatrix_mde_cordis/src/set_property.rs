use crate::Command;
pub use bellatrix_mde::contracts::events::{ModelingSetProperty, ModelingSetPropertyRequested};
use crate::project::{ProjectId, ProjectService};
use cordis::Context;

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

pub(crate) async fn set_property(
    context: &Context,
    projects: &ProjectService,
    request: ModelingSetPropertyRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = set_property_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
