use crate::Command;
pub use bellatrix_mde_contracts::events::{ModelingResizeView, ModelingResizeViewRequested};
use bellatrix_project::ProjectService;
use cordis::Context;

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
