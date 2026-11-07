use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingSetNameExpression, ModelingSetNameExpressionRequested,
};
use crate::project::ProjectId;

pub(crate) fn set_name_expression_command(
    request: ModelingSetNameExpressionRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetNameExpression {
            element_id: request.element_id,
            expression: request.expression,
        },
    )
}
