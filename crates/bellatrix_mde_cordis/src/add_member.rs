use crate::Command;
pub use bellatrix_mde_contracts::events::{
    ModelingAddMember, ModelingAddMemberRequested, ModelingMoveParameter,
    ModelingMoveParameterRequested,
};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::Context;

pub(crate) fn add_member_command(request: ModelingAddMemberRequested) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::AddMember {
            owner_id: request.owner_id,
            kind: request.kind,
        },
    )
}

pub(crate) async fn add_member(
    context: &Context,
    projects: &ProjectService,
    request: ModelingAddMemberRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = add_member_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
pub(crate) fn move_parameter_command(
    request: ModelingMoveParameterRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::MoveParameter {
            parameter_id: request.parameter_id,
            up: request.up,
        },
    )
}

pub(crate) async fn move_parameter(
    context: &Context,
    projects: &ProjectService,
    request: ModelingMoveParameterRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = move_parameter_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
