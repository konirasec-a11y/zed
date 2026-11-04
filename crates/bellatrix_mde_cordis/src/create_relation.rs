pub use bellatrix_mde_contracts::events::{ModelingCreateRelationRequested, ModelingCreateRelationAtViewsRequested, ModelingCreateRelationAtViewsOrVisualDraft, ModelingCreateRelation, ModelingCreateRelationOrVisualDraft};
use crate::Command;
use crate::ModelingRelationKind;
use bellatrix_project::{ProjectId, ProjectService};
use cordis::{Context, Event};

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
