use crate::Command;
pub use bellatrix_mde::contracts::events::{
    ModelingCreateRelation, ModelingCreateRelationAtViewsOrVisualDraft,
    ModelingCreateRelationAtViewsRequested, ModelingCreateRelationOrVisualDraft,
    ModelingCreateRelationRequested,
};
use crate::project::ProjectId;

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
