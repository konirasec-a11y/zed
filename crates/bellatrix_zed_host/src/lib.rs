//! Bellatrix services embedded in Zed's existing application lifecycle.
//! No GPUI Kit, separate Workspace, or parallel event bus is created here.
use bellatrix_mde::Project;
use bellatrix_mde_cordis::{MdePlugin, MdeUiBridge, with_model};
use bellatrix_mde_workbench::{
    MdeUiSource, NativeDocumentImporter, NativeDocumentLease, from_document, register_mde_workspace,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectOpen,
    ProjectOpenRequested, ProjectPlugin, ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context as CordisContext, FiberHandle, Plugin, PreparedPlugin, Routing};
use gpui::{App, Global};
use std::{error::Error, io, path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

struct PreviewListener(Option<cordis::event::ListenerRegistration>);
impl Drop for PreviewListener {
    fn drop(&mut self) {
        if let Some(listener) = self.0.take() {
            listener.remove();
        }
    }
}

pub struct BellatrixZedHost {
    runtime: Runtime,
    _context: CordisContext,
    _projects: Arc<ProjectService>,
    bridge: Option<MdeUiBridge>,
    provider: Option<FiberHandle>,
    mde: Option<FiberHandle>,
}
impl Global for BellatrixZedHost {}
impl BellatrixZedHost {
    fn native_document_importer(&self) -> NativeDocumentImporter {
        let runtime = self.runtime.handle().clone();
        let context = self._context.clone();
        let projects = Arc::clone(&self._projects);
        NativeDocumentImporter::new(move |document, is_saved, editable_mdj| {
            let runtime = runtime.clone();
            let context = context.clone();
            let projects = Arc::clone(&projects);
            Box::pin(async move {
                let cleanup_runtime = runtime.clone();
                // The Tokio task owns a successfully imported lease even when
                // its GPUI receiver disappears while the request is in flight.
                runtime.spawn(async move {
                    let snapshot = from_document(&document)?;
                    let watched_id = document.id().clone();
                    let closed_id = watched_id.clone();
                    let (revision_tx, revisions) = tokio::sync::watch::channel(document.revision());
                    let closed_tx = revision_tx.clone();
                    let changed_listener = context.on::<bellatrix_project::ProjectStateChangedEvent, _>(cordis::event::observer_sync(
                        move |_, changed: bellatrix_project::ProjectStateChanged| {
                            if changed.id == watched_id { revision_tx.send_replace(changed.revision); }
                            Ok::<_, std::convert::Infallible>(())
                        },
                    )).map_err(|error| error.to_string())?;
                    let changed_listener = PreviewListener(Some(changed_listener));
                    let closed_listener = context.on::<bellatrix_project::ProjectClosedEvent, _>(cordis::event::observer_sync(
                        move |_, closed: bellatrix_project::ProjectClosed| {
                            if closed.id == closed_id { closed_tx.send_replace(u64::MAX); }
                            Ok::<_, std::convert::Infallible>(())
                        },
                    )).map_err(|error| error.to_string())?;
                    let closed_listener = PreviewListener(Some(closed_listener));
                    let imported = match context.query::<bellatrix_project::ProjectImport>(
                        Routing::Unscoped, bellatrix_project::ProjectImportRequested { document, is_saved },
                    ).await.map_err(|error| error.to_string())? {
                        QueryOutcome::Answer(Ok(document)) => document,
                        QueryOutcome::Answer(Err(error @ bellatrix_project::ProjectError::AlreadyOpen(_))) => return Err(format!("Cannot open MDE preview: {error}. Close the existing MDE tab before reopening a saved copy with this identity. A configured/demo source may also own this identity.")),
                        QueryOutcome::Answer(Err(error)) => return Err(format!("Cannot open MDE preview: {error}")),
                        QueryOutcome::Miss => return Err("Project import service is unavailable".into()),
                    };
                    if editable_mdj {
                        let bridge = match MdeUiBridge::start_mdj(&context, Arc::clone(&imported)) {
                            Ok(bridge) => bridge,
                            Err(error) => {
                                match context.query::<bellatrix_project::ProjectCloseSnapshot>(Routing::Unscoped,
                                    bellatrix_project::ProjectCloseSnapshotRequested { document: Arc::clone(&imported) }).await {
                                    Ok(QueryOutcome::Answer(Ok(()))) => {},
                                    result => eprintln!("MDJ failed-start cleanup retained state: {result:?}"),
                                }
                                return Err(error.to_string());
                            }
                        };
                        let session = bridge.mdj_session().ok_or("MDJ session was not created")?;
                        let source = MdeUiSource::from_bridge(Arc::clone(&projects), &bridge);
                        let save_context = context.clone();
                        let save_runtime = cleanup_runtime.clone();
                        let save_lock = Arc::new(tokio::sync::Mutex::new(()));
                        let cleanup_lock = Arc::clone(&save_lock);
                        return Ok(NativeDocumentLease::new(source, move || {
                            drop(changed_listener);
                            drop(closed_listener);
                            drop(bridge);
                            cleanup_runtime.spawn(async move {
                                if let Err(error) = session.wait_closed().await {
                                    eprintln!("MDJ cleanup retained state: {error}");
                                    return;
                                }
                                let _save_guard = cleanup_lock.lock().await;
                                let document = match session.snapshot() {
                                    Ok(document) => document,
                                    Err(error) => { eprintln!("MDJ cleanup retained state: {error}"); return; }
                                };
                                match context.query::<bellatrix_project::ProjectCloseSnapshot>(Routing::Unscoped,
                                    bellatrix_project::ProjectCloseSnapshotRequested { document }).await {
                                    Ok(QueryOutcome::Answer(Ok(()))) => {},
                                    result => eprintln!("MDJ cleanup did not close its snapshot: {result:?}"),
                                }
                            });
                        }).with_save_copy(move |document, path| {
                            let context = save_context.clone();
                            let runtime = save_runtime.clone();
                            let save_lock = Arc::clone(&save_lock);
                            Box::pin(async move {
                                let outcome_path = path.clone();
                                runtime.spawn(async move {
                                    let _save_guard = save_lock.lock().await;
                                    let exported = match context.query::<bellatrix_mde_cordis::ModelingExportMdj>(Routing::Unscoped,
                                        bellatrix_mde_cordis::ModelingExportMdjRequested { document: Arc::clone(&document) }).await.map_err(|error| error.to_string())? {
                                        QueryOutcome::Answer(result) => result?,
                                        QueryOutcome::Miss => return Err("MDJ export service unavailable".into()),
                                    };
                                    if !exported.matches_snapshot(&document) { return Err("MDJ export snapshot changed".into()); }
                                    let ticket = match context.query::<bellatrix_project::ProjectPrepareEncodedSave>(Routing::Unscoped,
                                        exported.prepare_save(path.clone())).await.map_err(|error| error.to_string())? {
                                        QueryOutcome::Answer(result) => result.map_err(|error| error.to_string())?,
                                        QueryOutcome::Miss => return Err("MDJ save preparation service unavailable".into()),
                                    };
                                    let published = match context.query::<bellatrix_project::ProjectPublishNewFile>(Routing::Unscoped,
                                        bellatrix_project::ProjectPublishNewFileRequested { ticket }).await.map_err(|error| error.to_string())? {
                                        QueryOutcome::Answer(result) => result.map_err(|error| error.to_string())?,
                                        QueryOutcome::Miss => return Err("MDJ publication service unavailable".into()),
                                    };
                                    if !Arc::ptr_eq(published.document(), &document) { return Err(format!("Copy published to {}, but publication identity differed; document remains dirty", path.display())); }
                                    let warning = published.cleanup_warning().map(str::to_owned);
                                    if let Some(warning) = &warning { eprintln!("MDJ copy {}: {warning}", published.path().display()); }
                                    match context.query::<bellatrix_project::ProjectAcknowledgeExternalSave>(Routing::Unscoped,
                                        published.acknowledge()).await {
                                        Ok(QueryOutcome::Answer(Ok(_))) => Ok(warning),
                                        result => Err(format!("Copy written to {}, but acknowledgement failed and the document remains dirty: {result:?}", path.display())),
                                    }
                                }).await.map_err(|error| format!("MDJ copy task failed for {}: {error}. Publication outcome is uncertain; inspect this destination before retrying.", outcome_path.display()))?
                            })
                        }));
                    }
                    let mut source = MdeUiSource::read_only(Arc::clone(&projects), Arc::clone(&imported), snapshot);
                    source.revisions = revisions;
                    Ok(NativeDocumentLease::new(source, move || {
                        drop(changed_listener);
                        drop(closed_listener);
                        cleanup_runtime.spawn(async move {
                            match context.query::<bellatrix_project::ProjectCloseSnapshot>(
                                Routing::Unscoped,
                                bellatrix_project::ProjectCloseSnapshotRequested { document: imported },
                            ).await {
                                Ok(QueryOutcome::Answer(Ok(()))) => {},
                                Ok(QueryOutcome::Answer(Err(error))) => eprintln!("MDE preview cleanup retained state: {error}"),
                                Ok(QueryOutcome::Miss) => eprintln!("MDE preview cleanup service unavailable"),
                                Err(error) => eprintln!("MDE preview cleanup failed: {error}"),
                            }
                        });
                    }))
                }).await.map_err(|error| error.to_string())?
            })
        })
    }

    fn start() -> Result<(Self, MdeUiSource), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let context = CordisContext::new();
        let projects = Arc::new(ProjectService::new());
        let (provider, mde, project_id) = runtime.block_on(async {
            let provider_plugin = ProjectPlugin::new(Arc::clone(&projects));
            let provider_input = provider_plugin.prepare(())?;
            let provider = context
                .spawn(PreparedPlugin::from_input(provider_plugin, provider_input))
                .await?;
            let mde_plugin = MdePlugin;
            mde_plugin.prepare(())?;
            let mde = match context
                .spawn(PreparedPlugin::from_input(mde_plugin, ()))
                .await
            {
                Ok(fiber) => fiber,
                Err(error) => {
                    let _ = provider.dispose().await;
                    return Err(Box::<dyn Error>::from(error));
                }
            };
            let opened = async {
                match std::env::var_os("BELLATRIX_PROJECT_FILE") {
                    Some(path) => context
                        .query::<ProjectOpen>(
                            Routing::Unscoped,
                            ProjectOpenRequested {
                                path: PathBuf::from(path),
                            },
                        )
                        .await
                        .map_err(Box::<dyn Error>::from),
                    None => {
                        let model: Project = match std::env::var_os("BELLATRIX_MODEL_FILE") {
                            Some(path) => serde_json::from_slice(&std::fs::read(path)?)?,
                            None => serde_json::from_str(include_str!(
                                "../../bellatrix_mde/fixtures/todo.json"
                            ))?,
                        };
                        let id = ProjectId::new(model.id.clone())?;
                        let document = ProjectDocument::new(id, model.name.clone())?;
                        let document = with_model(document, &model).map_err(io::Error::other)?;
                        context
                            .query::<ProjectCreate>(
                                Routing::Unscoped,
                                ProjectCreateRequested { document },
                            )
                            .await
                            .map_err(Box::<dyn Error>::from)
                    }
                }
            }
            .await;
            let result: Result<ProjectId, Box<dyn Error>> = match opened {
                Ok(QueryOutcome::Answer(Ok(document))) => Ok(document.id().clone()),
                Ok(QueryOutcome::Answer(Err(error))) => Err(error.into()),
                Ok(QueryOutcome::Miss) => Err(io::Error::other("ProjectService missing").into()),
                Err(error) => Err(error),
            };
            match result {
                Ok(id) => Ok::<_, Box<dyn Error>>((provider, mde, id)),
                Err(error) => {
                    let _ = mde.dispose().await;
                    let _ = provider.dispose().await;
                    Err(error)
                }
            }
        })?;
        let bridge = {
            let _guard = runtime.enter();
            MdeUiBridge::start(&context, project_id)?
        };
        let source = MdeUiSource::from_bridge(Arc::clone(&projects), &bridge);
        Ok((
            Self {
                runtime,
                _context: context,
                _projects: projects,
                bridge: Some(bridge),
                provider: Some(provider),
                mde: Some(mde),
            },
            source,
        ))
    }
}
impl Drop for BellatrixZedHost {
    fn drop(&mut self) {
        self.bridge.take();
        let mde = self.mde.take();
        let provider = self.provider.take();
        if let (Some(mde), Some(provider)) = (mde, provider) {
            let result = self.runtime.block_on(async move {
                let mde_result = mde.dispose().await;
                let project_result = provider.dispose().await;
                (mde_result, project_result)
            });
            if let (Err(error), _) = &result {
                eprintln!("Bellatrix MDE disposal failed: {error}");
            }
            if let (_, Err(error)) = &result {
                eprintln!("Bellatrix Project disposal failed: {error}");
            }
        }
    }
}
/// Called once after Zed's editor/workspace initialization and before its first window.
pub fn install(cx: &mut App) -> Result<(), Box<dyn Error>> {
    let (host, source) = BellatrixZedHost::start()?;
    let importer = host.native_document_importer();
    register_mde_workspace(cx, source, importer);
    cx.set_global(host);
    Ok(())
}
