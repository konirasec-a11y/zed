pub use bellatrix_mde_contracts::events::{ModelingSetPropertyRequested, ModelingSetProperty};
use crate::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};
use serde_json::Value;

pub(crate) async fn set_property(
    context: &Context,
    projects: &ProjectService,
    request: ModelingSetPropertyRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::SetProperty {
            element_id: request.element_id,
            path: request.path,
            value: request.value,
        },
    )
    .await
}
