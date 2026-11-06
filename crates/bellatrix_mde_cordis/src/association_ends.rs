use crate::Command;
pub use bellatrix_mde_contracts::events::{
    ModelingEditAssociationEnds, ModelingEditAssociationEndsRequested,
};
use bellatrix_project::ProjectService;
use cordis::Context;

pub(crate) async fn edit_association_ends(
    context: &Context,
    projects: &ProjectService,
    request: ModelingEditAssociationEndsRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::EditAssociationEnds {
            association_id: request.association_id,
            edits: request.edits,
        },
    )
    .await
}
