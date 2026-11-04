use crate::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ModelingSetPropertyRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub path: String,
    pub value: Value,
    pub expected_revision: u64,
}
pub struct ModelingSetProperty;
impl Event for ModelingSetProperty {
    const NAME: &'static str = "bellatrix.mde.set-property.v1";
    type Args = ModelingSetPropertyRequested;
    type Output = Result<u64, String>;
}
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
