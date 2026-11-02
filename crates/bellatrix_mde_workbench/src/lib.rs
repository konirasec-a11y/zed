//! Engine-MDE as a native Zed Workspace Item.
//!
//! Zed owns the GPUI runtime, tabs, focus and docks. Cordis owns domain
//! operations. The item stores only transient gestures/selection.

mod snapshot;
mod source;
pub mod panels;
mod integration;
mod theme;
mod hierarchy;
mod property_editor;
mod symbol_library;
pub use symbol_library::MdeSymbolLibraryItem;
pub use integration::register_mde_workspace;
pub use theme::WorkbenchPalette;

pub use snapshot::{LEGACY_MODELING_NAMESPACE, MDE_NAMESPACE, MdeSnapshot, from_document};
pub use source::MdeUiSource;

use bellatrix_mde_cordis::{
    MdeUiCommand, ModelingCreateElementRequested, ModelingElementKind,
    ModelingMoveViewRequested, ModelingRenameRequested, ModelingResizeViewRequested,
    ModelingDeleteElementRequested, ModelingSetPropertyRequested,
};
use bellatrix_mde_model::Geometry;
use bellatrix_ui_uml::UmlClassCard;
use bellatrix_project::{ProjectId, ProjectSaveRequested};
use gpui::{
    App, Context, DragMoveEvent, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Point, Pixels, Render,
    KeyDownEvent, ScrollEvent,
    SharedString, Task, Window, div, prelude::*, px,
};
use std::path::PathBuf;
use ui::{Clickable, Label};
use workspace::{Workspace, item::Item};

#[derive(Clone)]
pub struct MdeToolDrag {
    pub project_id: ProjectId,
    pub kind: ModelingElementKind,
}

impl Render for MdeToolDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let name = match self.kind {
            ModelingElementKind::Class => "Class",
            ModelingElementKind::Interface => "Interface",
        };
        div()
            .w(px(140.0))
            .h(px(48.0))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(palette.selection)
            .bg(palette.surface)
            .text_color(palette.text)
            .child(Label::new(name))
    }
}

enum CanvasGesture {
    Move {
        view_id: String,
        start: Point<Pixels>,
        dx: f64,
        dy: f64,
        revision: u64,
    },
    Resize {
        view_id: String,
        start: Point<Pixels>,
        width: f64,
        height: f64,
        dx: f64,
        dy: f64,
        revision: u64,
    },
}

/// A Zed Item (document tab), with no GPUI Kit types or second dock runtime.
pub struct MdeWorkbenchItem {
    source: MdeUiSource,
    focus_handle: FocusHandle,
    selected_element: Option<String>,
    active_diagram: Option<String>,
    selected_tool: Option<ModelingElementKind>,
    gesture: Option<CanvasGesture>,
    drop_point: Option<(f64, f64)>,
    // Viewport is strictly transient UI state, never part of ProjectDocument.
    zoom: f32,
    pan: (f32, f32),
    pan_origin: Option<(Point<Pixels>, (f32, f32))>,
    feedback: String,
    _watch_task: Task<()>,
}

impl MdeWorkbenchItem {
    pub fn new(source: MdeUiSource, cx: &mut Context<Self>) -> Self {
        let initial_diagram = source.snapshot().ok()
            .and_then(|s| s.section.diagrams.first().map(|d| d.id.clone()));
        let mut revisions = source.revisions.clone();
        let mut feedback = source.feedback.clone();
        let task = cx.spawn(async move |this, cx| {
            loop {
                tokio::select! {
                    changed = revisions.changed() => {
                        if changed.is_err() { break; }
                        if this.update(cx, |item, cx| item.refresh(cx)).is_err() { break; }
                    }
                    changed = feedback.changed() => {
                        if changed.is_err() { break; }
                        let text = feedback.borrow().clone();
                        if this.update(cx, |item, cx| {
                            item.feedback = text;
                            cx.notify();
                        }).is_err() { break; }
                    }
                }
            }
        });
        Self {
            source,
            focus_handle: cx.focus_handle(),
            selected_element: None,
            active_diagram: initial_diagram,
            selected_tool: None,
            gesture: None,
            drop_point: None,
            zoom: 1.0,
            pan: (0.0, 0.0),
            pan_origin: None,
            feedback: String::new(),
            _watch_task: task,
        }
    }

    pub fn source(&self) -> &MdeUiSource {
        &self.source
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.source.project_id
    }

    pub fn selected_element(&self) -> Option<&str> {
        self.selected_element.as_deref()
    }

