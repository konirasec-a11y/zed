pub use bellatrix_mde_protocol::projection::*;
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use cordis::{Context, Event};
use cordis::event::responder;
use std::{convert::Infallible, sync::Arc};

/// A derived read result, never a document, command or persistence input.
#[derive(Clone, Debug)]
pub struct ModelingSnapshot {
    pub project_id: ProjectId,
    pub name: String,
    pub revision: u64,
    pub section: ModelingProjection,
    document: Arc<ProjectDocument>,
}
impl ModelingSnapshot {
    pub fn matches_document(&self, document: &Arc<ProjectDocument>) -> bool {
        Arc::ptr_eq(&self.document, document)
    }
}
#[derive(Clone)]
pub struct ModelingReadProjectionRequested {
    pub document: Arc<ProjectDocument>,
}
pub struct ModelingReadProjection;
impl Event for ModelingReadProjection {
    const NAME: &'static str = "bellatrix.mde.read-projection.v1";
    type Args = ModelingReadProjectionRequested;
    type Output = Result<ModelingSnapshot, String>;
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), std::io::Error> {
    context.on::<ModelingReadProjection, _>(responder(move |_: Context, request: ModelingReadProjectionRequested| {
        let projects = Arc::clone(&projects);
        async move {
            let result = tokio::task::spawn_blocking(move || {
                let current = projects.snapshot(request.document.id()).map_err(|error| error.to_string())?;
                if !Arc::ptr_eq(&current, &request.document) {
                    return Err("Projection source is no longer the active project snapshot".into());
                }
                let projection = project_document(Arc::clone(&request.document))?;
                let current = projects.snapshot(request.document.id()).map_err(|error| error.to_string())?;
                if !Arc::ptr_eq(&current, &request.document) {
                    return Err("Project changed while preparing its projection".into());
                }
                Ok(projection)
            }).await.map_err(|error| error.to_string()).and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        }
    })).map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(())
}

fn project_document(document: Arc<ProjectDocument>) -> Result<ModelingSnapshot, String> {
    let section = document.section(crate::MODELING_NAMESPACE).ok_or("unsupported or missing MDE section: expected bellatrix.mde")?;
    let section_json = serde_json::to_string(section).map_err(|error| error.to_string())?;
    let section = bellatrix_mde::project_model_section(document.id().as_str(), document.name(), &section_json)?;
    Ok(ModelingSnapshot { project_id: document.id().clone(), name: document.name().to_owned(),
        revision: document.revision(), section, document })
}

pub(crate) fn start_feed(
    context: Context,
    project_id: ProjectId,
    mut changes: tokio::sync::watch::Receiver<u64>,
    session: Option<crate::MdjSession>,
) -> (
    tokio::sync::watch::Receiver<Result<Arc<ModelingSnapshot>, String>>,
    tokio::task::JoinHandle<()>,
) {
    let (sender, receiver) = tokio::sync::watch::channel(Err("Preparing project projection".to_owned()));
    let task = tokio::spawn(async move {
        loop {
            changes.borrow_and_update();
            let document = if let Some(session) = &session {
                session.snapshot()
            } else {
                context.try_service::<ProjectService>().map_err(|error| error.to_string())
                    .and_then(|projects| projects.snapshot(&project_id).map_err(|error| error.to_string()))
            };
            let result = match document {
                Ok(document) => match context.query::<ModelingReadProjection>(cordis::Routing::Unscoped,
                    ModelingReadProjectionRequested { document }).await {
                    Ok(cordis::event::QueryOutcome::Answer(result)) => result.map(Arc::new),
                    Ok(cordis::event::QueryOutcome::Miss) => Err("Project projection responder unavailable".into()),
                    Err(error) => Err(error.to_string()),
                },
                Err(error) => Err(error),
            };
            sender.send_replace(result);
            if changes.changed().await.is_err() { break; }
        }
    });
    (receiver, task)
}
