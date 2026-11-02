use bellatrix_mde::commands::{Command, ModelingElementKind};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

#[derive(Clone, Debug)]
pub struct ModelingCreateElementRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub kind: ModelingElementKind,
    pub name: Option<String>,
    pub owner_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub expected_revision: u64,
}
pub struct ModelingCreateElement;
impl Event for ModelingCreateElement {
    const NAME: &'static str = "bellatrix.mde.create-element.v1";
    type Args = ModelingCreateElementRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn create_element(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateElementRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::CreateElement {
            owner_id: request.owner_id,
            diagram_id: request.diagram_id,
            kind: request.kind,
            name: request.name,
            x: request.x,
            y: request.y,
        },
    )
    .await
}
