//! Engine-MDE consumes Cordis services and exposes public, typed bellatrix-mde-modeling operations.
use crate::view_edits::{ModelingMoveView, move_view};
use bellatrix_mde_model::{ModelingSection, Project};
use bellatrix_mde_modeling::Graph;
use bellatrix_project::{
    ProjectDocument, ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService,
};
use cordis::event::{QueryOutcome, responder};
use cordis::{Context, Event, InjectSpec, Plugin, Routing, Service};
use std::{convert::Infallible, io, sync::Arc};

pub const MODELING_NAMESPACE: &str = "bellatrix.bellatrix-mde-modeling";
pub const LEGACY_MODELING_NAMESPACE: &str = "bellatrix.modeling";

pub(crate) fn namespace_for(document: &ProjectDocument) -> &'static str {
    if document.section(MODELING_NAMESPACE).is_some() { MODELING_NAMESPACE } else { LEGACY_MODELING_NAMESPACE }
}

pub(crate) fn from_document(document: &ProjectDocument) -> Result<ModelingSection, String> {
    let state = document
        .section(MODELING_NAMESPACE)
        .ok_or("bellatrix-project has no Modeling section")?;
    serde_json::from_value(state.clone()).map_err(|e| e.to_string())
}

pub fn with_model(document: ProjectDocument, model: &Project) -> Result<ProjectDocument, String> {
    if document.id().as_str() != model.id || document.name() != model.name {
        return Err("bellatrix-mde-model and platform bellatrix-project identities differ".into());
    }
    Graph::new(model).validate().map_err(|e| e.to_string())?;
    let state =
        serde_json::to_value(ModelingSection::from_model(model)).map_err(|e| e.to_string())?;
    document
        .with_section(MODELING_NAMESPACE, state)
        .map_err(|e| e.to_string())
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
    const NAME: &'static str = "bellatrix.bellatrix-mde-modeling.rename.v1";
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
            .map_err(|e| io::Error::other(e.to_string()))?;
        crate::task_events::register(&ctx, Arc::clone(&projects))?;
        let creation_projects = Arc::clone(&projects);
        let _creation = ctx
            .on::<crate::create_element::ModelingCreateElement, _>(responder(
                move |ctx: Context, req: crate::create_element::ModelingCreateElementRequested| {
                    let projects = Arc::clone(&creation_projects);
                    async move {
                        let result = crate::create_element::create_element(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let link_projects = Arc::clone(&projects);
        let _create_relation = ctx
            .on::<crate::create_relation::ModelingCreateRelation, _>(responder(
                move |ctx: Context, req: crate::create_relation::ModelingCreateRelationRequested| {
                    let projects = Arc::clone(&link_projects);
                    async move {
                        let result = crate::create_relation::create_relation(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let delete_projects = Arc::clone(&projects);
        let _deletion = ctx
            .on::<crate::delete_element::ModelingDeleteElement, _>(responder(
                move |ctx: Context, req: crate::delete_element::ModelingDeleteElementRequested| {
                    let projects = Arc::clone(&delete_projects);
                    async move {
                        let result = crate::delete_element::delete_element(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let property_projects = Arc::clone(&projects);
        let _property_edit = ctx
            .on::<crate::set_property::ModelingSetProperty, _>(responder(
                move |ctx: Context, req: crate::set_property::ModelingSetPropertyRequested| {
                    let projects = Arc::clone(&property_projects);
                    async move {
                        let result = crate::set_property::set_property(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let interface_projects = Arc::clone(&projects);
        let _interface_quick_edit = ctx
            .on::<crate::interface_quick_edit::ModelingInterfaceQuickEdit, _>(responder(
                move |ctx: Context, req: crate::interface_quick_edit::ModelingInterfaceQuickEditRequested| {
                    let projects = Arc::clone(&interface_projects);
                    async move {
                        let result = crate::interface_quick_edit::execute_quick_edit(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let member_projects = Arc::clone(&projects);
        let _add_member = ctx
            .on::<crate::add_member::ModelingAddMember, _>(responder(
                move |ctx: Context, req: crate::add_member::ModelingAddMemberRequested| {
                    let projects = Arc::clone(&member_projects);
                    async move {
                        let result = crate::add_member::add_member(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let resizing_projects = Arc::clone(&projects);
        let _resize = ctx
            .on::<crate::resize_view::ModelingResizeView, _>(responder(
                move |ctx: Context, req: crate::resize_view::ModelingResizeViewRequested| {
                    let projects = Arc::clone(&resizing_projects);
                    async move {
                        let result = crate::resize_view::resize_view(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let movement_projects = Arc::clone(&projects);
        let _movement = ctx
            .on::<ModelingMoveView, _>(responder(
                move |ctx, req: crate::view_edits::ModelingMoveViewRequested| {
                    let projects = Arc::clone(&movement_projects);
                    async move {
                        let result = move_view(&ctx, &projects, req).await;
                        Ok::<_, Infallible>(Some(result))
                    }
                },
            ))
            .map_err(|e| io::Error::other(e.to_string()))?;
        let _listener = ctx
            .on::<ModelingRename, _>(responder(move |ctx, req: ModelingRenameRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let result = rename(&ctx, &projects, req).await;
                    Ok::<_, Infallible>(Some(result))
                }
            }))
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(())
    }
}

async fn rename(
    ctx: &Context,
    projects: &ProjectService,
    request: ModelingRenameRequested,
) -> Result<u64, String> {
    if request.new_name.trim().is_empty() {
        return Err("name must not be blank".into());
    }
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|e| e.to_string())?;
    if document.revision() != request.expected_revision {
        return Err("stale Modeling edit; reload bellatrix-project snapshot".into());
    }
    let mut section = from_document(&document)?;
    let namespace = namespace_for(&document);
    let element = section
        .elements
        .iter_mut()
        .find(|e| e.id == request.element_id)
        .ok_or("bellatrix-mde-model element not found")?;
    element.name = request.new_name;
    Graph::new(&section.to_model(document.id().as_str(), document.name()))
        .validate()
        .map_err(|e| e.to_string())?;
    let value = serde_json::to_value(section).map_err(|e| e.to_string())?;
    let answer = ctx
        .query::<ProjectReplace>(
            Routing::Unscoped,
            ProjectReplaceRequested {
                id: request.project_id,
                namespace: namespace.into(),
                expected_revision: request.expected_revision,
                state: value,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    match answer {
        QueryOutcome::Answer(Ok(doc)) => Ok(doc.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService is unavailable".into()),
    }
}
