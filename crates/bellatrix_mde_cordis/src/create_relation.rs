use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingCreateRelation, ModelingCreateRelationAtViewsOrVisualDraft,
    ModelingCreateRelationAtViewsRequested, ModelingCreateRelationOrVisualDraft,
    ModelingCreateRelationRequested,
};
use crate::project::{ProjectId, ProjectService};
use cordis::Context;

pub(crate) fn create_relation_at_views_or_visual_draft_command(
    request: ModelingCreateRelationAtViewsRequested,
) -> (ProjectId, u64, Command) {
    (
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
}

pub(crate) async fn create_relation_at_views_or_visual_draft(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateRelationAtViewsRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) =
        create_relation_at_views_or_visual_draft_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
pub(crate) fn create_relation_or_visual_draft_command(
    request: ModelingCreateRelationRequested,
) -> (ProjectId, u64, Command) {
    (
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
}

pub(crate) async fn create_relation_or_visual_draft(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateRelationRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = create_relation_or_visual_draft_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
pub(crate) fn create_relation_command(
    request: ModelingCreateRelationRequested,
) -> (ProjectId, u64, Command) {
    (
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
}

pub(crate) async fn create_relation(
    context: &Context,
    projects: &ProjectService,
    request: ModelingCreateRelationRequested,
) -> Result<u64, String> {
    let (project_id, expected_revision, command) = create_relation_command(request);
    crate::mutation::execute(context, projects, project_id, expected_revision, command).await
}
