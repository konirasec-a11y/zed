use crate::Command;
use crate::ModelingRelationKind;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

#[derive(Clone, Debug)]
pub struct ModelingCreateRelationRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub owner_id: Option<String>,
    pub kind: ModelingRelationKind,
    pub from: String,
    pub to: String,
    pub expected_revision: u64,
}
#[derive(Clone, Debug)]
pub struct ModelingCreateRelationAtViewsRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub owner_id: Option<String>,
    pub kind: ModelingRelationKind,
    pub source_view_id: String,
    pub target_view_id: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub expected_revision: u64,
}

pub struct ModelingCreateRelationAtViewsOrVisualDraft;
impl Event for ModelingCreateRelationAtViewsOrVisualDraft {
    const NAME: &'static str = "bellatrix.mde.create-relation-at-views-or-visual-draft.v1";
    type Args = ModelingCreateRelationAtViewsRequested;
    type Output = Result<u64, String>;
}

pub(crate) async fn create_relation_at_views_or_visual_draft(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateRelationAtViewsRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::CreateRelationAtViewsOrVisualDraft {
            diagram_id: request.diagram_id,
            owner_id: request.owner_id,
            kind: request.kind,
            from: request.from,
            to: request.to,
            source_view_id: request.source_view_id,
            target_view_id: request.target_view_id,
        },
    )
    .await
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
pub(crate) async fn create_relation_or_visual_draft(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateRelationRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::CreateRelationOrVisualDraft {
            diagram_id: request.diagram_id,
            owner_id: request.owner_id,
            kind: request.kind,
            from: request.from,
            to: request.to,
        },
    )
    .await
}
pub(crate) async fn create_relation(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateRelationRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::CreateRelation {
            diagram_id: request.diagram_id,
            owner_id: request.owner_id,
            kind: request.kind,
            from: request.from,
            to: request.to,
        },
    )
    .await
}
