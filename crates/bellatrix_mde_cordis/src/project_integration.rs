use crate::view_edits::{ModelingMoveView, move_view};
use bellatrix_mde::{Graph, MODELING_NAMESPACE, ModelingSection, Project, commands::Command};
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use cordis::event::responder;
use cordis::{Context, Event, InjectSpec, Plugin, Service};
use std::{convert::Infallible, io, sync::Arc};

pub(crate) fn from_document(document: &ProjectDocument) -> Result<ModelingSection, String> {
    let state = document
        .section(MODELING_NAMESPACE)
        .ok_or("unsupported or missing MDE section: expected bellatrix.mde; beta documents in other namespaces are not supported")?;
    serde_json::from_value(state.clone()).map_err(|error| error.to_string())
}
pub fn with_model(document: ProjectDocument, model: &Project) -> Result<ProjectDocument, String> {
    if document.id().as_str() != model.id || document.name() != model.name {
        return Err("MDE and platform project identities differ".into());
    }
    Graph::new(model)
        .validate()
        .map_err(|error| error.to_string())?;
    let state = serde_json::to_value(ModelingSection::from_model(model))
        .map_err(|error| error.to_string())?;
    document
        .with_section(MODELING_NAMESPACE, state)
        .map_err(|error| error.to_string())
}
#[derive(Clone)]
pub struct ModelingRenameRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub new_name: String,
    pub expected_revision: u64,
}
pub struct ModelingRename;
impl Event for ModelingRename {
    const NAME: &'static str = "bellatrix.mde.rename.v1";
    type Args = ModelingRenameRequested;
    type Output = Result<u64, String>;
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
        crate::task_events::register(&ctx, Arc::clone(&projects))?;
        crate::diagrams::register(&ctx, Arc::clone(&projects))?;
        crate::views::register(&ctx, Arc::clone(&projects))?;
        crate::view_style::register(&ctx, Arc::clone(&projects))?;
        let quick_projects = Arc::clone(&projects);
        let _quick_edit = ctx
            .on::<crate::ModelingQuickEdit, _>(responder(
                move |ctx: Context, request: crate::ModelingQuickEditRequested| {
                    let projects = Arc::clone(&quick_projects);
                    async move {
                        let result = crate::quick_edit_commands::execute_quick_edit(
                            &ctx, &projects, request,
                        )
                        .await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|error| io::Error::other(error.to_string()))?;
        let creation_projects = Arc::clone(&projects);
        let _creation = ctx.on::<crate::create_element::ModelingCreateElement, _>(responder(
            move |ctx: Context, request: crate::create_element::ModelingCreateElementRequested| {
                let projects = Arc::clone(&creation_projects);
                async move {
                    let result = crate::create_element::create_element(&ctx, &projects, request).await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        )).map_err(|error| io::Error::other(error.to_string()))?;
        let link_projects = Arc::clone(&projects);
        let _create_relation = ctx.on::<crate::create_relation::ModelingCreateRelation, _>(responder(
            move |ctx: Context, request: crate::create_relation::ModelingCreateRelationRequested| {
                let projects = Arc::clone(&link_projects);
                async move {
                    let result = crate::create_relation::create_relation(&ctx, &projects, request).await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        )).map_err(|error| io::Error::other(error.to_string()))?;
        let delete_projects = Arc::clone(&projects);
        let _deletion = ctx.on::<crate::delete_element::ModelingDeleteElement, _>(responder(
            move |ctx: Context, request: crate::delete_element::ModelingDeleteElementRequested| {
                let projects = Arc::clone(&delete_projects);
                async move {
                    let result = crate::delete_element::delete_element(&ctx, &projects, request).await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        )).map_err(|error| io::Error::other(error.to_string()))?;
        let property_projects = Arc::clone(&projects);
        let _property_edit = ctx
            .on::<crate::set_property::ModelingSetProperty, _>(responder(
                move |ctx: Context, request: crate::set_property::ModelingSetPropertyRequested| {
                    let projects = Arc::clone(&property_projects);
                    async move {
                        let result =
                            crate::set_property::set_property(&ctx, &projects, request).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|error| io::Error::other(error.to_string()))?;
        let names_projects = Arc::clone(&projects);
        let _name_expression = ctx.on::<crate::name_expression::ModelingSetNameExpression, _>(responder(
            move |ctx: Context, request: crate::name_expression::ModelingSetNameExpressionRequested| {
                let projects = Arc::clone(&names_projects);
                async move {
                    let result = crate::name_expression::set_name_expression(&ctx, &projects, request).await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        )).map_err(|error| io::Error::other(error.to_string()))?;
        let member_projects = Arc::clone(&projects);
        let _add_member = ctx
            .on::<crate::add_member::ModelingAddMember, _>(responder(
                move |ctx: Context, request: crate::add_member::ModelingAddMemberRequested| {
                    let projects = Arc::clone(&member_projects);
                    async move {
                        let result = crate::add_member::add_member(&ctx, &projects, request).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|error| io::Error::other(error.to_string()))?;
        let resizing_projects = Arc::clone(&projects);
        let _resize = ctx
            .on::<crate::resize_view::ModelingResizeView, _>(responder(
                move |ctx: Context, request: crate::resize_view::ModelingResizeViewRequested| {
                    let projects = Arc::clone(&resizing_projects);
                    async move {
                        let result =
                            crate::resize_view::resize_view(&ctx, &projects, request).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|error| io::Error::other(error.to_string()))?;
        let movement_projects = Arc::clone(&projects);
        let _movement = ctx
            .on::<ModelingMoveView, _>(responder(
                move |ctx: Context, request: crate::view_edits::ModelingMoveViewRequested| {
                    let projects = Arc::clone(&movement_projects);
                    async move {
                        let result = move_view(&ctx, &projects, request).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|error| io::Error::other(error.to_string()))?;
        let _listener = ctx
            .on::<ModelingRename, _>(responder(
                move |ctx: Context, request: ModelingRenameRequested| {
                    let projects = Arc::clone(&projects);
                    async move {
                        let result = rename(&ctx, &projects, request).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|error| io::Error::other(error.to_string()))?;
        Ok(())
    }
}
async fn rename(
    context: &Context,
    projects: &ProjectService,
    request: ModelingRenameRequested,
) -> Result<u64, String> {
    crate::mutation::execute(
        context,
        projects,
        request.project_id,
        request.expected_revision,
        Command::Rename {
            element_id: request.element_id,
            name: request.new_name,
        },
    )
    .await
}

#[cfg(test)]
mod namespace_tests {
    use super::*;

    #[test]
    fn obsolete_namespaces_are_rejected_without_creating_or_migrating_state()
    -> Result<(), Box<dyn std::error::Error>> {
        for namespace in ["bellatrix.bellatrix-mde-modeling", "bellatrix.modeling"] {
            let state = serde_json::json!({"elements": [], "relations": [], "diagrams": []});
            let document =
                ProjectDocument::new(ProjectId::new("old-beta-document")?, "Old beta document")?
                    .with_section(namespace, state.clone())?;
            assert!(
                matches!(from_document(&document), Err(error) if error.contains("not supported"))
            );
            assert_eq!(document.section(namespace), Some(&state));
            assert!(document.section(MODELING_NAMESPACE).is_none());
        }
        Ok(())
    }
}
