use crate::workbench_ui::native_document_guard::{
    NativeBufferState, NativeDocumentConflict, NativeDocumentGuard,
};
use crate::workbench_ui::{MdeUiSource, MdeWorkbenchItem, from_document};
use bellatrix_project::{ProjectDocument, ProjectId};
use gpui::{
    App, AppContext, Context, Entity, EntityId, EventEmitter, FocusHandle, Focusable,
    PathPromptOptions, Subscription, Window, actions,
};
use language::{Buffer, BufferEvent, BufferSnapshot, DiskState};
use project::{DirectoryLister, ProjectPath};
use std::{cell::Cell, future::Future, path::PathBuf, pin::Pin, rc::Rc, sync::Arc};
use ui::{ContextMenu, DropdownMenu, DropdownStyle, prelude::*};
use workspace::{
    Workspace,
    item::{Item, ItemBufferKind, ItemEvent, SaveOptions},
};

type ImportFuture = Pin<Box<dyn Future<Output = Result<NativeDocumentLease, String>> + Send>>;

#[derive(Clone)]
pub struct NativeDocumentImporter(
    Arc<dyn Fn(ProjectDocument, bool, bool) -> ImportFuture + Send + Sync>,
);

impl NativeDocumentImporter {
    pub fn new(
        import: impl Fn(ProjectDocument, bool, bool) -> ImportFuture + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(import))
    }
}

type SaveCopyFuture = Pin<Box<dyn Future<Output = Result<Option<String>, String>> + Send>>;
type SaveCopy = Arc<dyn Fn(Arc<ProjectDocument>, PathBuf) -> SaveCopyFuture + Send + Sync>;

