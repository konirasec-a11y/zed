use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingCreateElement, ModelingCreateElementOrVisualDraft, ModelingCreateElementRequested,
    ModelingCreateModelElement, ModelingCreateModelElementRequested,
};
use crate::project::ProjectId;

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
