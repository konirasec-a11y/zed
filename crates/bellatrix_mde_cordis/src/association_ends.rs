use crate::Command;
use crate::AssociationEndEdit;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

#[derive(Clone, Debug)]
pub struct ModelingEditAssociationEndsRequested {
    pub project_id: ProjectId,
    pub association_id: String,
    pub edits: Vec<AssociationEndEdit>,
    pub expected_revision: u64,
}

pub struct ModelingEditAssociationEnds;
impl Event for ModelingEditAssociationEnds {
    const NAME: &'static str = "bellatrix.mde.edit-association-ends.v1";
    type Args = ModelingEditAssociationEndsRequested;
    type Output = Result<u64, String>;
}

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
