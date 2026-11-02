//! Bellatrix services embedded in Zed's existing application lifecycle.
//! No GPUI Kit, separate Workspace, or parallel event bus is created here.
use bellatrix_mde_cordis::{MdePlugin, MdeUiBridge, with_model};
use bellatrix_mde::Project;
use bellatrix_mde_workbench::{MdeUiSource, register_mde_workspace};
use bellatrix_project::{ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectOpen,
    ProjectOpenRequested, ProjectPlugin, ProjectService};
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
    fn start() -> Result<(Self, MdeUiSource), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let context = CordisContext::new();
        let projects = Arc::new(ProjectService::new());
        let (provider, mde, project_id) = runtime.block_on(async {
            let provider_plugin = ProjectPlugin::new(Arc::clone(&projects));
            let provider_input = provider_plugin.prepare(())?;
            let provider = context.spawn(PreparedPlugin::from_input(provider_plugin, provider_input)).await?;
            let mde_plugin = MdePlugin;
            mde_plugin.prepare(())?;
            let mde = match context.spawn(PreparedPlugin::from_input(mde_plugin, ())).await {
                Ok(fiber) => fiber,
                Err(error) => { let _ = provider.dispose().await; return Err(Box::<dyn Error>::from(error)); }
            };
            let opened = async {
                match std::env::var_os("BELLATRIX_PROJECT_FILE") {
                    Some(path) => context.query::<ProjectOpen>(Routing::Unscoped,
                        ProjectOpenRequested { path: PathBuf::from(path) }).await.map_err(Box::<dyn Error>::from),
                    None => {
                        let model: Project = match std::env::var_os("BELLATRIX_MODEL_FILE") {
                            Some(path) => serde_json::from_slice(&std::fs::read(path)?)?,
                            None => serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?,
                        };
                        let id = ProjectId::new(model.id.clone())?;
                        let document = ProjectDocument::new(id, model.name.clone())?;
                        let document = with_model(document, &model).map_err(io::Error::other)?;
                        context.query::<ProjectCreate>(Routing::Unscoped,
                            ProjectCreateRequested { document }).await.map_err(Box::<dyn Error>::from)
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
                Err(error) => { let _ = mde.dispose().await; let _ = provider.dispose().await; Err(error) }
            }
        })?;
        let bridge = { let _guard = runtime.enter(); MdeUiBridge::start(&context, project_id)? };
        let source = MdeUiSource::from_bridge(Arc::clone(&projects), &bridge);
        Ok((Self { runtime, _context: context, _projects: projects, bridge: Some(bridge),
            provider: Some(provider), mde: Some(mde) }, source))
    }
}
impl Drop for BellatrixZedHost {
    fn drop(&mut self) {
        self.bridge.take();
        let mde = self.mde.take(); let provider = self.provider.take();
        if let (Some(mde), Some(provider)) = (mde, provider) {
            let result = self.runtime.block_on(async move {
                let mde_result = mde.dispose().await;
                let project_result = provider.dispose().await;
                (mde_result, project_result)
            });
            if let (Err(error), _) = &result { eprintln!("Bellatrix MDE disposal failed: {error}"); }
            if let (_, Err(error)) = &result { eprintln!("Bellatrix Project disposal failed: {error}"); }
        }
    }
}
/// Called once after Zed's editor/workspace initialization and before its first window.
pub fn install(cx: &mut App) -> Result<(), Box<dyn Error>> {
    let (host, source) = BellatrixZedHost::start()?;
    register_mde_workspace(cx, source); cx.set_global(host); Ok(())
}
