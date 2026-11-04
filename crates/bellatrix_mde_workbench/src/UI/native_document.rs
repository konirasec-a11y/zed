use crate::workbench_ui::native_document_guard::{NativeBufferState, NativeDocumentGuard};
use crate::workbench_ui::{MdeUiSource, MdeWorkbenchItem};
use bellatrix_project::ProjectDocument;
use gpui::{
    App, AppContext, Context, Entity, EntityId, EventEmitter, FocusHandle, Focusable,
    PathPromptOptions, Subscription, Window, actions,
};
use language::{Buffer, BufferSnapshot, DiskState};
use project::{DirectoryLister, ProjectPath};
use std::{cell::Cell, future::Future, path::PathBuf, pin::Pin, rc::Rc, sync::Arc};
use ui::{ContextMenu, DropdownMenu, DropdownStyle, prelude::*};
use workspace::{
    Workspace,
    item::{Item, ItemBufferKind, ItemEvent, SaveOptions},
};

type ImportFuture = Pin<Box<dyn Future<Output = Result<NativeDocumentLease, String>> + Send>>;
type PrepareFuture =
    Pin<Box<dyn Future<Output = Result<(ProjectDocument, Vec<String>), String>> + Send>>;

#[derive(Clone)]
pub struct NativeDocumentImporter {
    prepare: Arc<dyn Fn(Vec<u8>, bool) -> PrepareFuture + Send + Sync>,
    import: Arc<dyn Fn(ProjectDocument, bool) -> ImportFuture + Send + Sync>,
}

impl NativeDocumentImporter {
    pub fn new(
        prepare: impl Fn(Vec<u8>, bool) -> PrepareFuture + Send + Sync + 'static,
        import: impl Fn(ProjectDocument, bool) -> ImportFuture + Send + Sync + 'static,
    ) -> Self {
        Self {
            prepare: Arc::new(prepare),
            import: Arc::new(import),
        }
    }
}

type DocumentWriteFuture = Pin<Box<dyn Future<Output = Result<Option<String>, String>> + Send>>;
type DocumentWrite =
    Arc<dyn Fn(Arc<ProjectDocument>, PathBuf) -> DocumentWriteFuture + Send + Sync>;

pub struct NativeDocumentLease {
    save: Option<DocumentWrite>,
    mdj_export: Option<DocumentWrite>,
    source: MdeUiSource,
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl NativeDocumentLease {
    pub fn new(source: MdeUiSource, cleanup: impl FnOnce() + Send + 'static) -> Self {
        Self {
            source,
            save: None,
            mdj_export: None,
            cleanup: Some(Box::new(cleanup)),
        }
    }
}

impl NativeDocumentLease {
    pub fn with_save(
        mut self,
        save: impl Fn(Arc<ProjectDocument>, PathBuf) -> DocumentWriteFuture + Send + Sync + 'static,
    ) -> Self {
        self.save = Some(Arc::new(save));
        self
    }

