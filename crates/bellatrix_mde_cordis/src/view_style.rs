use crate::mutation::register_command;
use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingReorderView, ModelingReorderViewRequested, ModelingSetViewLabel,
    ModelingSetViewLabelRequested, ModelingSetViewPath, ModelingSetViewPathRequested,
    ModelingSetViewRouting, ModelingSetViewRoutingRequested, ModelingSetViewStyle,
    ModelingSetViewStyleRequested, ModelingSetViewTiming, ModelingSetViewTimingRequested,
};
use crate::project::{ProjectId, ProjectService};
use cordis::Context;
use std::{io, sync::Arc};

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
    register_command::<ModelingSetViewLabel>(context, &projects, set_view_label_command)?;
    register_command::<ModelingSetViewTiming>(context, &projects, set_view_timing_command)?;
    register_command::<ModelingSetViewStyle>(context, &projects, set_view_style_command)?;
    register_command::<ModelingSetViewPath>(context, &projects, set_view_path_command)?;
    register_command::<ModelingSetViewRouting>(context, &projects, set_view_routing_command)?;
    register_command::<ModelingReorderView>(context, &projects, reorder_view_command)?;
    Ok(())
}
