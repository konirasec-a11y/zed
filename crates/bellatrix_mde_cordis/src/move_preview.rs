pub use bellatrix_mde_contracts::events::{ModelingPreviewMoveRequested, ModelingPreviewMove};
use crate::command_contract::ViewOccurrence;
use bellatrix_project::{ProjectDocument, ProjectService};
use cordis::event::{QueryOutcome, responder};
use cordis::{Context, Event, Routing};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tokio::sync::watch;

pub const MOVE_PREVIEW_PENDING: &str = "Checking this move with Cordis";

pub(crate) fn register(
    context: &Context,
    projects: Arc<ProjectService>,
) -> Result<(), std::io::Error> {
    context
        .on::<ModelingPreviewMove, _>(responder(
            move |_: Context, request: ModelingPreviewMoveRequested| {
                let projects = Arc::clone(&projects);
                async move {
                    let result = tokio::task::spawn_blocking(move || {
                        let current = projects
                            .snapshot(request.document.id())
                            .map_err(|error| error.to_string())?;
                        if !Arc::ptr_eq(&current, &request.document) {
                            return Err(
                                "Move preview source is no longer the active project snapshot"
                                    .into(),
                            );
                        }
                        let result = preview(&request);
                        let current = projects
                            .snapshot(request.document.id())
                            .map_err(|error| error.to_string())?;
                        if !Arc::ptr_eq(&current, &request.document) {
                            return Err("Project changed while checking this move".into());
                        }
                        result
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

fn preview(request: &ModelingPreviewMoveRequested) -> Result<(), String> {
    let command = if request.diagram {
        crate::Command::SetExplorerParent {
            subject_id: request.subject_id.clone(),
            parent_id: request.parent_id.clone(),
            parent_occurrence: request.parent_occurrence.clone(),
        }
    } else {
        crate::Command::SetOwner {
            element_id: request.subject_id.clone(),
            owner_id: request.parent_id.clone(),
        }
    };
    let section_json = request
        .document
        .section(crate::MODELING_NAMESPACE)
        .ok_or("MDE section is missing")?
        .to_string();
    bellatrix_mde::apply_model_command(
        request.document.id().as_str(),
        request.document.name(),
        &section_json,
        request.document.revision(),
        command,
    )
    .map(|_| ())
}

#[derive(Clone, PartialEq, Eq)]
struct MoveIntent {
    subject_id: String,
    parent_id: Option<String>,
    parent_occurrence: Option<ViewOccurrence>,
    diagram: bool,
}

#[derive(Default)]
struct MovePreviewCache {
    document: Option<Arc<ProjectDocument>>,
    revision: u64,
    generation: u64,
    completed: Vec<(MoveIntent, Result<(), String>)>,
    pending: Vec<MoveIntent>,
}

impl MovePreviewCache {
    fn set_document(&mut self, document: Arc<ProjectDocument>) {
        if self.matches_document(&document) {
            return;
        }
        self.revision = document.revision();
        self.document = Some(document);
        self.generation = self.generation.wrapping_add(1);
        self.completed.clear();
        self.pending.clear();
    }

    fn matches_document(&self, document: &Arc<ProjectDocument>) -> bool {
        self.revision == document.revision()
            && self
                .document
                .as_ref()
                .is_some_and(|cached| Arc::ptr_eq(cached, document))
    }
}

#[derive(Clone)]
pub struct MovePreviewClient {
    context: Context,
    runtime: tokio::runtime::Handle,
    cache: Arc<Mutex<MovePreviewCache>>,
    changes: watch::Sender<u64>,
}

impl MovePreviewClient {
    pub(crate) fn new(context: Context, runtime: tokio::runtime::Handle) -> Self {
        let (changes, _) = watch::channel(0);
        Self {
            context,
            runtime,
            cache: Arc::new(Mutex::new(MovePreviewCache::default())),
            changes,
        }
    }

    pub fn changes(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }

    pub fn set_document(&self, document: Arc<ProjectDocument>) -> Result<(), String> {
        self.cache
            .lock()
            .map_err(|error| error.to_string())?
            .set_document(document);
        Ok(())
    }

    pub fn check(&self, request: ModelingPreviewMoveRequested) -> Option<Result<(), String>> {
        let intent = MoveIntent {
            subject_id: request.subject_id.clone(),
            parent_id: request.parent_id.clone(),
            parent_occurrence: request.parent_occurrence.clone(),
            diagram: request.diagram,
        };
        let generation = {
            let mut cache = match self.cache.lock() {
                Ok(cache) => cache,
                Err(error) => return Some(Err(error.to_string())),
            };
            cache.set_document(Arc::clone(&request.document));
            if let Some((_, result)) = cache
                .completed
                .iter()
                .find(|(candidate, _)| candidate == &intent)
            {
                return Some(result.clone());
            }
            if cache.pending.contains(&intent) {
                return None;
            }
            cache.pending.push(intent.clone());
            cache.generation
        };
        let context = self.context.clone();
        let cache = Arc::clone(&self.cache);
        let changes = self.changes.clone();
        let _task = self.runtime.spawn(async move {
            let document = Arc::clone(&request.document);
            let result = match context
                .query::<ModelingPreviewMove>(Routing::Unscoped, request)
                .await
            {
                Ok(QueryOutcome::Answer(result)) => result,
                Ok(QueryOutcome::Miss) => Err("Cordis move preview responder unavailable".into()),
                Err(error) => Err(format!("Cordis move preview failed: {error}")),
            };
            match cache.lock() {
                Ok(mut cache) => {
                    if cache.generation != generation || !cache.matches_document(&document) {
                        return;
                    }
                    cache.pending.retain(|candidate| candidate != &intent);
                    cache.completed.push((intent, result));
                }
                Err(error) => {
                    eprintln!("MDE move preview cache unavailable: {error}");
                }
            }
            changes.send_modify(|version| *version = version.wrapping_add(1));
        });
        None
    }
}