    pub fn selected_tool(&self) -> Option<ModelingElementKind> {
        self.selected_tool
    }

    pub fn active_diagram(&self) -> Option<&str> {
        self.active_diagram.as_deref()
    }

    pub fn select_element(&mut self, id: String, cx: &mut Context<Self>) {
        if self.selected_element.as_deref() != Some(id.as_str()) {
            self.selected_element = Some(id);
            cx.notify();
        }
    }

    pub fn select_diagram(&mut self, id: String, cx: &mut Context<Self>) {
        self.active_diagram = Some(id);
        self.gesture = None;
        cx.notify();
    }

    pub fn select_tool(&mut self, kind: ModelingElementKind, cx: &mut Context<Self>) {
        self.selected_tool = if self.selected_tool == Some(kind) { None } else { Some(kind) };
        cx.notify();
    }


    pub fn set_zoom(&mut self, value: f32, cx: &mut Context<Self>) {
        self.zoom = value.clamp(0.25, 3.0);
        cx.notify();
    }

    pub fn reset_viewport(&mut self, cx: &mut Context<Self>) {
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
        self.pan_origin = None;
        cx.notify();
    }

    fn scroll_viewport(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.0));
        let dx = f32::from(delta.x);
        let dy = f32::from(delta.y);
        if event.modifiers.control {
            self.set_zoom(self.zoom * if dy > 0.0 { 1.12 } else { 1.0 / 1.12 }, cx);
        } else {
            // Wheel and two-finger trackpad movement scroll the virtual canvas
            // without changing the persisted geometry of its elements.
            self.pan.0 += dx;
            self.pan.1 += dy;
            cx.notify();
        }
    }

    fn start_pan(&mut self, position: Point<Pixels>) {
        self.pan_origin = Some((position, self.pan));
    }

    fn update_pan(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Middle) { return; }
        if let Some((position, (x, y))) = &self.pan_origin {
            self.pan = (
                *x + f32::from(event.position.x - position.x),
                *y + f32::from(event.position.y - position.y),
            );
            cx.notify();
        }
    }

    /// User-editable Inspector values are typed and revision-checked in MDE.
    /// ProjectService remains authoritative; this is only a command proposal.
    pub fn set_selected_property(
        &mut self, path: String, value: serde_json::Value, cx: &mut Context<Self>
    ) {
        let Some(element_id) = self.selected_element.clone() else {
            self.feedback = "Select a model element first".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        self.submit(MdeUiCommand::SetProperty(ModelingSetPropertyRequested {
            project_id: self.project_id().clone(),
            element_id,
            path,
            value,
            expected_revision: snapshot.revision,
        }), cx);
    }

    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(element_id) = self.selected_element.clone() else {
            self.feedback = "Select an element to delete".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        // Canvas deletion affects only subjects with a diagram View.
        // A Project Explorer file/directory selection is never silently removed.
        if !snapshot.section.diagrams.iter().any(|diagram| diagram.views.iter().any(
            |view| view.subject.as_deref() == Some(element_id.as_str())
        )) {
            self.feedback = "Delete is available for visible diagram elements only".into();
            cx.notify();
            return;
        }
        // All semantic deletes are versioned Cordis requests and preserve
        // unrelated models, filesystem nodes and user-created documents.
        self.submit(MdeUiCommand::DeleteElement(ModelingDeleteElementRequested {
            project_id: self.project_id().clone(),
            element_id,
            expected_revision: snapshot.revision,
        }), cx);
        self.selected_element = None;
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        // A newly confirmed Project revision makes any preview stale.
        self.gesture = None;
        cx.notify();
    }

    fn submit(&mut self, command: MdeUiCommand, cx: &mut Context<Self>) {
        self.feedback = match self.source.submit(command) {
            Ok(()) => "Command sent to Cordis".into(),
            Err(error) => format!("Command not sent: {error}"),
        };
        cx.notify();
    }

    pub fn rename_selected(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(id) = self.selected_element.clone() else {
            self.feedback = "Select an element first".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        self.submit(MdeUiCommand::Rename(ModelingRenameRequested {
            project_id: self.project_id().clone(),
            element_id: id,
            new_name: name,
            expected_revision: snapshot.revision,
        }), cx);
    }

    pub fn save_project(&mut self, path: String, cx: &mut Context<Self>) {
        if path.trim().is_empty() {
            self.feedback = "Specify a project file".into();
            cx.notify();
            return;
        }
        self.submit(MdeUiCommand::SaveProject(ProjectSaveRequested {
            id: self.project_id().clone(),
            path: PathBuf::from(path.trim()),
        }), cx);
    }

    pub fn create_element(
        &mut self,
        kind: ModelingElementKind,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let diagram_id = self.active_diagram.as_deref()
            .and_then(|id| snapshot.section.diagrams.iter().find(|d| d.id == id))
            .or_else(|| snapshot.section.diagrams.first())
            .map(|d| d.id.clone());
        let Some(diagram_id) = diagram_id else {
            self.feedback = "Project has no diagram".into();
            cx.notify();
            return;
        };
        self.submit(MdeUiCommand::CreateElement(ModelingCreateElementRequested {
            project_id: self.project_id().clone(),
            diagram_id,
            kind,
            name: None,
            x: x.max(0.0),
            y: y.max(0.0),
            expected_revision: snapshot.revision,
        }), cx);
        self.selected_tool = None;
    }

    /// Toolbar click fallback for placing a symbol. Drag/drop uses the true
    /// canvas-local coordinates from the GPUI DragMoveEvent instead.
    pub fn create_in_next_slot(&mut self, kind: ModelingElementKind, cx: &mut Context<Self>) {
        let count = self.source.snapshot().ok()
            .and_then(|s| s.section.diagrams.first().map(|d| d.views.len()))
            .unwrap_or(0);
        let x = 24.0 + ((count % 4) as f64 * 200.0);
        let y = 36.0 + ((count / 4) as f64 * 148.0);
        self.create_element(kind, x, y, cx);
    }

    fn start_move(
        &mut self,
        view_id: String,
        subject: String,
        revision: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.select_element(subject, cx);
        self.gesture = Some(CanvasGesture::Move {
            view_id, start: position, dx: 0.0, dy: 0.0, revision,
        });
    }

    fn start_resize(
        &mut self,
        view_id: String,
        width: f64,
        height: f64,
        revision: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.gesture = Some(CanvasGesture::Resize {
            view_id, start: position, width, height, dx: 0.0, dy: 0.0, revision,
        });
        cx.notify();
    }

    fn update_gesture(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !event.dragging() {
            return;
        }
        let Some(gesture) = &mut self.gesture else { return };
        let (start, dx, dy) = match gesture {
            CanvasGesture::Move { start, dx, dy, .. }
            | CanvasGesture::Resize { start, dx, dy, .. } => (start, dx, dy),
        };
        *dx = f64::from(event.position.x - start.x) / f64::from(self.zoom);
        *dy = f64::from(event.position.y - start.y) / f64::from(self.zoom);
        cx.notify();
    }

    fn finish_gesture(&mut self, cx: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else { return };
        let expected_revision = match &gesture {
            CanvasGesture::Move { revision, .. }
            | CanvasGesture::Resize { revision, .. } => *revision,
        };
        if self.source.snapshot().ok().map(|s| s.revision) != Some(expected_revision) {
            self.feedback = "Project changed during drag; retry".into();
            cx.notify();
            return;
        }
        let command = match gesture {
            CanvasGesture::Move { view_id, dx, dy, .. } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    cx.notify();
                    return;
                }
                MdeUiCommand::MoveView(ModelingMoveViewRequested {
                    project_id: self.project_id().clone(),
                    view_id, dx, dy, expected_revision,
                })
            }
            CanvasGesture::Resize { view_id, width, height, dx, dy, .. } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    cx.notify();
                    return;
                }
                MdeUiCommand::ResizeView(ModelingResizeViewRequested {
                    project_id: self.project_id().clone(),
                    view_id,
                    width: (width + dx).max(80.0),
                    height: (height + dy).max(48.0),
                    expected_revision,
                })
            }
        };
        self.submit(command, cx);
    }
}

