use crate::Command;
pub use bellatrix_mde_contracts::events::{
    ModelingCreateVisualDraft, ModelingCreateVisualDraftRequested, ModelingPlaceElementView,
    ModelingPlaceElementViewRequested, ModelingRemoveView, ModelingRemoveViewRequested,
};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::Context;
use cordis::event::responder;
use std::{convert::Infallible, io, sync::Arc};

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
    let draft_projects = Arc::clone(&projects);
    let _draft = context
        .on::<ModelingCreateVisualDraft, _>(responder(
            move |context: Context, request: ModelingCreateVisualDraftRequested| {
                let projects = Arc::clone(&draft_projects);
                async move {
                    let (project_id, expected_revision, command) =
                        create_visual_draft_command(request);
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        project_id,
                        expected_revision,
                        command,
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let place_projects = Arc::clone(&projects);
    let _place = context
        .on::<ModelingPlaceElementView, _>(responder(
            move |context: Context, request: ModelingPlaceElementViewRequested| {
                let projects = Arc::clone(&place_projects);
                async move {
                    let (project_id, expected_revision, command) =
                        place_element_view_command(request);
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        project_id,
                        expected_revision,
                        command,
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let _remove = context
        .on::<ModelingRemoveView, _>(responder(
            move |context: Context, request: ModelingRemoveViewRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let (project_id, expected_revision, command) = remove_view_command(request);
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        project_id,
                        expected_revision,
                        command,
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}
