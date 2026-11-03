pub use crate::kernel_commands::{
    AssociationEndDraft, AssociationEndEdit, AssociationEndOwner, Command,
    ModelingElementKind, ModelingMemberKind, ModelingTool, RemovedAssociationEnd,
    ViewOrder, name_expression_text,
};
use crate::Project;

pub fn apply(project: &Project, command: &Command, revision: u64) -> Result<Project, String> {
    crate::kernel_commands::apply_with_diagram_operations(
        project, command, revision, &crate::diagram_operation_policy::ModelingDiagramOperations,
    )
}
