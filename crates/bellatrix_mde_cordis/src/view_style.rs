use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingReorderView, ModelingReorderViewRequested, ModelingSetViewLabel,
    ModelingSetViewLabelRequested, ModelingSetViewPath, ModelingSetViewPathRequested,
    ModelingSetViewRouting, ModelingSetViewRoutingRequested, ModelingSetViewStyle,
    ModelingSetViewStyleRequested, ModelingSetViewTiming, ModelingSetViewTimingRequested,
};
use crate::project::{ProjectId, ProjectService};
use cordis::{Context, event::responder};
use std::{convert::Infallible, io, sync::Arc};

pub(crate) fn set_view_label_command(
    request: ModelingSetViewLabelRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetViewLabel {
            view_id: request.view_id,
            label: request.label,
        },
    )
}

pub(crate) fn set_view_timing_command(
    request: ModelingSetViewTimingRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetViewTiming {
            view_id: request.view_id,
            timing: request.timing,
        },
    )
}

pub(crate) fn set_view_style_command(
    request: ModelingSetViewStyleRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetViewStyle {
            diagram_id: request.diagram_id,
            view_id: request.view_id,
            style: request.style,
        },
    )
}

pub(crate) fn set_view_path_command(
    request: ModelingSetViewPathRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetViewPath {
            diagram_id: request.diagram_id,
            view_id: request.view_id,
            geometry: request.geometry,
        },
    )
}

pub(crate) fn set_view_routing_command(
    request: ModelingSetViewRoutingRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetViewRouting {
            diagram_id: request.diagram_id,
            view_id: request.view_id,
            routing: request.routing,
        },
    )
}

pub(crate) fn reorder_view_command(
    request: ModelingReorderViewRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::ReorderView {
            diagram_id: request.diagram_id,
            view_id: request.view_id,
            order: request.order,
        },
    )
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let label_projects = Arc::clone(&projects);
    let _label = context
        .on::<ModelingSetViewLabel, _>(responder(
            move |context: Context, request: ModelingSetViewLabelRequested| {
                let projects = Arc::clone(&label_projects);
                async move {
                    let (project_id, expected_revision, command) = set_view_label_command(request);
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
    let service = Arc::clone(&projects);
    let _timing = context
        .on::<ModelingSetViewTiming, _>(responder(
            move |context: Context, request: ModelingSetViewTimingRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let (project_id, expected_revision, command) = set_view_timing_command(request);
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
    let service = Arc::clone(&projects);
    let _style = context
        .on::<ModelingSetViewStyle, _>(responder(
            move |context: Context, request: ModelingSetViewStyleRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let (project_id, expected_revision, command) = set_view_style_command(request);
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
    let service = Arc::clone(&projects);
    let _path = context
        .on::<ModelingSetViewPath, _>(responder(
            move |context: Context, request: ModelingSetViewPathRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let (project_id, expected_revision, command) = set_view_path_command(request);
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
    let service = Arc::clone(&projects);
    let _routing = context
        .on::<ModelingSetViewRouting, _>(responder(
            move |context: Context, request: ModelingSetViewRoutingRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let (project_id, expected_revision, command) =
                        set_view_routing_command(request);
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
    let service = Arc::clone(&projects);
    let _order = context
        .on::<ModelingReorderView, _>(responder(
            move |context: Context, request: ModelingReorderViewRequested| {
                let projects = Arc::clone(&service);
                async move {
                    let (project_id, expected_revision, command) = reorder_view_command(request);
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
