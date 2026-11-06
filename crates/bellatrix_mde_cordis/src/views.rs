use crate::Command;
pub use bellatrix_mde_contracts::events::{
    ModelingCreateVisualDraft, ModelingCreateVisualDraftRequested, ModelingPlaceElementView,
    ModelingPlaceElementViewRequested, ModelingRemoveView, ModelingRemoveViewRequested,
};
use bellatrix_project::ProjectService;
use cordis::event::responder;
use cordis::Context;
use std::{convert::Infallible, io, sync::Arc};

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let draft_projects = Arc::clone(&projects);
    let _draft = context
        .on::<ModelingCreateVisualDraft, _>(responder(
            move |context: Context, request: ModelingCreateVisualDraftRequested| {
                let projects = Arc::clone(&draft_projects);
                async move {
                    let geometry = request.geometry;
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::CreateVisualDraft {
                            diagram_id: request.diagram_id,
                            notation_kind: request.notation_kind,
                            label: request.label,
                            geometry,
                            properties: request.properties,
                            parent_view_id: request.parent_view_id,
                            source_view_id: request.source_view_id,
                            target_view_id: request.target_view_id,
                        },
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
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::PlaceElementView {
                            diagram_id: request.diagram_id,
                            element_id: request.element_id,
                            x: request.x,
                            y: request.y,
                        },
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
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::RemoveView {
                            diagram_id: request.diagram_id,
                            view_id: request.view_id,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}
