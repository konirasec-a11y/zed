use crate::Command;
use crate::MODELING_NAMESPACE;
use crate::mutation::register_command;
pub use bellatrix_mde::contracts::events::{
    ModelingRename, ModelingRenameRequested, ModelingSetExplorerParent,
    ModelingSetExplorerParentRequested, ModelingSetOwner, ModelingSetOwnerRequested,
};
use crate::project::{ProjectDocument, ProjectId, ProjectService};
use cordis::{Context, InjectSpec, Plugin, Service};
use std::{convert::Infallible, io, sync::Arc};

/// Serialize the existing canonical section for an MDE application API.
pub(crate) fn section_json(document: &ProjectDocument) -> Result<String, String> {
    let section = document.section(MODELING_NAMESPACE)
        .ok_or("unsupported or missing MDE section: expected bellatrix.mde; beta documents in other namespaces are not supported")?;
    serde_json::to_string(section).map_err(|error| error.to_string())
}

pub(crate) fn set_owner_command(request: ModelingSetOwnerRequested) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetOwner {
            element_id: request.element_id,
            owner_id: request.owner_id,
        },
    )
}

pub(crate) fn set_explorer_parent_command(
    request: ModelingSetExplorerParentRequested,
) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::SetExplorerParent {
            subject_id: request.subject_id,
            parent_id: request.parent_id,
            parent_occurrence: request.parent_occurrence,
        },
    )
}

pub(crate) fn rename_command(request: ModelingRenameRequested) -> (ProjectId, u64, Command) {
    (
        request.project_id,
        request.expected_revision,
        Command::Rename {
            element_id: request.element_id,
            name: request.new_name,
        },
    )
}

pub struct MdePlugin;
impl Plugin for MdePlugin {
    type Config = ();
    type Input = ();
    type PrepareError = Infallible;
    type ApplyError = io::Error;
    fn inject(&self) -> InjectSpec {
        InjectSpec::none().require(ProjectService::NAME)
    }
    fn prepare(&self, (): ()) -> Result<Self::Input, Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, ctx: Context, _: &Self::Input) -> Result<(), Self::ApplyError> {
        let projects = ctx
            .try_service::<ProjectService>()
            .map_err(|error| io::Error::other(error.to_string()))?;
        crate::execution_journal::register(&ctx)?;
        crate::headless_operations::register(&ctx)
            .map_err(|error| io::Error::other(error.to_string()))?;
        crate::html_export::register(&ctx).map_err(|error| io::Error::other(error.to_string()))?;
        crate::document_codec::register(&ctx)
            .map_err(|error| io::Error::other(error.to_string()))?;
        crate::projection::register(&ctx, Arc::clone(&projects))?;
        crate::move_preview::register(&ctx, Arc::clone(&projects))?;
        crate::task_events::register(&ctx, Arc::clone(&projects))?;
        crate::diagrams::register(&ctx, Arc::clone(&projects))?;
        crate::views::register(&ctx, Arc::clone(&projects))?;
        crate::view_style::register(&ctx, Arc::clone(&projects))?;
        crate::mdj_interchange::register(&ctx, Arc::clone(&projects))?;
        register_command::<crate::ModelingQuickEdit>(
            &ctx,
            &projects,
            crate::quick_edit_commands::execute_quick_edit_command,
        )?;
        register_command::<crate::ModelingCreateElementOrVisualDraft>(
            &ctx,
            &projects,
            crate::create_element::create_element_or_visual_draft_command,
        )?;
        register_command::<crate::ModelingCreateRelationAtViewsOrVisualDraft>(
            &ctx,
            &projects,
            crate::create_relation::create_relation_at_views_or_visual_draft_command,
        )?;
        register_command::<crate::ModelingCreateRelationOrVisualDraft>(
            &ctx,
            &projects,
            crate::create_relation::create_relation_or_visual_draft_command,
        )?;
        register_command::<crate::ModelingCreateModelElement>(
            &ctx,
            &projects,
            crate::create_element::create_model_element_command,
        )?;
        register_command::<crate::ModelingCreateElement>(
            &ctx,
            &projects,
            crate::create_element::create_element_command,
        )?;
        register_command::<crate::ModelingCreateRelation>(
            &ctx,
            &projects,
            crate::create_relation::create_relation_command,
        )?;
        register_command::<crate::ModelingDeleteElement>(
            &ctx,
            &projects,
            crate::delete_element::delete_element_command,
        )?;
        register_command::<crate::ModelingSetProperty>(
            &ctx,
            &projects,
            crate::set_property::set_property_command,
        )?;
        register_command::<crate::ModelingSetNameExpression>(
            &ctx,
            &projects,
            crate::name_expression::set_name_expression_command,
        )?;
        register_command::<crate::ModelingAddMember>(
            &ctx,
            &projects,
            crate::add_member::add_member_command,
        )?;
        register_command::<crate::ModelingEditAssociationEnds>(
            &ctx,
            &projects,
            crate::association_ends::edit_association_ends_command,
        )?;
        register_command::<crate::ModelingMoveParameter>(
            &ctx,
            &projects,
            crate::add_member::move_parameter_command,
        )?;
        register_command::<crate::ModelingResizeView>(
            &ctx,
            &projects,
            crate::resize_view::resize_view_command,
        )?;
        register_command::<crate::ModelingMoveView>(
            &ctx,
            &projects,
            crate::view_edits::move_view_command,
        )?;
        register_command::<ModelingSetOwner>(&ctx, &projects, set_owner_command)?;
        register_command::<ModelingSetExplorerParent>(
            &ctx,
            &projects,
            set_explorer_parent_command,
        )?;
        register_command::<ModelingRename>(&ctx, &projects, rename_command)?;
        Ok(())
    }
}
