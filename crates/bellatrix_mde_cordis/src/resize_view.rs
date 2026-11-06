use crate::Command;
pub use bellatrix_mde_contracts::events::{ModelingResizeView, ModelingResizeViewRequested};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::Context;

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

pub(crate) async fn resize_view(
    context: &Context,
    projects: &ProjectService,
    request: ModelingResizeViewRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = resize_view_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
