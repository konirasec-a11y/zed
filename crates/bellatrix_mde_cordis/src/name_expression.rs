pub use bellatrix_mde_contracts::events::{ModelingSetNameExpressionRequested, ModelingSetNameExpression};
use crate::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

pub(crate) async fn set_name_expression(
    context: &Context,
    projects: &ProjectService,
    request: ModelingSetNameExpressionRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::SetNameExpression {
            element_id: request.element_id,
            expression: request.expression,
        },
    )
    .await
}
