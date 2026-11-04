use crate::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::event::responder;
use cordis::{Context, Event};
use std::{convert::Infallible, io, sync::Arc};

#[derive(Clone, Debug)]
pub struct ModelingCreateVisualDraftRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub notation_kind: String,
    pub label: Option<String>,
    pub geometry: crate::Geometry,
    pub properties: std::collections::BTreeMap<String, serde_json::Value>,
    pub parent_view_id: Option<String>,
    pub source_view_id: Option<String>,
    pub target_view_id: Option<String>,
    pub expected_revision: u64,
}
pub struct ModelingCreateVisualDraft;
impl Event for ModelingCreateVisualDraft {
    const NAME: &'static str = "bellatrix.mde.create-visual-draft.v1";
    type Args = ModelingCreateVisualDraftRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingRemoveViewRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub expected_revision: u64,
}
pub struct ModelingRemoveView;
impl Event for ModelingRemoveView {
    const NAME: &'static str = "bellatrix.mde.remove-view.v1";
    type Args = ModelingRemoveViewRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingPlaceElementViewRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
    pub expected_revision: u64,
}
pub struct ModelingPlaceElementView;
impl Event for ModelingPlaceElementView {
    const NAME: &'static str = "bellatrix.mde.place-element-view.v1";
    type Args = ModelingPlaceElementViewRequested;
    type Output = Result<u64, String>;
}

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
