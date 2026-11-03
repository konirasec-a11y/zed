use crate::native_document_guard::{
    NativeBufferState, NativeDocumentConflict, NativeDocumentGuard,
};
use crate::{MdeUiSource, MdeWorkbenchItem, from_document};
use bellatrix_project::ProjectDocument;
use gpui::{
    App, AppContext, Context, Entity, EntityId, EventEmitter, FocusHandle, Focusable,
    PathPromptOptions, Subscription, Window, actions,
};
use language::{Buffer, BufferEvent, BufferSnapshot, DiskState};
use project::{DirectoryLister, ProjectPath};
use std::{cell::Cell, future::Future, pin::Pin, rc::Rc, sync::Arc};
use ui::{ContextMenu, DropdownMenu, DropdownStyle, prelude::*};
use workspace::{Workspace, item::Item};

type ImportFuture = Pin<Box<dyn Future<Output = Result<NativeDocumentLease, String>> + Send>>;

#[derive(Clone)]
pub struct NativeDocumentImporter(Arc<dyn Fn(ProjectDocument, bool) -> ImportFuture + Send + Sync>);

impl NativeDocumentImporter {
    pub fn new(
        import: impl Fn(ProjectDocument, bool) -> ImportFuture + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(import))
    }
}

pub struct NativeDocumentLease {
    source: MdeUiSource,
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl NativeDocumentLease {
    pub fn new(source: MdeUiSource, cleanup: impl FnOnce() + Send + 'static) -> Self {
        Self {
            source,
            cleanup: Some(Box::new(cleanup)),
        }
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
    crate::panels::attach_panels(workspace, workbench, window, cx);
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
                prompt: Some("Open MDE project preview (read-only)".into()),
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
                let text = state.version.0.text();
                let baseline =
                    NativeDocumentGuard::new(state).map_err(|error| error.message().to_owned())?;
                let document: ProjectDocument = serde_json::from_str(&text)
                    .map_err(|error| format!("Invalid Bellatrix project document: {error}"))?;
                let snapshot = from_document(&document)?;
                bellatrix_mde::Graph::new(
                    &snapshot
                        .section
                        .to_model(document.id().as_str(), document.name()),
                )
                .validate()
                .map_err(|error| error.to_string())?;
                let lease = (importer.0)(document, baseline.is_saved()).await?;
                workspace
                    .update_in(cx, |workspace, window, cx| {
                        baseline
                            .check(&buffer_state(&buffer, cx))
                            .map_err(|error| error.message().to_owned())?;
                        let item =
                            cx.new(|cx| NativeMdePreviewItem::new(buffer, baseline, lease, cx));
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
    buffer: Entity<Buffer>,
    baseline: NativeDocumentGuard<NativeVersion, NativeIdentity>,
    workbench: Entity<MdeWorkbenchItem>,
    lease: NativeDocumentLease,
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
            .map(|snapshot| format!("{} ({id})", snapshot.name))
            .unwrap_or_else(|_| id.to_owned())
    }

    fn new(
        buffer: Entity<Buffer>,
        baseline: NativeDocumentGuard<NativeVersion, NativeIdentity>,
        lease: NativeDocumentLease,
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
            }
            cx.notify();
        });
        let mut revisions = lease.source.revisions.clone();
        let revision_task = cx.spawn(async move |this, cx| {
            while revisions.changed().await.is_ok() {
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        Self {
            buffer,
            baseline,
            workbench,
            lease,
            _subscription: subscription,
            _workbench_subscription: workbench_subscription,
            _revision_task: revision_task,
        }
    }
}
impl EventEmitter<()> for NativeMdePreviewItem {}
impl Focusable for NativeMdePreviewItem {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.workbench.read(cx).focus_handle(cx)
    }
}
impl Item for NativeMdePreviewItem {
    type Event = ();
    fn tab_content_text(&self, _: usize, _: &App) -> SharedString {
        format!(
            "MDE Preview (read-only) · {}",
            self.lease.source.project_id.as_str()
        )
        .into()
    }
    fn active_project_path(&self, cx: &App) -> Option<ProjectPath> {
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
        let message = if !self.lease.source.snapshot_is_current() {
            "The imported semantic state changed. This preview retains its original snapshot; resolve those changes before reopening."
        } else {
            self.baseline.check(&buffer_state(&self.buffer, cx)).err().map(NativeDocumentConflict::message)
            .unwrap_or("Read-only MDE preview. Edit the source in the native editor; close and reopen this preview to refresh.")
        };
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .child(Label::new("Read-only preview"))
                    .child(
                        DropdownMenu::new("mde-preview-diagram", label, menu)
                            .style(DropdownStyle::Subtle),
                    )
                    .child(
                        Button::new("mde-preview-validate", "Validate preview").on_click(
                            move |_, _, cx| {
                                validate_owner.update(cx, |item, cx| item.validate_diagram(cx));
                            },
                        ),
                    ),
            )
            .child(Label::new(message))
            .child(div().flex_1().min_h_0().child(self.workbench.clone()))
    }
}