pub fn open_in_workspace(
    workspace: &mut Workspace,
    source: MdeUiSource,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Entity<MdeWorkbenchItem> {
    let item = cx.new(|cx| MdeWorkbenchItem::new(source, cx));
    workspace.add_item_to_active_pane(Box::new(item.clone()), None, true, window, cx);
    item
}

impl EventEmitter<()> for MdeWorkbenchItem {}

impl Focusable for MdeWorkbenchItem {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for MdeWorkbenchItem {
    type Event = ();

    fn tab_content_text(&self, _: usize, _: &App) -> SharedString {
        self.source.projects.snapshot(&self.source.project_id)
            .map(|document| format!("MDE · {}", document.name()).into())
            .unwrap_or_else(|_| "MDE Workbench".into())
    }

    fn show_toolbar(&self) -> bool { false }
}

impl Render for MdeWorkbenchItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let background = palette.canvas;
        let surface = palette.surface;
        let border = palette.border;
        let selection = palette.selection;
        let muted = palette.muted;
        let zoom = self.zoom;
        let pan = self.pan;

        let mut title = format!("MDE · {}", self.project_id().as_str());
        let details: String;
        let mut cards = Vec::new();

        match self.source.snapshot() {
            Ok(snapshot) => {
                let diagram = self.active_diagram.as_deref()
                    .and_then(|id| snapshot.section.diagrams.iter().find(|d| d.id == id))
                    .or_else(|| snapshot.section.diagrams.first());
                details = format!(
                    "{} elements · {} relations · rev {}",
                    snapshot.section.elements.len(), snapshot.section.relations.len(),
                    snapshot.revision,
                );
                if let Some(diagram) = diagram {
                    title = format!("{} / {}", snapshot.name, diagram.name);
                    let mut views = diagram.views.iter().collect::<Vec<_>>();
                    views.sort_by_key(|v| v.z_index);
                    for view in views {
                        let Some(subject) = view.subject.as_deref() else { continue };
                        let Some(element) = snapshot.section.elements.iter()
                            .find(|e| e.id == subject) else { continue };
                        let Geometry::Rect { x, y, width, height } = &view.geometry else { continue };
                        let (x, y, width, height) = (*x, *y, *width, *height);
                        let revision = snapshot.revision;
                        if ![x,y,width,height].iter().all(|n| n.is_finite()) { continue }
                        let mut px_x = x as f32;
                        let mut px_y = y as f32;
                        let mut px_w = width as f32;
                        let mut px_h = height as f32;
                        match &self.gesture {
                            Some(CanvasGesture::Move { view_id, dx, dy, .. }) if view_id == &view.id => {
                                px_x += *dx as f32;
                                px_y += *dy as f32;
                            }
                            Some(CanvasGesture::Resize { view_id, dx, dy, .. }) if view_id == &view.id => {
                                px_w = (width + dx).max(80.0) as f32;
                                px_h = (height + dy).max(48.0) as f32;
                            }
                            _ => {}
                        }
                        let is_selected = self.selected_element.as_deref() == Some(subject);
                        let mut class_card = UmlClassCard::new(element.name.clone())
                            .selected(is_selected);
                        for relation in &snapshot.section.relations {
                            if relation.kind != "owns" || relation.from != element.id {
                                continue;
                            }
                            let Some(member) = snapshot.section.elements.iter()
                                .find(|candidate| candidate.id == relation.to) else {
                                continue;
                            };
                            if member.kind == "rust.field" {
                                class_card = class_card.attribute(member.name.clone());
                            } else if member.kind.starts_with("rust.") {
                                class_card = class_card.operation(member.name.clone());
                            }
                        }
                        let drag_owner = cx.weak_entity();
                        let drag_id = view.id.clone();
                        let resize_owner = cx.weak_entity();
                        let resize_id = view.id.clone();
                        let element_id = subject.to_owned();
                        cards.push(
                            div()
                                .id(format!("mde-view-{}", view.id))
                                .absolute()
                                .left(px(px_x * zoom + pan.0))
                                .top(px(px_y * zoom + pan.1))
                                .w(px(px_w * zoom))
                                .h(px(px_h * zoom))
                                .bg(surface)
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, window, cx| {
                                    let _ = drag_owner.update(cx, |item, cx| {
                                        item.start_move(drag_id.clone(), element_id.clone(),
                                            revision, event.position, cx);
                                        cx.focus_self(window);
                                    });
                                    cx.stop_propagation();
                                })
                                .child(
                                    class_card
                                )
                                .child(
                                    div()
                                        .id(format!("mde-resize-{}", view.id))
                                        .absolute()
                                        .bottom(px(0.0))
                                        .right(px(0.0))
                                        .w(px(14.0))
                                        .h(px(14.0))
                                        .bg(selection)
                                        .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                                            let _ = resize_owner.update(cx, |item, cx| {
                                                item.start_resize(resize_id.clone(), width, height,
                                                    revision, event.position, cx);
                                            });
                                            cx.stop_propagation();
                                        }),
                                ),
                        );
                    }
                }
            }
            Err(error) => details = error,
        }

        let owner = cx.weak_entity();
        let move_owner = cx.weak_entity();
        let up_owner = cx.weak_entity();
        let out_owner = cx.weak_entity();
        let dragging_owner = cx.weak_entity();
        let dropping_owner = cx.weak_entity();
        let wheel_owner = cx.weak_entity();
        let pan_start_owner = cx.weak_entity();
        let pan_end_owner = cx.weak_entity();
        let pan_out_owner = cx.weak_entity();
        let delete_owner = cx.weak_entity();
        let zoom_in_owner = cx.weak_entity();
        let zoom_out_owner = cx.weak_entity();
        let reset_owner = cx.weak_entity();

        div()
            .id(format!("mde-workbench-{}", self.project_id().as_str()))
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .on_key_down(move |event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "delete" | "backspace") {
                    let _ = delete_owner.update(cx, |item, cx| item.delete_selected(cx));
                    cx.stop_propagation();
                }
            })
            .bg(background)
            .text_color(palette.text)
            .child(
                div()
                    .h(px(39.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(border)
                    .child(Label::new(title))
                    .child(
                        div()
                            .flex().gap_2().items_center()
                            .child(Label::new(details))
                            .child(
                                ui::Button::new("mde-zoom-out", "-").on_click(move |_, _, cx| {
                                    let _ = zoom_out_owner.update(cx, |item, cx| {
                                        item.set_zoom(item.zoom / 1.2, cx);
                                    });
                                }),
                            )
                            .child(Label::new(format!("{:.0}%", zoom * 100.0)))
                            .child(
                                ui::Button::new("mde-zoom-in", "+").on_click(move |_, _, cx| {
                                    let _ = zoom_in_owner.update(cx, |item, cx| {
                                        item.set_zoom(item.zoom * 1.2, cx);
                                    });
                                }),
                            )
                            .child(
                                ui::Button::new("mde-reset-view", "100%").on_click(move |_, _, cx| {
                                    let _ = reset_owner.update(cx, |item, cx| item.reset_viewport(cx));
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .id("mde-canvas")
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .on_scroll(move |event: &ScrollEvent, _, cx| {
                        let _ = wheel_owner.update(cx, |item, cx| item.scroll_viewport(event, cx));
                        cx.stop_propagation();
                    })
                    .on_mouse_down(MouseButton::Middle, move |event: &MouseDownEvent, _, cx| {
                        let _ = pan_start_owner.update(cx, |item, _| item.start_pan(event.position));
                        cx.stop_propagation();
                    })
                    .on_mouse_up(MouseButton::Middle, move |_: &MouseUpEvent, _, cx| {
                        let _ = pan_end_owner.update(cx, |item, _| item.pan_origin = None);
                    })
                    .on_mouse_up_out(MouseButton::Middle, move |_: &MouseUpEvent, _, cx| {
                        let _ = pan_out_owner.update(cx, |item, _| item.pan_origin = None);
                    })
                    .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _, cx| {
                        let _ = owner.update(cx, |item, cx| {
                            if let Some(kind) = item.selected_tool {
                                item.create_in_next_slot(kind, cx);
                            }
                        });
                    })
                    .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                        let _ = move_owner.update(cx, |item, cx| {
                            item.update_pan(event, cx);
                            item.update_gesture(event, cx);
                        });
                    })
                    .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                        let _ = up_owner.update(cx, |item, cx| item.finish_gesture(cx));
                    })
                    .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                        let _ = out_owner.update(cx, |item, cx| item.finish_gesture(cx));
                    })
                    .on_drag_move(move |event: &DragMoveEvent<MdeToolDrag>, _, cx| {
                        let dx = f64::from(event.event.position.x - event.bounds.origin.x);
                        let dy = f64::from(event.event.position.y - event.bounds.origin.y);
                        let _ = dragging_owner.update(cx, |item, _| {
                            item.drop_point = Some((
                                (dx - f64::from(item.pan.0)) / f64::from(item.zoom),
                                (dy - f64::from(item.pan.1)) / f64::from(item.zoom),
                            ));
                        });
                    })
                    .on_drop(move |drag: &MdeToolDrag, _, cx| {
                        let _ = dropping_owner.update(cx, |item, cx| {
                            if &drag.project_id != item.project_id() { return; }
                            let (x,y) = item.drop_point.take().unwrap_or((40.0,40.0));
                            item.create_element(drag.kind,x,y,cx);
                        });
                    })
                    .children(cards),
            )
            .child(
                div()
                    .h(px(25.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .border_t_1()
                    .border_color(border)
                    .text_color(muted)
                    .child(Label::new(self.feedback.clone())),
            )
    }
}
