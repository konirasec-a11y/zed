use crate::mutation::register_command;
use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingCreateVisualDraft, ModelingCreateVisualDraftRequested, ModelingPlaceElementView,
    ModelingPlaceElementViewRequested, ModelingRemoveView, ModelingRemoveViewRequested,
};
use crate::project::{ProjectId, ProjectService};
use cordis::Context;
use std::{io, sync::Arc};

pub(crate) fn create_visual_draft_command(
    request: ModelingCreateVisualDraftRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::CreateVisualDraft {
            diagram_id: request.diagram_id,
            notation_kind: request.notation_kind,
            label: request.label,
            geometry: request.geometry,
            properties: request.properties,
            parent_view_id: request.parent_view_id,
            source_view_id: request.source_view_id,
            target_view_id: request.target_view_id,
        },
    )
}

pub(crate) fn place_element_view_command(
    request: ModelingPlaceElementViewRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::PlaceElementView {
            diagram_id: request.diagram_id,
            element_id: request.element_id,
            x: request.x,
            y: request.y,
        },
    )
}

pub(crate) fn remove_view_command(
    request: ModelingRemoveViewRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::RemoveView {
            diagram_id: request.diagram_id,
            view_id: request.view_id,
        },
    )
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    register_command::<ModelingCreateVisualDraft>(context, &projects, create_visual_draft_command)?;
    register_command::<ModelingPlaceElementView>(context, &projects, place_element_view_command)?;
    register_command::<ModelingRemoveView>(context, &projects, remove_view_command)?;
    Ok(())
}
