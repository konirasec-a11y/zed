use crate::Command;
pub use bellatrix_mde_contracts::events::{
    ModelingSetNameExpression, ModelingSetNameExpressionRequested,
};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::Context;

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

pub(crate) async fn set_name_expression(
    context: &Context,
    projects: &ProjectService,
    request: ModelingSetNameExpressionRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = set_name_expression_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
