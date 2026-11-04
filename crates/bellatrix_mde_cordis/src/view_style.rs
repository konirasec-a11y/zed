use crate::{Command, Geometry};
use crate::{RelationshipRouting, Style, TimingPresentation, ViewOrder};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event, event::responder};
use std::{convert::Infallible, io, sync::Arc};

#[derive(Clone, Debug)]
pub struct ModelingSetViewLabelRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub label: Option<String>,
    pub expected_revision: u64,
}
pub struct ModelingSetViewLabel;
impl Event for ModelingSetViewLabel {
    const NAME: &'static str = "bellatrix.mde.set-view-label.v1";
    type Args = ModelingSetViewLabelRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingSetViewTimingRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub timing: Option<TimingPresentation>,
    pub expected_revision: u64,
}
pub struct ModelingSetViewTiming;
impl Event for ModelingSetViewTiming {
    const NAME: &'static str = "bellatrix.mde.set-view-timing.v1";
    type Args = ModelingSetViewTimingRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingSetViewStyleRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub style: Style,
    pub expected_revision: u64,
}
pub struct ModelingSetViewStyle;
impl Event for ModelingSetViewStyle {
    const NAME: &'static str = "bellatrix.mde.set-view-style.v1";
    type Args = ModelingSetViewStyleRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingSetViewPathRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub geometry: Geometry,
    pub expected_revision: u64,
}
pub struct ModelingSetViewPath;
impl Event for ModelingSetViewPath {
    const NAME: &'static str = "bellatrix.mde.set-view-path.v1";
    type Args = ModelingSetViewPathRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingSetViewRoutingRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub routing: RelationshipRouting,
    pub expected_revision: u64,
}
pub struct ModelingSetViewRouting;
impl Event for ModelingSetViewRouting {
    const NAME: &'static str = "bellatrix.mde.set-view-routing.v1";
    type Args = ModelingSetViewRoutingRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingReorderViewRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub order: ViewOrder,
    pub expected_revision: u64,
}
pub struct ModelingReorderView;
impl Event for ModelingReorderView {
    const NAME: &'static str = "bellatrix.mde.reorder-view.v1";
    type Args = ModelingReorderViewRequested;
    type Output = Result<u64, String>;
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let label_projects = Arc::clone(&projects);
    let _label = context
        .on::<ModelingSetViewLabel, _>(responder(
            move |context: Context, request: ModelingSetViewLabelRequested| {
                let projects = Arc::clone(&label_projects);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::SetViewLabel {
                            view_id: request.view_id,
                            label: request.label,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let service = Arc::clone(&projects);
    let _timing = context
        .on::<ModelingSetViewTiming, _>(responder(
            move |context: Context, request: ModelingSetViewTimingRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let timing = request.timing;
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::SetViewTiming {
                            view_id: request.view_id,
                            timing,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let service = Arc::clone(&projects);
    let _style = context
        .on::<ModelingSetViewStyle, _>(responder(
            move |context: Context, request: ModelingSetViewStyleRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let style = request.style;
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::SetViewStyle {
                            diagram_id: request.diagram_id,
                            view_id: request.view_id,
                            style,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let service = Arc::clone(&projects);
    let _path = context
        .on::<ModelingSetViewPath, _>(responder(
            move |context: Context, request: ModelingSetViewPathRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::SetViewPath {
                            diagram_id: request.diagram_id,
                            view_id: request.view_id,
                            geometry: request.geometry,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let service = Arc::clone(&projects);
    let _routing = context
        .on::<ModelingSetViewRouting, _>(responder(
            move |context: Context, request: ModelingSetViewRoutingRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let routing = request.routing;
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::SetViewRouting {
                            diagram_id: request.diagram_id,
                            view_id: request.view_id,
                            routing,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let service = Arc::clone(&projects);
    let _order = context
        .on::<ModelingReorderView, _>(responder(
            move |context: Context, request: ModelingReorderViewRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::ReorderView {
                            diagram_id: request.diagram_id,
                            view_id: request.view_id,
                            order: request.order,
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
