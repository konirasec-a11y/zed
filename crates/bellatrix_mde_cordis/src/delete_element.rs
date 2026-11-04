pub use bellatrix_mde_contracts::events::{ModelingDeleteElementRequested, ModelingDeleteElement};
use crate::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

pub(crate) async fn delete_element(
    context: &Context,
    projects: &ProjectService,
    request: ModelingDeleteElementRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::DeleteElement {
            element_id: request.element_id,
        },
    )
    .await
}
