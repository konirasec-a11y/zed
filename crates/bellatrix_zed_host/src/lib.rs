//! Bellatrix services embedded in Zed's existing application lifecycle.
//! No GPUI Kit, separate Workspace, or parallel event bus is created here.
use bellatrix_mde_cordis::{MdePlugin, MdeUiBridge};
use bellatrix_mde_ui::{
    MdeUiSource, NativeDocumentImporter, NativeDocumentLease, register_mde_workspace,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectId, ProjectOpen, ProjectOpenRequested,
    ProjectPlugin, ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context as CordisContext, FiberHandle, Plugin, PreparedPlugin, Routing};
use gpui::{App, Global};
use std::{error::Error, io, path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

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
        let prepare_runtime = runtime.clone();
        let prepare_context = context.clone();
        let projects = Arc::clone(&self._projects);
        NativeDocumentImporter::new(
            move |bytes, is_mdj| {
                let runtime = prepare_runtime.clone();
                let context = prepare_context.clone();
                Box::pin(async move {
                    runtime
                        .spawn(async move {
                            let format = if is_mdj {
                                bellatrix_mde_cordis::ModelingDocumentFormat::StarUml
                            } else {
                                bellatrix_mde_cordis::ModelingDocumentFormat::ProjectJson
                            };
                            match context
                                .query::<bellatrix_mde_cordis::ModelingPrepareDocument>(
                                    Routing::Unscoped,
                                    bellatrix_mde_cordis::ModelingPrepareDocumentRequested {
                                        bytes,
                                        format,
                                    },
                                )
                                .await
                                .map_err(|error| error.to_string())?
                            {
                                QueryOutcome::Answer(result) => {
                                    result.map(|prepared| (prepared.document, prepared.diagnostics))
                                }
                                QueryOutcome::Miss => {
                                    Err("MDE document preparation service unavailable".into())
                                }
                            }
                        })
                        .await
                        .map_err(|error| error.to_string())?
                })
            },
            move |document, is_saved| {
                let runtime = runtime.clone();
                let context = context.clone();
                let projects = Arc::clone(&projects);
                Box::pin(async move {
                    let cleanup_runtime = runtime.clone();
                    runtime.spawn(async move {
                        let imported = match context.query::<bellatrix_project::ProjectImport>(
                            Routing::Unscoped,
                            bellatrix_project::ProjectImportRequested { document, is_saved },
                        ).await.map_err(|error| error.to_string())? {
                            QueryOutcome::Answer(Ok(document)) => document,
                            QueryOutcome::Answer(Err(error)) => return Err(format!("Cannot open Bellatrix document: {error}. An existing tab may already own this project identity.")),
                            QueryOutcome::Miss => return Err("Project import service unavailable".into()),
                        };
                        let prepared = context.query::<bellatrix_mde_cordis::ModelingReadProjection>(
                            Routing::Unscoped,
                            bellatrix_mde_cordis::ModelingReadProjectionRequested { document: Arc::clone(&imported) },
                        ).await;
                        let bridge = match prepared {
                            Ok(QueryOutcome::Answer(Ok(_))) => MdeUiBridge::start(&context, imported.id().clone()).map_err(|error| error.to_string()),
                            result => Err(format!("Cannot prepare the native document projection: {result:?}")),
                        };
                        let bridge = match bridge {
                            Ok(bridge) => bridge,
                            Err(error) => {
                                if let Err(cleanup) = close_snapshot(&context, imported).await {
                                    eprintln!("Document opening cleanup retained state: {cleanup}");
                                }
                                return Err(error);
                            }
                        };
                        let session = bridge.session();
                        let save_journal = bridge.execution_journal();
                        let export_journal = bridge.execution_journal();
                        save_journal.record_result(imported.id().clone(), imported.revision(), "document.open", &Ok::<(), String>(()));
                        let source = MdeUiSource::from_bridge(projects, &bridge);
                        let save_context = context.clone();
                        let export_context = context.clone();
                        let save_runtime = cleanup_runtime.clone();
                        let export_runtime = cleanup_runtime.clone();
                        let io_lock = Arc::new(tokio::sync::Mutex::new(()));
                        let save_lock = Arc::clone(&io_lock);
                        let export_lock = Arc::clone(&io_lock);
                        Ok(NativeDocumentLease::new(source, move || {
                            drop(bridge);
                            cleanup_runtime.spawn(async move {
                                if let Err(error) = session.wait_closed().await {
                                    eprintln!("Document cleanup retained state: {error}");
                                    return;
                                }
                                let _guard = io_lock.lock().await;
                                match session.snapshot() {
                                    Ok(document) => if let Err(error) = close_snapshot(&context, document).await {
                                        eprintln!("Document cleanup retained state: {error}");
                                    },
                                    Err(error) => eprintln!("Document cleanup skipped a closed or replaced session: {error}"),
                                }
                            });
                        }).with_save(move |document, path, expected_bytes| {
                            let context = save_context.clone();
                            let runtime = save_runtime.clone();
                            let lock = Arc::clone(&save_lock);
                            let journal = Arc::clone(&save_journal);
                            let project_id = document.id().clone();
                            let revision = document.revision();
                            Box::pin(async move {
                                let outcome_path = path.clone();
                                let result = runtime.spawn(async move {
                                    let _guard = lock.lock().await;
                                    let revision = document.revision();
                                    match context.query::<bellatrix_project::ProjectSave>(Routing::Unscoped,
                                        bellatrix_project::ProjectSaveRequested { expected_bytes, document, path }).await.map_err(|error| error.to_string())? {
                                        QueryOutcome::Answer(Ok(saved)) if saved == revision => Ok(None),
                                        QueryOutcome::Answer(Ok(_)) => Err("Save completed for a different revision; the document remains unsaved".into()),
                                        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
                                        QueryOutcome::Miss => Err("Project save service unavailable".into()),
                                    }
                                }).await.map_err(|error| format!("Save task failed for {}: {error}. Inspect the destination before retrying.", outcome_path.display())).and_then(|result| result);
                                journal.record_result(project_id, revision, "document.save", &result);
                                result
                            })
                        }).with_mdj_export(move |document, path| {
                            let context = export_context.clone();
                            let runtime = export_runtime.clone();
                            let lock = Arc::clone(&export_lock);
                            let journal = Arc::clone(&export_journal);
                            let project_id = document.id().clone();
                            let revision = document.revision();
                            Box::pin(async move {
                                let outcome_path = path.clone();
                                let result = runtime.spawn(async move {
                                    let _guard = lock.lock().await;
                                    let exported = match context.query::<bellatrix_mde_cordis::ModelingExportMdj>(Routing::Unscoped,
                                        bellatrix_mde_cordis::ModelingExportMdjRequested { document: Arc::clone(&document) }).await.map_err(|error| error.to_string())? {
                                        QueryOutcome::Answer(result) => result?,
                                        QueryOutcome::Miss => return Err("MDJ export service unavailable".into()),
                                    };
                                    if !exported.matches_snapshot(&document) { return Err("MDJ export snapshot changed".into()); }
                                    let ticket = match context.query::<bellatrix_project::ProjectPrepareEncodedSave>(Routing::Unscoped,
                                        exported.prepare_save(path.clone())).await.map_err(|error| error.to_string())? {
                                        QueryOutcome::Answer(result) => result.map_err(|error| error.to_string())?,
                                        QueryOutcome::Miss => return Err("Export publication service unavailable".into()),
                                    };
                                    let published = match context.query::<bellatrix_project::ProjectPublishNewFile>(Routing::Unscoped,
                                        bellatrix_project::ProjectPublishNewFileRequested { ticket }).await.map_err(|error| error.to_string())? {
                                        QueryOutcome::Answer(result) => result.map_err(|error| error.to_string())?,
                                        QueryOutcome::Miss => return Err("Export publication service unavailable".into()),
                                    };
                                    // Export is not a native save. Dropping the publication releases its reservation without clearing dirty state.
                                    Ok(published.cleanup_warning().map(str::to_owned))
                                }).await.map_err(|error| format!("MDJ export task failed for {}: {error}. Publication outcome is uncertain; inspect the destination before retrying.", outcome_path.display())).and_then(|result| result);
                                journal.record_result(project_id, revision, "document.export_mdj", &result);
                                result
                            })
                        }))
                    }).await.map_err(|error| error.to_string())?
                })
            },
        )
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
                        let bytes = match std::env::var_os("BELLATRIX_MODEL_FILE") {
                            Some(path) => std::fs::read(path)?,
                            None => {
                                include_bytes!("../../bellatrix_mde/fixtures/blank.json").to_vec()
                            }
                        };
                        let prepared = match context
                            .query::<bellatrix_mde_cordis::ModelingPrepareDocument>(
                                Routing::Unscoped,
                                bellatrix_mde_cordis::ModelingPrepareDocumentRequested {
                                    bytes,
                                    format: bellatrix_mde_cordis::ModelingDocumentFormat::ModelJson,
                                },
                            )
                            .await?
                        {
                            QueryOutcome::Answer(result) => result.map_err(io::Error::other)?,
                            QueryOutcome::Miss => {
                                return Err(io::Error::other(
                                    "MDE document preparation service unavailable",
                                )
                                .into());
                            }
                        };
                        context
                            .query::<ProjectCreate>(
                                Routing::Unscoped,
                                ProjectCreateRequested {
                                    document: prepared.document,
                                },
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

async fn close_snapshot(
    context: &CordisContext,
    document: Arc<bellatrix_project::ProjectDocument>,
) -> Result<(), String> {
    match context
        .query::<bellatrix_project::ProjectCloseSnapshot>(
            Routing::Unscoped,
            bellatrix_project::ProjectCloseSnapshotRequested { document },
        )
        .await
        .map_err(|error| error.to_string())?
    {
        QueryOutcome::Answer(result) => result.map_err(|error| error.to_string()),
        QueryOutcome::Miss => Err("Project cleanup service unavailable".into()),
    }
}