pub struct NativeDocumentLease {
    save_copy: Option<SaveCopy>,
    source: MdeUiSource,
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl NativeDocumentLease {
    pub fn new(source: MdeUiSource, cleanup: impl FnOnce() + Send + 'static) -> Self {
        Self {
            source,
            save_copy: None,
            cleanup: Some(Box::new(cleanup)),
        }
    }
}

impl NativeDocumentLease {
    pub fn with_save_copy(
        mut self,
        save: impl Fn(Arc<ProjectDocument>, PathBuf) -> SaveCopyFuture + Send + Sync + 'static,
    ) -> Self {
        self.save_copy = Some(Arc::new(save));
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

actions!(bellatrix_mde, [OpenMdeProjectPreview]);

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
        .active_item_as::<NativeMdePreviewItem>(cx)
        .map(|item| item.read(cx).label())
}

pub(crate) fn preview_is_active(workspace: &Workspace, cx: &App) -> bool {
    active_preview_label(workspace, cx).is_some()
}

pub(crate) fn active_workbench(
    workspace: &Workspace,
    cx: &App,
) -> Option<Entity<MdeWorkbenchItem>> {
    workspace
        .active_item_as::<NativeMdePreviewItem>(cx)
        .map(|item| item.read(cx).workbench.clone())
        .or_else(|| workspace.active_item_as::<MdeWorkbenchItem>(cx))
}

fn prepare_preview_panels(
    workspace: &mut Workspace,
    source: &MdeUiSource,
    preview: &Entity<NativeMdePreviewItem>,
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
    workspace.register_action(move |workspace, _: &OpenMdeProjectPreview, window, cx| {
        if !workspace.project().read(cx).is_local() {
            workspace.show_error(
                "MDE project preview currently requires a local workspace",
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
                prompt: Some("Open Bellatrix preview or StarUML (.mdj)".into()),
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
                            .items_of_type::<NativeMdePreviewItem>(cx)
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
                let (raw, read_diagnostic) = if is_mdj && clean {
                    match fs.load_bytes(&source_path).await {
                        Ok(bytes) => (Some(bytes), None),
                        Err(error) => (None, Some(format!("Read-only fallback: could not verify raw source bytes at {}: {error}", source_path.display()))),
                    }
                } else { (None, None) };
                let raw = raw.filter(|bytes| std::str::from_utf8(bytes).is_ok_and(|text| text.replace("\r\n", "\n") == buffer_snapshot.text()));
                let editable = is_mdj && raw.is_some();
                let original_bytes = raw.clone();
                let baseline =
                    NativeDocumentGuard::new(state).map_err(|error| error.message().to_owned())?;
                let (document, diagnostics) = cx.background_spawn(async move {
                    let text = buffer_snapshot.text();
                    let (document, diagnostics) = if is_mdj {
                        let imported = bellatrix_mde::mdj_conversion::import(raw.as_deref().unwrap_or(text.as_bytes()))
                            .map_err(|error| format!("Invalid StarUML MDJ document: {error}"))?;
                        let id = ProjectId::new(imported.project.id.clone())
                            .map_err(|error| error.to_string())?;
                        let document = ProjectDocument::new(id, imported.project.name.clone())
                            .map_err(|error| error.to_string())?;
                        let document = bellatrix_mde_cordis::with_model(document, &imported.project)?;
                        let mut diagnostics = vec![
                            if editable { "Limited StarUML editing: supported top-level labels, names, properties and styles only. Save creates a new copy and never replaces the source. UML mapping, inherited/composite styles, rendering, routing and resources remain partial." } else { "Read-only StarUML preview: clean UTF-8 disk bytes matching the editor could not be verified. The source is not modified; editor text provenance does not guarantee original encoding or line endings." }.to_owned(),
                        ];
                        diagnostics.extend(read_diagnostic);
                        if editable { diagnostics.push("Native MDJ validation is not enabled for this session; the validation control is unavailable.".into()); }
                        diagnostics.extend(imported.diagnostics.into_iter().map(|diagnostic| {
                            let path = if diagnostic.path.is_empty() {
                                "/"
                            } else {
                                &diagnostic.path
                            };
                            format!("{path}: {}", diagnostic.message)
                        }));
                        (document, diagnostics)
                    } else {
                        let document: ProjectDocument = serde_json::from_str(&text)
                            .map_err(|error| format!("Invalid Bellatrix project document: {error}"))?;
                        (document, Vec::new())
                    };
                    let snapshot = from_document(&document)?;
                    bellatrix_mde::Graph::new(
                        &snapshot
                            .section
                            .to_model(document.id().as_str(), document.name()),
                    )
                    .validate_presentation()
                    .map_err(|error| error.to_string())?;
                    Ok::<_, String>((document, diagnostics))
                }).await?;
                workspace
                    .update(cx, |_, cx| baseline.check(&buffer_state(&buffer, cx)))
                    .map_err(|error| error.to_string())?
                    .map_err(|error| error.message().to_owned())?;
                let lease = (importer.0)(document, baseline.is_saved(), editable).await?;
                workspace
                    .update_in(cx, |workspace, window, cx| {
                        baseline
                            .check(&buffer_state(&buffer, cx))
                            .map_err(|error| error.message().to_owned())?;
                        let item = cx.new(|cx| {
                            NativeMdePreviewItem::new(buffer, baseline, lease, diagnostics, source_path, original_bytes, cx)
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

struct NativeMdePreviewItem {
    source_path: PathBuf,
    original_bytes: Option<Vec<u8>>,
    saving: bool,
    discarded: bool,
    recovery: Option<String>,
    saved_copy: Option<Arc<ProjectDocument>>,
    buffer: Entity<Buffer>,
    baseline: NativeDocumentGuard<NativeVersion, NativeIdentity>,
    workbench: Entity<MdeWorkbenchItem>,
    lease: NativeDocumentLease,
    diagnostics: Vec<String>,
    diagnostic_page: usize,
    _subscription: Subscription,
    _workbench_subscription: Subscription,
    _revision_task: gpui::Task<()>,
}

impl NativeMdePreviewItem {
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
        baseline: NativeDocumentGuard<NativeVersion, NativeIdentity>,
        lease: NativeDocumentLease,
        diagnostics: Vec<String>,
        source_path: PathBuf,
        original_bytes: Option<Vec<u8>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let workbench = cx.new(|cx| MdeWorkbenchItem::new(lease.source.clone(), cx));
        let workbench_subscription = cx.observe(&workbench, |_, _, cx| cx.notify());
        let subscription = cx.subscribe(&buffer, |this, _, event, cx| {
            let reason = match event {
                BufferEvent::Edited { .. } | BufferEvent::Reloaded => {
                    Some(NativeDocumentConflict::TextChanged)
                }
                BufferEvent::FileHandleChanged => Some(NativeDocumentConflict::FileChanged),
                BufferEvent::ReloadNeeded => Some(NativeDocumentConflict::ReloadPending),
                _ => None,
            };
            if let Some(reason) = reason {
                this.baseline.invalidate(reason);
                this.lease.source.block_native_edits(true);
                cx.emit(ItemEvent::UpdateTab);
            }
            if this
                .baseline
                .check(&buffer_state(&this.buffer, cx))
                .is_err()
            {
                this.lease.source.block_native_edits(true);
                cx.emit(ItemEvent::UpdateTab);
            }
            cx.notify();
        });
        let mut revisions = lease.source.revisions.clone();
        let revision_task = cx.spawn(async move |this, cx| {
            while revisions.changed().await.is_ok() {
                if this
                    .update(cx, |_, cx| {
                        cx.emit(ItemEvent::Edit);
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
            source_path,
            original_bytes,
            saving: false,
            discarded: false,
            recovery: None,
            saved_copy: None,
            buffer,
            baseline,
            workbench,
            lease,
            diagnostics,
            diagnostic_page: 0,
            _subscription: subscription,
            _workbench_subscription: workbench_subscription,
            _revision_task: revision_task,
        }
    }
}
impl EventEmitter<ItemEvent> for NativeMdePreviewItem {}
impl Focusable for NativeMdePreviewItem {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.workbench.read(cx).focus_handle(cx)
    }
}
impl Item for NativeMdePreviewItem {
    type Event = ItemEvent;
    fn to_item_events(event: &ItemEvent, callback: &mut dyn FnMut(ItemEvent)) {
        callback(*event);
    }
    fn buffer_kind(&self, _: &App) -> ItemBufferKind {
        ItemBufferKind::Singleton
    }
    fn can_save(&self, _: &App) -> bool {
        !self.discarded && self.lease.save_copy.is_some()
    }
    fn can_save_as(&self, _: &App) -> bool {
        false
    }
    fn is_dirty(&self, cx: &App) -> bool {
        if self.discarded {
            return false;
        }
        if let Some(saved) = &self.saved_copy {
            return self.saving
                || self
                    .baseline
                    .check(&buffer_state(&self.buffer, cx))
                    .is_err()
                || !self.lease.source.snapshot_is_current()
                || self
                    .lease
                    .source
                    .mdj_session
                    .as_ref()
                    .is_none_or(|session| {
                        session.is_pending()
                            || !session
                                .snapshot()
                                .is_ok_and(|current| Arc::ptr_eq(saved, &current))
                    });
        }
        self.lease.source.is_mdj()
            && (!self.lease.source.snapshot_is_current()
                || self.saving
                || self.recovery.is_some()
                || self
                    .lease
                    .source
                    .mdj_session
                    .as_ref()
                    .is_some_and(|session| session.is_pending())
                || self
                    .lease
                    .source
                    .projects
                    .is_dirty(&self.lease.source.project_id)
                    .unwrap_or(true))
    }
    fn reload(
        &mut self,
        _project: Entity<project::Project>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Task<anyhow::Result<()>> {
        self.discarded = true;
        self.lease.source.block_native_edits(true);
        self.lease.save_copy = None;
        if let Some(cleanup) = self.lease.cleanup.take() {
            cleanup();
        }
        self.recovery = Some("MDJ tab discarded. The original Buffer was not changed. Close and reopen the source to continue.".into());
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
        gpui::Task::ready(Ok(()))
    }
    fn save(
        &mut self,
        options: SaveOptions,
        project: Entity<project::Project>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Task<anyhow::Result<()>> {
        if self.discarded {
            return gpui::Task::ready(Err(anyhow::anyhow!(
                "This MDJ tab was discarded; reopen the source"
            )));
        }
        if self.saved_copy.is_some() && !self.is_dirty(cx) {
            return gpui::Task::ready(Ok(()));
        }
        if self.saving || self.saved_copy.is_some() {
            return gpui::Task::ready(Err(anyhow::anyhow!(
                self.recovery
                    .clone()
                    .unwrap_or_else(|| "Save is already pending".into())
            )));
        }
        if options.autosave {
            return gpui::Task::ready(Err(anyhow::anyhow!(
                "MDJ requires an explicit Save a copy; autosave never opens a dialog"
            )));
        }
        if !self.is_dirty(cx) {
            return gpui::Task::ready(Ok(()));
        }
        let ready = (|| {
            self.baseline
                .check(&buffer_state(&self.buffer, cx))
                .map_err(|error| error.message().to_owned())?;
            if !self.lease.source.snapshot_is_current() {
                return Err("MDJ session changed; save was blocked".into());
            }
            let session = self
                .lease
                .source
                .mdj_session
                .as_ref()
                .ok_or("Read-only document")?;
            if session.is_pending() {
                return Err("An edit is still pending; retry after it completes".into());
            }
            Ok::<_, String>((
                session.snapshot()?,
                self.lease
                    .save_copy
                    .clone()
                    .ok_or("Save a copy is unavailable")?,
            ))
        })();
        let (expected, save_copy) = match ready {
            Ok(ready) => ready,
            Err(error) => return gpui::Task::ready(Err(anyhow::anyhow!(error))),
        };
        let directory = self
            .source_path
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let prompt = cx.prompt_for_new_path(directory, Some("model-copy.mdj"));
        let fs = project.read(cx).fs().clone();
        let source_path = self.source_path.clone();
        let original_bytes = self.original_bytes.clone();
        self.saving = true;
        self.lease.source.block_native_edits(true);
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result: anyhow::Result<()> = async {
                let path = prompt.await??.ok_or_else(|| anyhow::anyhow!("Save a copy cancelled; changes remain unsaved"))?;
                anyhow::ensure!(path.extension().and_then(|value| value.to_str()).is_some_and(|value| value.eq_ignore_ascii_case("mdj")), "Choose a new .mdj filename");
                let parent = path.parent().ok_or_else(|| anyhow::anyhow!("Destination has no parent directory"))?;
                let filename = path.file_name().ok_or_else(|| anyhow::anyhow!("Destination has no filename"))?;
                let destination = fs.canonicalize(parent).await?.join(filename);
                let source_parent = source_path.parent().ok_or_else(|| anyhow::anyhow!("Source has no parent directory"))?;
                let source_name = source_path.file_name().ok_or_else(|| anyhow::anyhow!("Source has no filename"))?;
                let source_identity = fs.canonicalize(source_parent).await?.join(source_name);
                let source_target = fs.canonicalize(&source_path).await?;
                anyhow::ensure!(destination != source_identity && destination != source_target, "Save a copy must use a different filename from the original source");
                let dirty_paths = this.update(cx, |_, cx| dirty_buffer_paths(&project, cx))?;
                for dirty in &dirty_paths {
                    let equivalent = if let (Some(parent), Some(filename)) = (dirty.parent(), dirty.file_name()) {
                        fs.canonicalize(parent).await?.join(filename) == destination
                    } else { dirty == &path };
                    anyhow::ensure!(!equivalent, "The destination has an unsaved text Buffer; choose another filename");
                }
                let raw = fs.load_bytes(&source_path).await?;
                anyhow::ensure!(original_bytes.as_deref() == Some(raw.as_slice()), "The original source changed on disk; save was blocked");
                this.update(cx, |this, cx| {
                    anyhow::ensure!(!this.discarded, "This MDJ tab was discarded; copy save was cancelled");
                    this.baseline.check(&buffer_state(&this.buffer, cx)).map_err(|error| anyhow::anyhow!(error.message()))?;
                    anyhow::ensure!(this.lease.source.snapshot_is_current(), "MDJ session changed; save was blocked");
                    anyhow::ensure!(dirty_buffer_paths(&project, cx) == dirty_paths, "Open dirty Buffers changed during save preparation; retry Save a copy");
                    Ok::<_, anyhow::Error>(())
                })??;
                let outcome = save_copy(Arc::clone(&expected), path.clone()).await;
                let disk_unchanged = fs.load_bytes(&source_path).await.is_ok_and(|bytes| original_bytes.as_deref() == Some(bytes.as_slice()));
                this.update(cx, |this, cx| {
                    if this.discarded { return Err(anyhow::anyhow!("MDJ tab was discarded while saving to {}; inspect the copy destination for the final outcome", path.display())); }
                    let source_changed = !disk_unchanged || dirty_buffer_paths(&project, cx) != dirty_paths || this.baseline.check(&buffer_state(&this.buffer, cx)).is_err();
                    match outcome {
                        Ok(warning) => {
                            let current = this.lease.source.mdj_session.as_ref().is_some_and(|session| !session.is_pending() && session.snapshot().is_ok_and(|current| Arc::ptr_eq(&expected, &current)));
                            if source_changed || !current || !this.lease.source.snapshot_is_current() {
                                this.recovery = Some(format!("Copy written to {}, but source or session changed during save. This tab retains unsaved changes; the original was not modified.", path.display()));
                                return Err(anyhow::anyhow!(this.recovery.clone().unwrap_or_default()));
                            }
                            this.saved_copy = Some(expected);
                            let mut message = format!("MDJ copy saved to {}. The original was not modified. Close this tab and reopen the copy to continue editing.", path.display());
                            if let Some(warning) = warning { message.push_str(&format!(" {warning}")); }
                            this.recovery = Some(message);
                            Ok(())
                        }
                        Err(error) => {
                            this.recovery = Some(format!("Copy save to {} did not complete: {error}. Keep this tab open and retry Save a copy to recover your changes; do not assume the original was saved.", path.display()));
                            Err(anyhow::anyhow!(error))
                        }
                    }
                })?
            }.await;
            this.update(cx, |this, cx| {
                this.saving = false;
                let blocked = this.discarded || this.recovery.is_some() || this.baseline.check(&buffer_state(&this.buffer, cx)).is_err();
                this.lease.source.block_native_edits(blocked);
                cx.emit(ItemEvent::UpdateTab);
                cx.notify();
            })?;
            result
        })
    }
    fn tab_content_text(&self, _: usize, _: &App) -> SharedString {
        format!(
            "{} · {}",
            if self.lease.source.is_mdj() {
                "MDJ · Save a copy"
            } else {
                "MDE Preview (read-only)"
            },
            self.lease.source.project_id.as_str()
        )
        .into()
    }
    fn active_project_path(&self, cx: &App) -> Option<ProjectPath> {
        if self.discarded || self.saved_copy.is_some() {
            return None;
        }
        self.buffer
            .read(cx)
            .file()
            .map(|file| ProjectPath::from_file(file.as_ref(), cx))
    }
    fn show_toolbar(&self) -> bool {
        false
    }
}
impl Render for NativeMdePreviewItem {
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
            "The imported semantic state changed. This preview retains its original snapshot; resolve those changes before reopening."
        } else {
            self.baseline.check(&buffer_state(&self.buffer, cx)).err().map(NativeDocumentConflict::message)
            .unwrap_or(if self.lease.source.is_mdj() { "Limited MDJ editing. Save writes a new copy; the original source remains unchanged." } else { "Read-only MDE preview. Edit the source in the native editor; close and reopen this preview to refresh." })
        };
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .child(Label::new(if self.lease.source.is_mdj() {
                        "MDJ · limited editing"
                    } else {
                        "Read-only preview"
                    }))
                    .child(
                        DropdownMenu::new("mde-preview-diagram", label, menu)
                            .style(DropdownStyle::Subtle),
                    )
                    .child(
                        Button::new(
                            "mde-preview-validate",
                            if self.lease.source.is_mdj() {
                                "MDJ validation unavailable"
                            } else {
                                "Validate preview"
                            },
                        )
                        .disabled(self.lease.source.is_mdj())
                        .on_click(move |_, _, cx| {
                            validate_owner.update(cx, |item, cx| item.validate_diagram(cx));
                        }),
                    ),
            )
            .child(Label::new(message))
            .when(!self.diagnostics.is_empty(), |preview| {
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
            })
            .when(!self.discarded, |container| {
                container.child(div().flex_1().min_h_0().child(self.workbench.clone()))
            })
    }
}
