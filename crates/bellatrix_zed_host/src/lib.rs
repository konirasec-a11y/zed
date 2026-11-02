//! Bellatrix services embedded in Zed's existing application lifecycle.
//! No GPUI Kit, separate Workspace, or parallel event bus is created here.

use bellatrix_mde_cordis::{MdePlugin, MdeUiBridge, with_model};
use bellatrix_mde_model::Project;
use bellatrix_mde_workbench::{MdeUiSource, register_mde_workspace};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectOpen,
    ProjectOpenRequested, ProjectPlugin, ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context as CordisContext, FiberHandle, Plugin, PreparedPlugin, Routing};
use gpui::{App, Global};
use std::{error::Error, io, path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

/// The GPUI Global owns the Cordis runtime and its plugin generations.
/// Domain state is persistent and remains independent of the UI lifetime.
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
    fn start() -> Result<(Self, MdeUiSource), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let ctx = CordisContext::new();
        let projects = Arc::new(ProjectService::new());
        let (provider, mde, project_id) = runtime.block_on(async {
            let provider_plugin = ProjectPlugin::new(Arc::clone(&projects));
            let provider_input = provider_plugin.prepare(())?;
            let provider = ctx.spawn(PreparedPlugin::from_input(
                provider_plugin, provider_input,
            )).await?;
            let mde_plugin = MdePlugin;
            mde_plugin.prepare(())?;
            let mde = match ctx.spawn(PreparedPlugin::from_input(
                mde_plugin, (),
            )).await {
                Ok(fiber) => fiber,
                Err(error) => {
                    let _ = provider.dispose().await;
                    return Err(Box::<dyn Error>::from(error));
                }
            };

            let opened = async {
                match std::env::var_os("BELLATRIX_PROJECT_FILE") {
                    Some(path) => ctx.query::<ProjectOpen>(
                        Routing::Unscoped,
                        ProjectOpenRequested { path: PathBuf::from(path) },
                    ).await.map_err(|error| Box::<dyn Error>::from(error)),
                    None => {
                        let model: Project = match std::env::var_os("BELLATRIX_MODEL_FILE") {
                            Some(path) => serde_json::from_slice(&std::fs::read(path)?)?,
                            None => serde_json::from_str(include_str!(
                                "../../bellatrix_mde_model/fixtures/todo.json"
                            ))?,
                        };
                        let id = ProjectId::new(model.id.clone())?;
                        let document = ProjectDocument::new(id, model.name.clone())?;
                        let document = with_model(document, &model)
                            .map_err(io::Error::other)?;
                        ctx.query::<ProjectCreate>(
                            Routing::Unscoped,
                            ProjectCreateRequested { document },
                        ).await.map_err(|error| Box::<dyn Error>::from(error))
                    }
                }
            }.await;

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

        // MdeUiBridge creates a Tokio task, so enter the runtime explicitly.
        let bridge = {
            let _guard = runtime.enter();
            MdeUiBridge::start(&ctx, project_id)?
        };
        let source = MdeUiSource::from_bridge(Arc::clone(&projects), &bridge);

        Ok((
            Self {
                runtime,
                _context: ctx,
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
        // Stop taking new commands before disposing the Cordis generations.
        self.bridge.take();
        let mde = self.mde.take();
        let provider = self.provider.take();
        if let (Some(mde), Some(provider)) = (mde, provider) {
            let result = self.runtime.block_on(async move {
                let mde_result = mde.dispose().await;
                let project_result = provider.dispose().await;
                (mde_result, project_result)
            });
            if let (Err(mde_error), _) = &result {
                eprintln!("Bellatrix MDE disposal failed: {mde_error}");
            }
            if let (_, Err(project_error)) = &result {
                eprintln!("Bellatrix Project disposal failed: {project_error}");
            }
        }
    }
}

/// Call once from Zed's own GPUI initialization, after editor/workspace init
/// and before the first Workspace window opens.
pub fn install(cx: &mut App) -> Result<(), Box<dyn Error>> {
    let (host, source) = BellatrixZedHost::start()?;
    register_mde_workspace(cx, source);
    cx.set_global(host);
    Ok(())
}
