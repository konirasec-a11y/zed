use crate::Command;
pub use bellatrix_mde_contracts::events::{
    ModelingAddMember, ModelingAddMemberRequested, ModelingMoveParameter,
    ModelingMoveParameterRequested,
};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

use crate::ModelingMemberKind;

pub(crate) async fn add_member(
    context: &Context,
    projects: &ProjectService,
    request: ModelingAddMemberRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::AddMember {
            owner_id: request.owner_id,
            kind: request.kind,
        },
    )
    .await
}

pub(crate) async fn move_parameter(
    context: &Context,
    projects: &ProjectService,
    request: ModelingMoveParameterRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::MoveParameter {
            parameter_id: request.parameter_id,
            up: request.up,
        },
    )
    .await
}
