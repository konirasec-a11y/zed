use bellatrix_mde::commands::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

#[derive(Clone, Debug)]
pub struct ModelingSetNameExpressionRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub expression: String,
    pub expected_revision: u64,
}
pub struct ModelingSetNameExpression;
impl Event for ModelingSetNameExpression {
    const NAME: &'static str = "bellatrix.mde.set-name-expression.v1";
    type Args = ModelingSetNameExpressionRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn set_name_expression(context: &Context, projects: &ProjectService,
    request: ModelingSetNameExpressionRequested) -> Result<u64, String> {
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision, Command::SetNameExpression {
        element_id: request.element_id, expression: request.expression,
    }).await
}
