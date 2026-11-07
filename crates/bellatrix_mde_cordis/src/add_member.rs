use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingAddMember, ModelingAddMemberRequested, ModelingMoveParameter,
    ModelingMoveParameterRequested,
};
use crate::project::ProjectId;

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