    pub fn with_mdj_export(
        mut self,
        export: impl Fn(Arc<ProjectDocument>, PathBuf) -> DocumentWriteFuture + Send + Sync + 'static,
    ) -> Self {
        self.mdj_export = Some(Arc::new(export));
        self
    }
}

impl Drop for NativeDocumentLease {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

actions!(bellatrix_mde, [OpenMdeDocument]);

#[derive(Clone)]
struct NativeVersion(BufferSnapshot);
impl PartialEq for NativeVersion {
    fn eq(&self, other: &Self) -> bool {
        self.0.version() == other.0.version()
    }
}
type NativeIdentity = (EntityId, Option<ProjectPath>, Option<DiskState>);
type NativeState = NativeBufferState<NativeVersion, NativeIdentity>;

fn buffer_state(buffer: &Entity<Buffer>, cx: &App) -> NativeState {
    let value = buffer.read(cx);
    NativeBufferState {
        version: NativeVersion(value.snapshot()),
        identity: (
            buffer.entity_id(),
            value
                .file()
                .map(|file| ProjectPath::from_file(file.as_ref(), cx)),
            value.file().map(|file| file.disk_state()),
        ),
        dirty: value.is_dirty() || value.has_unsaved_edits(),
        conflict: value.has_conflict(),
        read_only: value.read_only(),
    }
}

fn dirty_buffer_paths(project: &Entity<project::Project>, cx: &App) -> Vec<PathBuf> {
    let mut paths = project
        .read(cx)
        .opened_buffers(cx)
        .into_iter()
        .filter_map(|buffer| {
            let buffer = buffer.read(cx);
            (buffer.is_dirty() || buffer.has_unsaved_edits())
                .then(|| buffer.file().and_then(|file| file.file_system_abs_path(cx)))
                .flatten()
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

struct OpenPromptGuard(Rc<Cell<bool>>);
impl Drop for OpenPromptGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

pub(crate) fn active_preview_label(workspace: &Workspace, cx: &App) -> Option<String> {
    workspace
        .active_item_as::<NativeMdeDocumentItem>(cx)
        .map(|item| item.read(cx).label())
}

pub(crate) fn active_workbench(
    workspace: &Workspace,
    cx: &App,
) -> Option<Entity<MdeWorkbenchItem>> {
    workspace
        .active_item_as::<NativeMdeDocumentItem>(cx)
        .map(|item| item.read(cx).workbench.clone())
        .or_else(|| workspace.active_item_as::<MdeWorkbenchItem>(cx))
}

fn prepare_preview_panels(
    workspace: &mut Workspace,
    source: &MdeUiSource,
    preview: &Entity<NativeMdeDocumentItem>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    if source.set_preview_context(Some(preview.read(cx).label())) {
        let items = workspace
            .items_of_type::<MdeWorkbenchItem>(cx)
            .collect::<Vec<_>>();
        for item in items {
            item.update(cx, |item, cx| item.refresh(cx));
        }
    }
    let workbench = preview.read(cx).workbench.clone();
    crate::workbench_ui::panels::attach_panels(workspace, workbench, window, cx);
}

pub(crate) fn register_preview_action(
    workspace: &mut Workspace,
    importer: NativeDocumentImporter,
    presentation_source: MdeUiSource,
) {
    let pending = Rc::new(Cell::new(false));
    workspace.register_action(move |workspace, _: &OpenMdeDocument, window, cx| {
        if !workspace.project().read(cx).is_local() {
            workspace.show_error(
                "Opening a Bellatrix document currently requires a local workspace",
                cx,
            );
            return;
        }
        if pending.replace(true) {
            return;
        }
        let guard = OpenPromptGuard(Rc::clone(&pending));
        let paths = workspace.prompt_for_open_path(
            PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Open Bellatrix project or import StarUML (.mdj)".into()),
            },
            DirectoryLister::Local(
                workspace.project().clone(),
                workspace.app_state().fs.clone(),
            ),
            window,
            cx,
        );
        let importer = importer.clone();
        let presentation_source = presentation_source.clone();
        cx.spawn_in(window, async move |workspace, cx| {
            let _guard = guard;
            let result: Result<(), String> = async {
                let Some(path) = paths
                    .await
                    .map_err(|error| error.to_string())?
                    .and_then(|paths| paths.into_iter().next())
                else {
                    return Ok(());
                };
                let source_path = path.clone();
                let is_mdj = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("mdj"));
                let worktree_task = workspace
                    .update(cx, |workspace, cx| {
                        workspace.project().update(cx, |project, cx| {
                            project.find_or_create_worktree(path, false, cx)
                        })
                    })
                    .map_err(|error| error.to_string())?;
                let (worktree, path) = worktree_task.await.map_err(|error| error.to_string())?;
                let open_task = workspace
                    .update(cx, |workspace, cx| {
                        let path = ProjectPath {
                            worktree_id: worktree.read(cx).id(),
                            path,
                        };
                        workspace
                            .project()
                            .update(cx, |project, cx| project.open_buffer(path, cx))
                    })
                    .map_err(|error| error.to_string())?;
                let buffer = open_task.await.map_err(|error| error.to_string())?;
                let existing = workspace
                    .update_in(cx, |workspace, window, cx| {
                        let existing = workspace
                            .items_of_type::<NativeMdeDocumentItem>(cx)
                            .find(|item| item.read(cx).buffer.entity_id() == buffer.entity_id());
                        if let Some(item) = existing {
                            prepare_preview_panels(
                                workspace,
                                &presentation_source,
                                &item,
                                window,
                                cx,
                            );
                            workspace.activate_item(&item, true, true, window, cx);
                            true
                        } else {
                            false
                        }
                    })
                    .map_err(|error| error.to_string())?;
                if existing {
                    return Ok(());
                }
                let state = workspace
                    .update(cx, |_, cx| buffer_state(&buffer, cx))
                    .map_err(|error| error.to_string())?;
                let buffer_snapshot = state.version.0.clone();
                let clean = !state.dirty && !state.conflict && !state.read_only;
                let fs = workspace.update(cx, |workspace, _| workspace.app_state().fs.clone()).map_err(|error| error.to_string())?;
                let (raw, read_diagnostic) = if clean {
                    match fs.load_bytes(&source_path).await {
                        Ok(bytes) => (Some(bytes), None),
                        Err(error) => (None, Some(format!("Importing the captured editor text because raw bytes could not be verified at {}: {error}", source_path.display()))),
                    }
                } else { (None, None) };
                let raw = raw.filter(|bytes| std::str::from_utf8(bytes).is_ok_and(|text| text.replace("\r\n", "\n") == buffer_snapshot.text()));
                let native_saved = !is_mdj && raw.is_some();
                let native_path = native_saved.then(|| source_path.clone());
                let saved_bytes = if native_saved { raw.clone() } else { None };
                let protected_source = if native_saved { None } else { Some(fs.canonicalize(&source_path).await.unwrap_or_else(|_| source_path.clone())) };
                let baseline = NativeDocumentGuard::new(state).map_err(|error| error.message().to_owned())?;
                let bytes = raw.unwrap_or_else(|| buffer_snapshot.text().into_bytes());
                let (document, import_diagnostics) = (importer.prepare)(bytes, is_mdj).await?;
                let mut diagnostics = if is_mdj {
                    vec!["Imported into a native Bellatrix document. Save stores the complete Bellatrix project; the source is preserved. MDJ export is experimental and may reject unsupported mappings.".to_owned()]
                } else { Vec::new() };
                diagnostics.extend(read_diagnostic);
                diagnostics.extend(import_diagnostics);
                workspace
                    .update(cx, |_, cx| baseline.check(&buffer_state(&buffer, cx)))
                    .map_err(|error| error.to_string())?
                    .map_err(|error| error.message().to_owned())?;
                let lease = (importer.import)(document, native_saved).await?;
                workspace
                    .update_in(cx, |workspace, window, cx| {
                        baseline
                            .check(&buffer_state(&buffer, cx))
                            .map_err(|error| error.message().to_owned())?;
                        let project = workspace.project().clone();
                        let item = cx.new(|cx| {
                            NativeMdeDocumentItem::new(buffer, project, lease, diagnostics, source_path, protected_source, native_path, saved_bytes, cx)
                        });
                        prepare_preview_panels(workspace, &presentation_source, &item, window, cx);
                        workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
                        Ok::<(), String>(())
                    })
                    .map_err(|error| error.to_string())??;
                Ok(())
            }
            .await;
            if let Err(error) = result {
                if let Err(unavailable) =
                    workspace.update(cx, |workspace, cx| workspace.show_error(error, cx))
                {
                    eprintln!("MDE preview workspace closed: {unavailable}");
                }
            }
        })
        .detach();
    });
}

pub(crate) struct NativeMdeDocumentItem {
    entity_id: EntityId,
    source_path: PathBuf,
    protected_source: Option<PathBuf>,
    native_path: Option<PathBuf>,
    saved_bytes: Option<Vec<u8>>,
    project: Entity<project::Project>,
    saving: bool,
    discarded: bool,
    recovery: Option<String>,
    buffer: Entity<Buffer>,
    workbench: Entity<MdeWorkbenchItem>,
    lease: NativeDocumentLease,
    diagnostics: Vec<String>,
    diagnostics_expanded: bool,
    diagnostic_page: usize,
    _workbench_subscription: Subscription,
    _revision_task: gpui::Task<()>,
}

impl NativeMdeDocumentItem {
    fn label(&self) -> String {
        let id = self.lease.source.project_id.as_str();
        self.lease
            .source
            .snapshot()
            .map(|snapshot| {
                if snapshot.name.trim().is_empty() {
                    format!("Unnamed project ({id})")
                } else {
                    format!("{} ({id})", snapshot.name)
                }
            })
            .unwrap_or_else(|_| id.to_owned())
    }

    fn new(
        buffer: Entity<Buffer>,
        project: Entity<project::Project>,
        lease: NativeDocumentLease,
        diagnostics: Vec<String>,
        source_path: PathBuf,
        protected_source: Option<PathBuf>,
        native_path: Option<PathBuf>,
        saved_bytes: Option<Vec<u8>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let workbench = cx.new(|cx| MdeWorkbenchItem::new(lease.source.clone(), cx));
        let workbench_subscription = cx.observe(&workbench, |_, _, cx| cx.notify());
        let mut revisions = lease.source.revisions.clone();
        let mut activity = lease.source.activity();
        let revision_task = cx.spawn(async move |this, cx| {
            loop {
                let edited = tokio::select! {
                    changed = revisions.changed() => { if changed.is_err() { break; } true },
                    changed = activity.changed() => { if changed.is_err() { break; } false },
                };
                if this
                    .update(cx, |_, cx| {
                        if edited {
                            cx.emit(ItemEvent::Edit);
                        }
                        cx.emit(ItemEvent::UpdateTab);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            entity_id: cx.entity().entity_id(),
            source_path,
            protected_source,
            native_path,
            saved_bytes,
            project,
            saving: false,
            discarded: false,
            recovery: None,
            buffer,
            workbench,
            lease,
            diagnostics,
            diagnostics_expanded: false,
            diagnostic_page: 0,
            _workbench_subscription: workbench_subscription,
            _revision_task: revision_task,
        }
    }

    fn document_is_dirty(&self) -> bool {
        !self.discarded
            && (self.saving
                || !self.lease.source.snapshot_is_current()
                || self.lease.source.is_pending()
                || self
                    .lease
                    .source
                    .projects
                    .is_dirty(&self.lease.source.project_id)
                    .unwrap_or(true))
    }

    pub(crate) fn save_project_as(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Task<anyhow::Result<()>> {
        self.write_document(None, false, cx)
    }

    fn write_document(
        &mut self,
        path: Option<PathBuf>,
        export_mdj: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Task<anyhow::Result<()>> {
        let ready = (|| {
            anyhow::ensure!(
                !self.discarded,
                "This document was discarded; reopen it before saving"
            );
            anyhow::ensure!(!self.saving, "A document write is already pending");
            anyhow::ensure!(
                !self.lease.source.is_pending(),
                "An edit is pending; retry after it completes"
            );
            anyhow::ensure!(
                self.lease.source.snapshot_is_current(),
                "The document session changed"
            );
            let document = self.lease.source.document().map_err(anyhow::Error::msg)?;
            let write = if export_mdj {
                self.lease.mdj_export.clone()
            } else {
                self.lease.save.clone()
            }
            .ok_or_else(|| anyhow::anyhow!("Document writer unavailable"))?;
            Ok::<_, anyhow::Error>((document, write))
        })();
        let (document, write) = match ready {
            Ok(ready) => ready,
            Err(error) => return gpui::Task::ready(Err(error)),
        };
        let prompt = path.is_none().then(|| {
            cx.prompt_for_new_path(
                self.native_path
                    .as_ref()
                    .unwrap_or(&self.source_path)
                    .parent()
                    .unwrap_or(std::path::Path::new(".")),
                Some(if export_mdj {
                    "model-export.mdj"
                } else {
                    "bellatrix-project.json"
                }),
            )
        });
        let project = self.project.clone();
        let fs = project.read(cx).fs().clone();
        let protected_source = self.protected_source.clone();
        let previous_path = self.native_path.clone();
        let previous_bytes = self.saved_bytes.clone();
        self.saving = true;
        self.lease.source.block_native_edits(true);
        self.recovery = None;
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result: anyhow::Result<()> = async {
                let path = match (path, prompt) {
                    (Some(path), _) => path,
                    (None, Some(prompt)) => prompt.await??.ok_or_else(|| anyhow::anyhow!("Write cancelled; the document was not changed"))?,
                    _ => return Err(anyhow::anyhow!("Destination is unavailable")),
                };
                let extension = path.extension().and_then(|value| value.to_str());
                anyhow::ensure!(extension.is_some_and(|extension| extension.eq_ignore_ascii_case(if export_mdj { "mdj" } else { "json" })),
                    "Choose a {} filename", if export_mdj { ".mdj" } else { ".json" });
                let parent = path.parent().ok_or_else(|| anyhow::anyhow!("Destination has no parent directory"))?;
                let name = path.file_name().ok_or_else(|| anyhow::anyhow!("Destination has no filename"))?;
                let destination = fs.canonicalize(parent).await?.join(name);
                if let Some(source) = &protected_source {
                    anyhow::ensure!(destination != *source, "Choose a different destination; the imported source is preserved");
                }
                let dirty_paths = this.update(cx, |_, cx| dirty_buffer_paths(&project, cx))?;
                for dirty in &dirty_paths {
                    let equivalent = if let (Some(parent), Some(name)) = (dirty.parent(), dirty.file_name()) {
                        fs.canonicalize(parent).await?.join(name) == destination
                    } else { dirty == &path };
                    anyhow::ensure!(!equivalent, "The destination has unsaved text edits; choose another filename");
                }
                if !export_mdj && let (Some(previous), Some(bytes)) = (&previous_path, &previous_bytes) {
                    let same_destination = if let (Some(parent), Some(name)) = (previous.parent(), previous.file_name()) {
                        fs.canonicalize(parent).await?.join(name) == destination
                    } else { previous == &path };
                    if same_destination {
                        anyhow::ensure!(fs.load_bytes(previous).await? == *bytes, "The native project changed on disk; save to a new filename to preserve both versions");
                    }
                }
                let binding = if export_mdj { None } else {
                    let binding = project.update(cx, |project, cx| project.find_or_create_worktree(path.clone(), true, cx))?;
                    Some(binding.await?)
                };
                this.update(cx, |this, cx| {
                    anyhow::ensure!(!this.discarded, "The document was discarded; write cancelled");
                    anyhow::ensure!(!this.lease.source.is_pending(), "An edit became pending during write preparation");
                    let current = this.lease.source.document().map_err(anyhow::Error::msg)?;
                    anyhow::ensure!(Arc::ptr_eq(&current, &document), "The document changed during write preparation");
                    anyhow::ensure!(dirty_buffer_paths(&project, cx) == dirty_paths, "Open text edits changed during write preparation; retry");
                    Ok::<_, anyhow::Error>(())
                })??;
                let bytes = if export_mdj { None } else { Some(serde_json::to_vec_pretty(document.as_ref())?) };
                let mut warning = write(Arc::clone(&document), path.clone()).await.map_err(anyhow::Error::msg)?;
                if let Some((worktree, relative_path)) = binding {
                    let refresh = worktree.update(cx, |worktree, cx| worktree.as_local().map(|local| local.refresh_entry(relative_path, None, cx)))?;
                    if let Some(refresh) = refresh && let Err(error) = refresh.await {
                        let message = format!("The file was saved, but its workspace entry could not be refreshed: {error}");
                        warning = Some(warning.map_or_else(|| message.clone(), |existing| format!("{existing} {message}")));
                    }
                }
                this.update(cx, |this, cx| {
                    if !export_mdj {
                        this.native_path = Some(path.clone());
                        this.saved_bytes = bytes;
                    }
                    anyhow::ensure!(!this.discarded, "The file was written to {}, but its document tab was discarded", path.display());
                    let changed = !this.lease.source.document().is_ok_and(|current| Arc::ptr_eq(&current, &document));
                    let mut message = if export_mdj {
                        format!("Experimental MDJ export written to {}. Native project save state is unchanged.", path.display())
                    } else {
                        format!("Bellatrix project saved to {}.{}", path.display(), if changed { " Newer changes remain unsaved." } else { "" })
                    };
                    if let Some(warning) = warning { message.push_str(&format!(" {warning}")); }
                    this.recovery = Some(message);
                    cx.emit(ItemEvent::UpdateTab);
                    Ok::<_, anyhow::Error>(())
                })??;
                Ok(())
            }.await;
            this.update(cx, |this, cx| {
                this.saving = false;
                this.lease.source.block_native_edits(this.discarded);
                if let Err(error) = &result { this.recovery = Some(error.to_string()); }
                cx.emit(ItemEvent::UpdateTab);
                cx.notify();
            })?;
            result
        })
    }
}
impl project::ProjectItem for NativeMdeDocumentItem {
    fn try_open(
        _: &Entity<project::Project>,
        _: &ProjectPath,
        _: &mut App,
    ) -> Option<gpui::Task<anyhow::Result<Entity<Self>>>> {
        None
    }
    fn entry_id(&self, cx: &App) -> Option<project::ProjectEntryId> {
        let path = Item::active_project_path(self, cx)?;
        self.project
            .read(cx)
            .entry_for_path(&path, cx)
            .map(|entry| entry.id)
    }
    fn project_path(&self, cx: &App) -> Option<ProjectPath> {
        Item::active_project_path(self, cx)
    }
    fn is_dirty(&self) -> bool {
        self.document_is_dirty()
    }
}
impl EventEmitter<ItemEvent> for NativeMdeDocumentItem {}
impl Focusable for NativeMdeDocumentItem {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.workbench.read(cx).focus_handle(cx)
    }
}
impl Item for NativeMdeDocumentItem {
    type Event = ItemEvent;
    fn to_item_events(event: &ItemEvent, callback: &mut dyn FnMut(ItemEvent)) {
        callback(*event);
    }
    fn for_each_project_item(
        &self,
        _: &App,
        callback: &mut dyn FnMut(EntityId, &dyn project::ProjectItem),
    ) {
        if self.native_path.is_some() && !self.discarded {
            callback(self.entity_id, self);
        }
    }
    fn buffer_kind(&self, _: &App) -> ItemBufferKind {
        ItemBufferKind::Singleton
    }
    fn can_save(&self, _: &App) -> bool {
        !self.discarded && self.lease.save.is_some()
    }
    fn can_save_as(&self, _: &App) -> bool {
        !self.discarded && self.lease.save.is_some()
    }
    fn is_dirty(&self, _: &App) -> bool {
        self.document_is_dirty()
    }
    fn reload(
        &mut self,
        _project: Entity<project::Project>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Task<anyhow::Result<()>> {
        self.discarded = true;
        self.lease.source.block_native_edits(true);
        self.lease.save = None;
        self.lease.mdj_export = None;
        if let Some(cleanup) = self.lease.cleanup.take() {
            cleanup();
        }
        self.recovery = Some(
            "Document discarded. Reopen the saved Bellatrix project or import the source again."
                .into(),
        );
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
        gpui::Task::ready(Ok(()))
    }
    fn save(
        &mut self,
        options: SaveOptions,
        _project: Entity<project::Project>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Task<anyhow::Result<()>> {
        if options.autosave && self.native_path.is_none() {
            return gpui::Task::ready(Err(anyhow::anyhow!(
                "Choose a Bellatrix project filename with Save before autosaving"
            )));
        }
        if !self.document_is_dirty() {
            return gpui::Task::ready(Ok(()));
        }
        self.write_document(self.native_path.clone(), false, cx)
    }
    fn save_as(
        &mut self,
        project: Entity<project::Project>,
        path: ProjectPath,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Task<anyhow::Result<()>> {
        let Some(path) = project.read(cx).absolute_path(&path, cx) else {
            return gpui::Task::ready(Err(anyhow::anyhow!("Save requires a local destination")));
        };
        self.write_document(Some(path), false, cx)
    }
    fn suggested_filename(&self, _: &App) -> SharedString {
        self.native_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "bellatrix-project.json".into())
            .into()
    }
    fn tab_content_text(&self, _: usize, _: &App) -> SharedString {
        format!("MDE · {}", self.lease.source.project_id.as_str()).into()
    }
    fn tab_tooltip_text(&self, _: &App) -> Option<SharedString> {
        Some("Native Bellatrix document. MDJ export is a separate experimental operation.".into())
    }
    fn active_project_path(&self, cx: &App) -> Option<ProjectPath> {
        if self.discarded {
            return None;
        }
        self.native_path.as_ref().and_then(|path| {
            self.project
                .read(cx)
                .project_path_for_absolute_path(path, cx)
        })
    }
    fn show_toolbar(&self) -> bool {
        false
    }
}
impl Render for NativeMdeDocumentItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let diagrams = self
            .lease
            .source
            .snapshot()
            .map(|snapshot| snapshot.section.diagrams)
            .unwrap_or_default();
        let owner = self.workbench.clone();
        let selected = owner.read(cx).active_diagram();
        let label = diagrams
            .iter()
            .find(|diagram| Some(diagram.id.as_str()) == selected)
            .map(|diagram| diagram.name.clone())
            .unwrap_or_else(|| "Diagram".into());
        let menu = ContextMenu::build(window, cx, move |menu, _, _| {
            diagrams.into_iter().fold(menu, |menu, diagram| {
                let owner = owner.clone();
                menu.entry(diagram.name, None, move |_, cx| {
                    owner.update(cx, |item, cx| item.select_diagram(diagram.id.clone(), cx));
                })
            })
        });
        let validate_owner = self.workbench.clone();
        let message = if let Some(recovery) = &self.recovery {
            recovery.as_str()
        } else if !self.lease.source.snapshot_is_current() {
            "The document session is no longer open. Reopen the saved Bellatrix project to continue."
        } else {
            "Native Bellatrix document. Save preserves the complete project; MDJ export is experimental."
        };
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_2()
                    .child(Label::new("Bellatrix document"))
                    .child(
                        DropdownMenu::new("mde-document-diagram", label, menu)
                            .style(DropdownStyle::Subtle),
                    )
                    .child(
                        Button::new("mde-document-validate", "Validate")
                            .disabled(self.discarded || !self.lease.source.snapshot_is_current())
                            .on_click(move |_, _, cx| {
                                validate_owner.update(cx, |item, cx| item.validate_diagram(cx));
                            }),
                    )
                    .child(
                        Button::new("mde-document-export-mdj", "Export MDJ (experimental)")
                            .disabled(
                                self.discarded || self.saving || self.lease.mdj_export.is_none(),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.write_document(None, true, cx).detach_and_log_err(cx);
                            })),
                    )
                    .when(!self.diagnostics.is_empty(), |header| {
                        header.child(
                            Button::new(
                                "mde-toggle-import-diagnostics",
                                format!(
                                    "{} import diagnostics ({})",
                                    if self.diagnostics_expanded {
                                        "Hide"
                                    } else {
                                        "Show"
                                    },
                                    self.diagnostics.len()
                                ),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.diagnostics_expanded = !this.diagnostics_expanded;
                                cx.notify();
                            })),
                        )
                    }),
            )
            .child(Label::new(message.to_owned()))
            .when(
                self.diagnostics_expanded && !self.diagnostics.is_empty(),
                |preview| {
                    let page_size = 50;
                    let page_count = self.diagnostics.len().div_ceil(page_size);
                    let start = self.diagnostic_page * page_size;
                    preview.child(
                        v_flex()
                            .flex_shrink_0()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Label::new(format!(
                                        "Import diagnostics ({}) · Page {} of {}",
                                        self.diagnostics.len(),
                                        self.diagnostic_page + 1,
                                        page_count
                                    )))
                                    .child(
                                        Button::new("mde-diagnostics-previous", "Previous")
                                            .disabled(self.diagnostic_page == 0)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.diagnostic_page =
                                                    this.diagnostic_page.saturating_sub(1);
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("mde-diagnostics-next", "Next")
                                            .disabled(self.diagnostic_page + 1 >= page_count)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.diagnostic_page =
                                                    (this.diagnostic_page + 1).min(page_count - 1);
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .id(("mde-import-diagnostics", self.diagnostic_page))
                                    .max_h(px(160.0))
                                    .overflow_y_scroll()
                                    .children(
                                        self.diagnostics
                                            .iter()
                                            .skip(start)
                                            .take(page_size)
                                            .map(|message| Label::new(message.clone())),
                                    ),
                            ),
                    )
                },
            )
            .when(!self.discarded, |container| {
                container.child(div().flex_1().min_h_0().child(self.workbench.clone()))
            })
    }
}
