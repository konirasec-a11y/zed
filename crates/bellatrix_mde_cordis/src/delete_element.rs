use bellatrix_mde::commands::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

#[derive(Clone, Debug)]
pub struct ModelingDeleteElementRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub expected_revision: u64,
}
pub struct ModelingDeleteElement;
impl Event for ModelingDeleteElement {
    const NAME: &'static str = "bellatrix.mde.delete-element.v1";
    type Args = ModelingDeleteElementRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn delete_element(context: &Context, projects: &ProjectService,
    request: ModelingDeleteElementRequested) -> Result<u64, String> {
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision, Command::DeleteElement {
        element_id: request.element_id,
    }).await
}
