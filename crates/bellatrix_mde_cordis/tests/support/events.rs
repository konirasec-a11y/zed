use bellatrix_mde_cordis::{
    MODELING_NAMESPACE, MdePlugin, ModelingDocumentFormat, ModelingPrepareDocument,
    ModelingPrepareDocumentRequested, ModelingReadProjection, ModelingReadProjectionRequested,
    ModelingSnapshot,
};
use bellatrix_project::{
    ProjectDocument, ProjectId, ProjectImport, ProjectImportRequested, ProjectPlugin,
    ProjectService,
};
use cordis::{Context, PreparedPlugin, Routing, event::QueryOutcome};
use serde_json::Value;
use std::{error::Error, sync::Arc};

pub async fn prepare(context: &Context, model: &Value) -> Result<ProjectDocument, Box<dyn Error>> {
    let reply = context
        .query::<ModelingPrepareDocument>(
            Routing::Unscoped,
            ModelingPrepareDocumentRequested {
                bytes: serde_json::to_vec(model)?,
                format: ModelingDocumentFormat::ModelJson,
            },
        )
        .await?;
    match reply {
        QueryOutcome::Answer(Ok(prepared)) => Ok(prepared.document),
        QueryOutcome::Answer(Err(error)) => Err(error.into()),
        QueryOutcome::Miss => Err("document preparation responder unavailable".into()),
    }
}

pub async fn projection(
    context: &Context,
    projects: &ProjectService,
    id: &ProjectId,
) -> Result<ModelingSnapshot, Box<dyn Error>> {
    let reply = context
        .query::<ModelingReadProjection>(
            Routing::Unscoped,
            ModelingReadProjectionRequested {
                document: projects.snapshot(id)?,
            },
        )
        .await?;
    match reply {
        QueryOutcome::Answer(Ok(snapshot)) => Ok(snapshot),
        QueryOutcome::Answer(Err(error)) => Err(error.into()),
        QueryOutcome::Miss => Err("read projection responder unavailable".into()),
    }
}

pub fn section(projects: &ProjectService, id: &ProjectId) -> Result<Value, Box<dyn Error>> {
    projects
        .snapshot(id)?
        .section(MODELING_NAMESPACE)
        .cloned()
        .ok_or_else(|| "modeling section unavailable".into())
}

pub fn model_document(document: &ProjectDocument) -> Result<Value, Box<dyn Error>> {
    let mut model = document
        .section(MODELING_NAMESPACE)
        .cloned()
        .ok_or("modeling section unavailable")?;
    let object = model
        .as_object_mut()
        .ok_or("modeling section must be an object")?;
    object.insert("schema_version".into(), 1.into());
    object.insert("id".into(), document.id().as_str().into());
    object.insert("name".into(), document.name().into());
    Ok(model)
}

pub async fn project_document(
    document: ProjectDocument,
) -> Result<ModelingSnapshot, Box<dyn Error>> {
    let context = Context::new();
    let projects = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&projects)),
            Arc::clone(&projects),
        ))
        .await?;
    let modeling = context
        .spawn(PreparedPlugin::from_input(MdePlugin, ()))
        .await?;
    let id = document.id().clone();
    let result = async {
        match context
            .query::<ProjectImport>(
                Routing::Unscoped,
                ProjectImportRequested {
                    document,
                    is_saved: false,
                },
            )
            .await?
        {
            QueryOutcome::Answer(Ok(_)) => projection(&context, &projects, &id).await,
            QueryOutcome::Answer(Err(error)) => Err(Box::<dyn Error>::from(error)),
            QueryOutcome::Miss => Err("project import responder unavailable".into()),
        }
    }
    .await;
    let modeling_disposed = modeling.dispose().await;
    let provider_disposed = provider.dispose().await;
    let snapshot = result?;
    modeling_disposed?;
    provider_disposed?;
    Ok(snapshot)
}
