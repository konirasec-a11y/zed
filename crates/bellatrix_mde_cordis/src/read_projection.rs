pub use bellatrix_mde_contracts::events::{
    ModelingReadProjection, ModelingReadProjectionRequested, ModelingSnapshot,
};
pub use bellatrix_mde_contracts::protocol::projection::*;
use bellatrix_project::{ProjectDocument, ProjectId, ProjectService};
use cordis::event::responder;
use cordis::{Context, Event};
use std::{convert::Infallible, sync::Arc};

pub(crate) fn register(
    context: &Context,
    projects: Arc<ProjectService>,
) -> Result<(), std::io::Error> {
    context
        .on::<ModelingReadProjection, _>(responder(
            move |_: Context, request: ModelingReadProjectionRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let result = tokio::task::spawn_blocking(move || {
                        let current = projects
                            .snapshot(request.document.id())
                            .map_err(|error| error.to_string())?;
                        if !Arc::ptr_eq(&current, &request.document) {
                            return Err(
                                "Projection source is no longer the active project snapshot".into(),
                            );
                        }
                        let projection = project_document(Arc::clone(&request.document))?;
                        let current = projects
                            .snapshot(request.document.id())
                            .map_err(|error| error.to_string())?;
                        if !Arc::ptr_eq(&current, &request.document) {
                            return Err("Project changed while preparing its projection".into());
                        }
                        Ok(projection)
                    })
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result);
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(())
}

fn project_document(document: Arc<ProjectDocument>) -> Result<ModelingSnapshot, String> {
    let section = document
        .section(crate::MODELING_NAMESPACE)
        .ok_or("unsupported or missing MDE section: expected bellatrix.mde")?;
    let section_json = serde_json::to_string(section).map_err(|error| error.to_string())?;
    let section = bellatrix_mde::project_model_section(
        document.id().as_str(),
        document.name(),
        &section_json,
    )?;
    Ok(ModelingSnapshot::new(document, section))
}

pub(crate) fn start_feed(
    context: Context,
    mut changes: tokio::sync::watch::Receiver<u64>,
    session: crate::MdeSession,
) -> (
    tokio::sync::watch::Receiver<Result<Arc<ModelingSnapshot>, String>>,
    tokio::task::JoinHandle<()>,
) {
    let (sender, receiver) =
        tokio::sync::watch::channel(Err("Preparing project projection".to_owned()));
    let task = tokio::spawn(async move {
        loop {
            changes.borrow_and_update();
            let document = session.snapshot();
            let result = match document {
                Ok(document) => {
                    let result = match context
                        .query::<ModelingReadProjection>(
                            cordis::Routing::Unscoped,
                            ModelingReadProjectionRequested {
                                document: Arc::clone(&document),
                            },
                        )
                        .await
                    {
                        Ok(cordis::event::QueryOutcome::Answer(result)) => result.map(Arc::new),
                        Ok(cordis::event::QueryOutcome::Miss) => {
                            Err("Project projection responder unavailable".into())
                        }
                        Err(error) => Err(error.to_string()),
                    };
                    match session.snapshot() {
                        // A superseded result, including a stale-projection error,
                        // must not replace the frame while the latest one is built.
                        Ok(current) if !Arc::ptr_eq(&current, &document) => continue,
                        Ok(_) => result,
                        Err(error) => Err(error),
                    }
                }
                Err(error) => Err(error),
            };
            sender.send_replace(result);
            if changes.changed().await.is_err() {
                break;
            }
        }
    });
    (receiver, task)
}
