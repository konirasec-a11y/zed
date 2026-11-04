use crate::Command;
use crate::ModelingElementKind;
pub use bellatrix_mde_contracts::events::{
    ModelingCreateElement, ModelingCreateElementOrVisualDraft, ModelingCreateElementRequested,
    ModelingCreateModelElement, ModelingCreateModelElementRequested,
};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

pub(crate) async fn create_element_or_visual_draft(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateElementRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
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
    .await
}
pub(crate) async fn create_element(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateElementRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
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
    .await
}

pub(crate) async fn create_model_element(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateModelElementRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::CreateModelElement {
            owner_id: request.owner_id,
            kind: request.kind,
            name: request.name,
        },
    )
    .await
}
