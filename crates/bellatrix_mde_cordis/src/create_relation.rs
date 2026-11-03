use bellatrix_mde::{ModelingRelationKind, commands::Command};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

pub use bellatrix_mde::projected_relationship_paths as projected_automatic_routes;

#[derive(Clone, Debug)]
pub struct ModelingCreateRelationRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub kind: ModelingRelationKind,
    pub from: String,
    pub to: String,
    pub expected_revision: u64,
}
pub struct ModelingCreateRelation;
impl Event for ModelingCreateRelation {
    const NAME: &'static str = "bellatrix.mde.create-relation.v1";
    type Args = ModelingCreateRelationRequested;
    type Output = Result<u64, String>;
}
pub struct ModelingCreateRelationOrVisualDraft;
impl Event for ModelingCreateRelationOrVisualDraft {
    const NAME: &'static str = "bellatrix.mde.create-relation-or-visual-draft.v1";
    type Args = ModelingCreateRelationRequested;
    type Output = Result<u64, String>;
}
pub(crate) async fn create_relation_or_visual_draft(context: &Context, projects: &ProjectService,
    request: ModelingCreateRelationRequested) -> Result<u64, String> {
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision,
        Command::CreateRelationOrVisualDraft { diagram_id: request.diagram_id, kind: request.kind, from: request.from, to: request.to }).await
}
pub(crate) async fn create_relation(context: &Context, projects: &ProjectService,
    request: ModelingCreateRelationRequested) -> Result<u64, String> {
    crate::mutation::execute(context, projects, request.project_id, request.expected_revision, Command::CreateRelation {
        diagram_id: request.diagram_id, kind: request.kind, from: request.from, to: request.to,
    }).await
}
