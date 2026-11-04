pub use bellatrix_mde_contracts::events::{ModelingResizeViewRequested, ModelingResizeView};
use crate::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

pub(crate) async fn resize_view(
    context: &Context,
    projects: &ProjectService,
    request: ModelingResizeViewRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::ResizeView {
            view_id: request.view_id,
            width: request.width,
            height: request.height,
        },
    )
    .await
}
