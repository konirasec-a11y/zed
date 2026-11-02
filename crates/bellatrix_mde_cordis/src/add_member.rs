use bellatrix_mde::commands::Command;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

pub use bellatrix_mde::commands::ModelingMemberKind;

#[derive(Clone, Debug)]
pub struct ModelingAddMemberRequested {
    pub project_id: ProjectId,
    pub owner_id: String,
    pub kind: ModelingMemberKind,
    pub expected_revision: u64,
}
pub struct ModelingAddMember;
impl Event for ModelingAddMember {
    const NAME: &'static str = "bellatrix.mde.add-member.v1";
    type Args = ModelingAddMemberRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn add_member(context: &Context, projects: &ProjectService,
    request: ModelingAddMemberRequested) -> Result<u64, String> {
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision, Command::AddMember {
        owner_id: request.owner_id, kind: request.kind,
    }).await
}
