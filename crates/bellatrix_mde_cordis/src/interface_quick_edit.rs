//! Compatibility boundary for the original interface-only public event.
//! Common behavior now lives in quick_edit_commands; no mutation is implemented here.
use crate::quick_edit_commands::{self, ModelingQuickEditRequested};
use crate::project_integration::from_document;
use bellatrix_mde_modeling::{UmlQuickCommand, UmlType};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceQuickAction {
    AddNote, AddConstraint, AddReception, AddSubinterface, AddSuperinterface, AddRealizingClass,
}
impl From<InterfaceQuickAction> for UmlQuickCommand {
    fn from(action: InterfaceQuickAction) -> Self {
        match action {
            InterfaceQuickAction::AddNote => Self::AddNote,
            InterfaceQuickAction::AddConstraint => Self::AddConstraint,
            InterfaceQuickAction::AddReception => Self::AddReception,
            InterfaceQuickAction::AddSubinterface => Self::AddSubtype,
            InterfaceQuickAction::AddSuperinterface => Self::AddSupertype,
            InterfaceQuickAction::AddRealizingClass => Self::AddRealizingClass,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ModelingInterfaceQuickEditRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub interface_id: String,
    pub action: InterfaceQuickAction,
    pub expected_revision: u64,
}
pub struct ModelingInterfaceQuickEdit;
impl Event for ModelingInterfaceQuickEdit {
    const NAME: &'static str = "bellatrix.mde.interface-quick-edit.v1";
    type Args = ModelingInterfaceQuickEditRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn execute_quick_edit(
    context: &Context, projects: &ProjectService, request: ModelingInterfaceQuickEditRequested,
) -> Result<u64, String> {
    let document = projects.snapshot(&request.project_id).map_err(|error| error.to_string())?;
    let section = from_document(&document)?;
    let subject = section.elements.iter().find(|element| element.id == request.interface_id)
        .ok_or("interface not found")?;
    if UmlType::from_kind(&subject.kind) != Some(UmlType::Interface) {
        return Err("legacy Interface Quick Edit requires an Interface".into());
    }
    quick_edit_commands::execute_quick_edit(context, projects, ModelingQuickEditRequested {
        project_id: request.project_id, diagram_id: request.diagram_id,
        element_id: request.interface_id, action: request.action.into(),
        expected_revision: request.expected_revision,
    }).await
}
