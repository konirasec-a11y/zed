use bellatrix_mde::{UmlQuickCommand, commands::Command};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

#[derive(Clone, Debug)]
pub struct ModelingQuickEditRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub element_id: String,
    pub action: UmlQuickCommand,
    pub expected_revision: u64,
}
pub struct ModelingQuickEdit;
impl Event for ModelingQuickEdit {
    const NAME: &'static str = "bellatrix.mde.quick-edit.v1";
    type Args = ModelingQuickEditRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn execute_quick_edit(context: &Context, projects: &ProjectService,
    request: ModelingQuickEditRequested) -> Result<u64, String> {
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision, Command::QuickEdit {
        diagram_id: request.diagram_id, element_id: request.element_id, action: request.action,
    }).await
}
