use crate::Command;
pub use bellatrix_mde_contracts::events::{
    ModelingCreateElement, ModelingCreateElementOrVisualDraft, ModelingCreateElementRequested,
    ModelingCreateModelElement, ModelingCreateModelElementRequested,
};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::Context;

pub(crate) fn create_element_or_visual_draft_command(
    request: ModelingCreateElementRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::CreateElementOrVisualDraft {
            diagram_id: request.diagram_id,
            owner_id: request.owner_id,
            kind: request.kind,
            name: request.name,
            x: request.x,
            y: request.y,
        },
    )
}

pub(crate) async fn create_element_or_visual_draft(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateElementRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = create_element_or_visual_draft_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
pub(crate) fn create_element_command(
    request: ModelingCreateElementRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::CreateElement {
            owner_id: request.owner_id,
            diagram_id: request.diagram_id,
            kind: request.kind,
            name: request.name,
            x: request.x,
            y: request.y,
        },
    )
}

pub(crate) async fn create_element(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateElementRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = create_element_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
pub(crate) fn create_model_element_command(
    request: ModelingCreateModelElementRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::CreateModelElement {
            owner_id: request.owner_id,
            kind: request.kind,
            name: request.name,
        },
    )
}

pub(crate) async fn create_model_element(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateModelElementRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = create_model_element_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
