use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingEditAssociationEnds, ModelingEditAssociationEndsRequested,
};
use crate::project::ProjectId;

pub(crate) fn edit_association_ends_command(
    request: ModelingEditAssociationEndsRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::EditAssociationEnds {
            association_id: request.association_id,
            edits: request.edits,
        },
    )
}
