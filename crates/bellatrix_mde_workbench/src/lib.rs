//! Engine-MDE as a native Zed Workspace Item.
//! Zed owns presentation; bellatrix_mde owns semantics; Cordis publishes revisions.
mod snapshot;
mod source;
pub mod panels;
mod integration;
mod theme;
mod hierarchy;
mod property_editor;
mod quick_edit_view;
mod symbol_library;
pub use symbol_library::MdeSymbolLibraryItem;
pub use integration::register_mde_workspace;
pub use theme::WorkbenchPalette;
pub use snapshot::{LEGACY_MODELING_NAMESPACE, MDE_NAMESPACE, MdeSnapshot, from_document};
pub use source::MdeUiSource;

use bellatrix_mde_cordis::{
    MdeUiCommand, ModelingCreateElementRequested, ModelingCreateRelationRequested,
    ModelingMoveViewRequested, ModelingRenameRequested, ModelingResizeViewRequested,
    ModelingDeleteElementRequested, ModelingSetPropertyRequested,
    ModelingQuickEditRequested, ModelingSetNameExpressionRequested, ModelingAddMemberRequested,
};
use bellatrix_mde::{Geometry, PathSegment, UmlQuickCommand, UmlType, ModelingRelationKind,
    RelationshipGraph, relationship_presentation, classifier_contents, interface_layout};
use bellatrix_mde::commands::{ModelingElementKind, ModelingMemberKind, ModelingTool};
use bellatrix_ui_uml::{UmlClassifier, UmlFigure, UmlLineSegment, UmlRelationGlyph};
use quick_edit_view::quick_edit_for_element;
use ui_input::InputField;
use bellatrix_project::{ProjectId, ProjectSaveRequested};
use gpui::{
    App, AppContext, Context, DragMoveEvent, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Point, Pixels, Render,
    KeyDownEvent, ScrollEvent, SharedString, Task, Window, div, prelude::*, px,
};
use std::path::PathBuf;
use ui::{Clickable, Label};
use workspace::{Workspace, item::Item};

#[derive(Clone)]
pub struct MdeToolDrag { pub project_id: ProjectId, pub kind: ModelingTool }
impl Render for MdeToolDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        div().w(px(140.0)).h(px(48.0)).flex().items_center().justify_center()
            .border_1().border_color(palette.selection).bg(palette.surface)
            .text_color(palette.text).child(Label::new(self.kind.label()))
    }
}
enum CanvasGesture {
    Move { view_id: String, start: Point<Pixels>, dx: f64, dy: f64, revision: u64 },
    Resize { view_id: String, start: Point<Pixels>, width: f64, height: f64, dx: f64, dy: f64, revision: u64 },
}
pub struct MdeWorkbenchItem {
    source: MdeUiSource,
    focus_handle: FocusHandle,
    selected_element: Option<String>,
    inline_editor: Option<Entity<InputField>>,
    editing_subject: Option<String>,
    active_diagram: Option<String>,
    selected_tool: Option<ModelingTool>,
    pending_relation_source: Option<String>,
    gesture: Option<CanvasGesture>,
    drop_point: Option<(f64, f64)>,
    zoom: f32,
    pan: (f32, f32),
    pan_origin: Option<(Point<Pixels>, (f32, f32))>,
    feedback: String,
    _watch_task: Task<()>,
}
#[derive(Debug, PartialEq, Eq)]
enum RelationClick { First, Repeat, Connect(String) }
fn choose_relation_endpoint(pending: &mut Option<String>, clicked: &str) -> RelationClick {
    match pending.take() {
        None => { *pending = Some(clicked.to_owned()); RelationClick::First }
        Some(from) if from == clicked => { *pending = Some(from); RelationClick::Repeat }
        Some(from) => RelationClick::Connect(from),
    }
}

impl MdeWorkbenchItem {
    pub fn new(source: MdeUiSource, cx: &mut Context<Self>) -> Self {
        let initial_diagram = source.snapshot().ok().and_then(|snapshot| snapshot.section.diagrams.first().map(|diagram| diagram.id.clone()));
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
                        if this.update(cx, |item, cx| { item.feedback = text; cx.notify(); }).is_err() { break; }
                    }
                }
            }
        });
        Self { source, focus_handle: cx.focus_handle(), selected_element: None, inline_editor: None,
            editing_subject: None, active_diagram: initial_diagram, selected_tool: None,
            pending_relation_source: None, gesture: None, drop_point: None, zoom: 1.0,
            pan: (0.0, 0.0), pan_origin: None, feedback: String::new(), _watch_task: task }
    }
    pub fn source(&self) -> &MdeUiSource { &self.source }
    pub fn project_id(&self) -> &ProjectId { &self.source.project_id }
    pub fn selected_element(&self) -> Option<&str> { self.selected_element.as_deref() }
    pub fn selected_tool(&self) -> Option<ModelingTool> { self.selected_tool }
    pub fn active_diagram(&self) -> Option<&str> { self.active_diagram.as_deref() }
    pub fn select_element(&mut self, id: String, cx: &mut Context<Self>) {
        if self.selected_element.as_deref() != Some(id.as_str()) {
            self.selected_element = Some(id);
            self.inline_editor = None;
            self.editing_subject = None;
            cx.notify();
        }
    }
    fn name_expression_text(element: &bellatrix_mde::Element) -> String {
        bellatrix_mde::commands::name_expression_text(element)
    }
    pub(crate) fn run_quick_action(&mut self, action: UmlQuickCommand, cx: &mut Context<Self>) {
        let Some(element_id) = self.selected_element.clone() else {
            self.feedback = "Select a UML element first".into(); cx.notify(); return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into(); cx.notify(); return;
        };
        let Some(element) = snapshot.section.elements.iter().find(|element| element.id == element_id) else {
            self.feedback = "Element was removed from the model".into(); cx.notify(); return;
        };
        if !action.accepts(&element.kind) {
            self.feedback = format!("{} does not support this action", element.kind); cx.notify(); return;
        }
        let Some(diagram_id) = self.active_diagram.clone().or_else(|| snapshot.section.diagrams.first().map(|diagram| diagram.id.clone())) else {
            self.feedback = "No active UML diagram".into(); cx.notify(); return;
        };
        self.submit(MdeUiCommand::QuickEdit(ModelingQuickEditRequested {
            project_id: self.project_id().clone(), element_id, diagram_id, action, expected_revision: snapshot.revision,
        }), cx);
    }
    fn begin_inline_edit(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(snapshot) = self.source.snapshot() else { return };
        let Some(target) = snapshot.section.elements.iter().find(|element| element.id == id) else { return };
        if self.editing_subject.as_deref() == Some(id.as_str()) { return; }
        let classifier = UmlType::from_kind(&target.kind).is_some_and(UmlType::is_classifier);
        let hint = if classifier { "<< stereotype >> name" } else { "Name" };
        let field = cx.new(|cx| InputField::new(window, cx, hint).label_min_width(px(96.0)));
        let display = if classifier { Self::name_expression_text(target) } else { target.name.clone() };
        field.update(cx, |field, cx| field.set_text(&display, window, cx));
        self.select_element(id.clone(), cx);
        self.editing_subject = Some(id);
        self.inline_editor = Some(field.clone());
        self.gesture = None;
        field.read(cx).focus_handle(cx).focus(window, cx);
        let text_editor = field.read(cx).editor().clone();
        text_editor.select_all(window, cx);
        cx.notify();
    }
    fn commit_inline_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(id), Some(field)) = (self.editing_subject.clone(), self.inline_editor.clone()) else { return };
        let name = field.read(cx).text(cx).trim().to_owned();
        if name.is_empty() { field.update(cx, |field, cx| field.set_error(Some("Name is required"), cx)); return; }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into(); cx.notify(); return;
        };
        let Some(element) = snapshot.section.elements.iter().find(|element| element.id == id) else {
            self.cancel_inline_edit(window, cx); return;
        };
        let classifier = UmlType::from_kind(&element.kind).is_some_and(UmlType::is_classifier);
        let changed = name != if classifier { Self::name_expression_text(element) } else { element.name.clone() };
        self.editing_subject = None;
        self.inline_editor = None;
        if changed {
            if classifier {
                self.submit(MdeUiCommand::SetNameExpression(ModelingSetNameExpressionRequested {
                    project_id: self.project_id().clone(), element_id: id, expression: name, expected_revision: snapshot.revision,
                }), cx);
            } else {
                self.submit(MdeUiCommand::SetProperty(ModelingSetPropertyRequested {
                    project_id: self.project_id().clone(), element_id: id, path: "name".into(),
                    value: serde_json::Value::String(name), expected_revision: snapshot.revision,
                }), cx);
            }
        }
        self.focus_handle.focus(window, cx);
        cx.notify();
    }
    fn cancel_inline_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_subject = None;
        self.inline_editor = None;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }
    pub fn select_diagram(&mut self, id: String, cx: &mut Context<Self>) {
        self.active_diagram = Some(id);
        self.gesture = None;
        self.pending_relation_source = None;
        self.selected_tool = None;
        self.editing_subject = None;
        self.inline_editor = None;
        cx.notify();
    }
    pub fn select_tool(&mut self, kind: ModelingTool, cx: &mut Context<Self>) {
        if self.selected_tool == Some(kind) { self.cancel_tool(cx); } else { self.arm_drag_tool(kind, cx); }
    }
    pub fn cancel_tool(&mut self, cx: &mut Context<Self>) {
        self.selected_tool = None;
        self.pending_relation_source = None;
        self.feedback = "No relationship tool selected".into();
        cx.notify();
    }
    fn link_endpoint(&mut self, kind: ModelingRelationKind, target: String, cx: &mut Context<Self>) {
        let from = match choose_relation_endpoint(&mut self.pending_relation_source, &target) {
            RelationClick::First => {
                self.select_element(target, cx);
                self.feedback = format!("{}: source selected. Click a second element", kind.label());
                cx.notify(); return;
            }
            RelationClick::Repeat => {
                self.feedback = format!("{}: choose another element to connect", kind.label());
                cx.notify(); return;
            }
            RelationClick::Connect(from) => from,
        };
        self.select_element(target.clone(), cx);
        let Ok(snapshot) = self.source.snapshot() else { self.feedback = "Project snapshot unavailable".into(); cx.notify(); return; };
        let Some(diagram_id) = self.active_diagram.clone().or_else(|| snapshot.section.diagrams.first().map(|diagram| diagram.id.clone())) else {
            self.feedback = "Project has no diagram".into(); cx.notify(); return;
        };
        self.submit(MdeUiCommand::CreateRelation(ModelingCreateRelationRequested {
            project_id: self.project_id().clone(), diagram_id, kind, from, to: target, expected_revision: snapshot.revision,
        }), cx);
        self.selected_tool = None; cx.notify();
    }
    pub fn arm_drag_tool(&mut self, tool: ModelingTool, cx: &mut Context<Self>) {
        self.gesture = None;
        self.inline_editor = None;
        self.editing_subject = None;
        self.selected_tool = Some(tool);
        self.pending_relation_source = None;
        self.feedback = match tool {
            ModelingTool::Relation(kind) => format!("{}: {}", kind.label(), kind.endpoint_instruction()),
            ModelingTool::Element(kind) => format!("{}: click canvas to place", kind.label()),
        }; cx.notify();
    }
    pub fn set_zoom(&mut self, value: f32, cx: &mut Context<Self>) { self.zoom = value.clamp(0.25, 3.0); cx.notify(); }
    pub fn reset_viewport(&mut self, cx: &mut Context<Self>) { self.zoom = 1.0; self.pan = (0.0, 0.0); self.pan_origin = None; cx.notify(); }
    fn scroll_viewport(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.0));
        let delta_x = f32::from(delta.x); let delta_y = f32::from(delta.y);
        if event.modifiers.control { self.set_zoom(self.zoom * if delta_y > 0.0 { 1.12 } else { 1.0 / 1.12 }, cx); }
        else { self.pan.0 += delta_x; self.pan.1 += delta_y; cx.notify(); }
    }
    fn start_pan(&mut self, position: Point<Pixels>) { self.pan_origin = Some((position, self.pan)); }
    fn update_pan(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Middle) { return; }
        if let Some((position, (x, y))) = &self.pan_origin {
            self.pan = (*x + f32::from(event.position.x - position.x), *y + f32::from(event.position.y - position.y)); cx.notify();
        }
    }
    pub fn add_member_to_selected(&mut self, kind: ModelingMemberKind, cx: &mut Context<Self>) {
        let Some(owner_id) = self.selected_element.clone() else { self.feedback = "Select a classifier first".into(); cx.notify(); return; };
        let Ok(snapshot) = self.source.snapshot() else { self.feedback = "Project snapshot unavailable".into(); cx.notify(); return; };
        self.submit(MdeUiCommand::AddMember(ModelingAddMemberRequested {
            project_id: self.project_id().clone(), owner_id, kind, expected_revision: snapshot.revision,
        }), cx);
    }
    pub fn set_selected_property(&mut self, path: String, value: serde_json::Value, cx: &mut Context<Self>) {
        let Some(element_id) = self.selected_element.clone() else { self.feedback = "Select a model element first".into(); cx.notify(); return; };
        let Ok(snapshot) = self.source.snapshot() else { self.feedback = "Project snapshot unavailable".into(); cx.notify(); return; };
        self.submit(MdeUiCommand::SetProperty(ModelingSetPropertyRequested {
            project_id: self.project_id().clone(), element_id, path, value, expected_revision: snapshot.revision,
        }), cx);
    }
    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(element_id) = self.selected_element.clone() else { self.feedback = "Select an element to delete".into(); cx.notify(); return; };
        let Ok(snapshot) = self.source.snapshot() else { self.feedback = "Project snapshot unavailable".into(); cx.notify(); return; };
        if !snapshot.section.diagrams.iter().any(|diagram| diagram.views.iter().any(|view| view.subject.as_deref() == Some(element_id.as_str()))) {
            self.feedback = "Delete is available for visible diagram elements only".into(); cx.notify(); return;
        }
        self.submit(MdeUiCommand::DeleteElement(ModelingDeleteElementRequested {
            project_id: self.project_id().clone(), element_id, expected_revision: snapshot.revision,
        }), cx);
        self.selected_element = None; cx.notify();
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) { self.gesture = None; cx.notify(); }
    fn submit(&mut self, command: MdeUiCommand, cx: &mut Context<Self>) {
        self.feedback = match self.source.submit(command) { Ok(()) => "Command sent to Cordis".into(), Err(error) => format!("Command not sent: {error}") };
        cx.notify();
    }
    pub fn rename_selected(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(id) = self.selected_element.clone() else { self.feedback = "Select an element first".into(); cx.notify(); return; };
        let Ok(snapshot) = self.source.snapshot() else { self.feedback = "Project snapshot unavailable".into(); cx.notify(); return; };
        self.submit(MdeUiCommand::Rename(ModelingRenameRequested { project_id: self.project_id().clone(), element_id: id, new_name: name, expected_revision: snapshot.revision }), cx);
    }
    pub fn save_project(&mut self, path: String, cx: &mut Context<Self>) {
        if path.trim().is_empty() { self.feedback = "Specify a project file".into(); cx.notify(); return; }
        self.submit(MdeUiCommand::SaveProject(ProjectSaveRequested { id: self.project_id().clone(), path: PathBuf::from(path.trim()) }), cx);
    }
    pub fn create_element(&mut self, kind: ModelingElementKind, x: f64, y: f64, cx: &mut Context<Self>) {
        let Ok(snapshot) = self.source.snapshot() else { self.feedback = "Project snapshot unavailable".into(); cx.notify(); return; };
        let diagram_id = self.active_diagram.as_deref().and_then(|id| snapshot.section.diagrams.iter().find(|diagram| diagram.id == id))
            .or_else(|| snapshot.section.diagrams.first()).map(|diagram| diagram.id.clone());
        let Some(diagram_id) = diagram_id else { self.feedback = "Project has no diagram".into(); cx.notify(); return; };
        self.submit(MdeUiCommand::CreateElement(ModelingCreateElementRequested {
            project_id: self.project_id().clone(), diagram_id, kind, name: None, x: x.max(0.0), y: y.max(0.0), expected_revision: snapshot.revision,
        }), cx); self.selected_tool = None;
    }
    pub fn create_in_next_slot(&mut self, kind: ModelingElementKind, cx: &mut Context<Self>) {
        let count = self.source.snapshot().ok().and_then(|snapshot| snapshot.section.diagrams.first().map(|diagram| diagram.views.len())).unwrap_or(0);
        self.create_element(kind, 24.0 + ((count % 4) as f64 * 200.0), 36.0 + ((count / 4) as f64 * 148.0), cx);
    }
    fn start_move(&mut self, view_id: String, subject: String, revision: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(ModelingTool::Relation(kind)) = self.selected_tool { self.link_endpoint(kind, subject, cx); return; }
        self.select_element(subject, cx);
        self.gesture = Some(CanvasGesture::Move { view_id, start: position, dx: 0.0, dy: 0.0, revision });
    }
    fn start_resize(&mut self, view_id: String, width: f64, height: f64, revision: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.gesture = Some(CanvasGesture::Resize { view_id, start: position, width, height, dx: 0.0, dy: 0.0, revision }); cx.notify();
    }
    fn update_gesture(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !event.dragging() { return; }
        let Some(gesture) = &mut self.gesture else { return };
        let (start, dx, dy) = match gesture { CanvasGesture::Move { start, dx, dy, .. } | CanvasGesture::Resize { start, dx, dy, .. } => (start, dx, dy) };
        *dx = f64::from(event.position.x - start.x) / f64::from(self.zoom);
        *dy = f64::from(event.position.y - start.y) / f64::from(self.zoom); cx.notify();
    }
    fn finish_gesture(&mut self, cx: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else { return };
        let expected_revision = match &gesture { CanvasGesture::Move { revision, .. } | CanvasGesture::Resize { revision, .. } => *revision };
        if self.source.snapshot().ok().map(|snapshot| snapshot.revision) != Some(expected_revision) {
            self.feedback = "Project changed during drag; retry".into(); cx.notify(); return;
        }
        let command = match gesture {
            CanvasGesture::Move { view_id, dx, dy, .. } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 { cx.notify(); return; }
                MdeUiCommand::MoveView(ModelingMoveViewRequested { project_id: self.project_id().clone(), view_id, dx, dy, expected_revision })
            }
            CanvasGesture::Resize { view_id, width, height, dx, dy, .. } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 { cx.notify(); return; }
                MdeUiCommand::ResizeView(ModelingResizeViewRequested {
                    project_id: self.project_id().clone(), view_id, width: (width + dx).max(80.0), height: (height + dy).max(48.0), expected_revision,
                })
            }
        }; self.submit(command, cx);
    }
}

pub fn open_in_workspace(workspace: &mut Workspace, source: MdeUiSource, window: &mut Window, cx: &mut Context<Workspace>) -> Entity<MdeWorkbenchItem> {
    let item = cx.new(|cx| MdeWorkbenchItem::new(source, cx));
    workspace.add_item_to_active_pane(Box::new(item.clone()), None, true, window, cx); item
}
impl EventEmitter<()> for MdeWorkbenchItem {}
impl Focusable for MdeWorkbenchItem { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus_handle.clone() } }
impl Item for MdeWorkbenchItem {
    type Event = ();
    fn tab_content_text(&self, _: usize, _: &App) -> SharedString {
        self.source.projects.snapshot(&self.source.project_id).map(|document| format!("MDE · {}", document.name()).into()).unwrap_or_else(|_| "MDE Workbench".into())
    }
    fn show_toolbar(&self) -> bool { false }
}

impl Render for MdeWorkbenchItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
        let background = palette.canvas;
        let border = palette.border;
        let selection = palette.selection;
        let muted = palette.muted;
        let zoom = self.zoom;
        let pan = self.pan;
        let relation_hint = match self.selected_tool {
            Some(ModelingTool::Relation(kind)) => Some(if self.pending_relation_source.is_some() {
                format!("{}: click second element (target)", kind.label())
            } else { format!("{}: {}", kind.label(), kind.endpoint_instruction()) }),
            _ => None,
        };
        let mut title = format!("MDE · {}", self.project_id().as_str());
        let details: String;
        let mut cards = Vec::new();
        let mut edges = Vec::new();
        match self.source.snapshot() {
            Ok(snapshot) => {
                let diagram = self.active_diagram.as_deref().and_then(|id| snapshot.section.diagrams.iter().find(|diagram| diagram.id == id)).or_else(|| snapshot.section.diagrams.first());
                details = format!("{} elements · {} relations · rev {}", snapshot.section.elements.len(), snapshot.section.relations.len(), snapshot.revision);
                if let Some(diagram) = diagram {
                    title = format!("{} / {}", snapshot.name, diagram.name);
                    let auto_routes = RelationshipGraph::new(&snapshot.section, &diagram.id).paths();
                    let mut views = diagram.views.iter().collect::<Vec<_>>();
                    views.sort_by_key(|view| view.z_index);
                    for view in views {
                        let Some(subject) = view.subject.as_deref() else { continue };
                        if let Some(relation) = snapshot.section.relations.iter().find(|relation| relation.id == subject) {
                            let geometry = auto_routes.get(subject).unwrap_or(&view.geometry);
                            if let Geometry::Path { start, segments } = geometry {
                                let mut points = vec![*start];
                                for segment in segments {
                                    match segment { PathSegment::Line { to } => points.push(*to), PathSegment::Cubic { control1, control2, to } => points.extend([*control1, *control2, *to]) }
                                }
                                if points.len() < 2 || points.iter().any(|point| !point.x.is_finite() || !point.y.is_finite()) { continue; }
                                let Some(end) = points.last().copied() else { continue };
                                let min_x = points.iter().map(|point| point.x).fold(f64::INFINITY, f64::min) as f32;
                                let min_y = points.iter().map(|point| point.y).fold(f64::INFINITY, f64::min) as f32;
                                let max_x = points.iter().map(|point| point.x).fold(f64::NEG_INFINITY, f64::max) as f32;
                                let max_y = points.iter().map(|point| point.y).fold(f64::NEG_INFINITY, f64::max) as f32;
                                let padding = 24.0_f32;
                                let width = (max_x - min_x + 2.0 * padding).max(48.0) * zoom;
                                let height = (max_y - min_y + 2.0 * padding).max(48.0) * zoom;
                                let place = |point: bellatrix_mde::Point| ((point.x as f32 - min_x + padding) * zoom, (point.y as f32 - min_y + padding) * zoom);
                                let presentation = relationship_presentation(relation);
                                let local_segments = segments.iter().map(|segment| match segment {
                                    PathSegment::Line { to } => UmlLineSegment::Line { to: place(*to) },
                                    PathSegment::Cubic { control1, control2, to } => UmlLineSegment::Cubic {
                                        control1: place(*control1), control2: place(*control2), to: place(*to),
                                    },
                                }).collect();
                                let glyph = UmlRelationGlyph::new(presentation.style, width, height, place(*start), place(end))
                                    .segments(local_segments).selected(self.selected_element.as_deref() == Some(subject));
                                let edge_owner = cx.weak_entity(); let edge_id = relation.id.clone();
                                edges.push(div().id(format!("mde-edge-{}", view.id)).absolute()
                                    .left(px((min_x - padding) * zoom + pan.0)).top(px((min_y - padding) * zoom + pan.1)).w(px(width)).h(px(height)).cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                        let _ = edge_owner.update(cx, |item, cx| { item.select_element(edge_id.clone(), cx); cx.focus_self(window); });
                                        cx.stop_propagation();
                                    }).child(glyph));
                            }
                            continue;
                        }
                        let Some(element) = snapshot.section.elements.iter().find(|element| element.id == subject) else { continue };
                        let Geometry::Rect { x, y, width, height } = &view.geometry else { continue };
                        let (x, y, width, height) = (*x, *y, *width, *height);
                        let revision = snapshot.revision;
                        if ![x, y, width, height].iter().all(|value| value.is_finite()) { continue; }
                        let mut local_x = x as f32; let mut local_y = y as f32;
                        let mut local_width = width as f32; let mut local_height = height as f32;
                        match &self.gesture {
                            Some(CanvasGesture::Move { view_id, dx, dy, .. }) if view_id == &view.id => { local_x += *dx as f32; local_y += *dy as f32; }
                            Some(CanvasGesture::Resize { view_id, dx, dy, .. }) if view_id == &view.id => { local_width = (width + dx).max(80.0) as f32; local_height = (height + dy).max(48.0) as f32; }
                            _ => {}
                        }
                        let semantic_type = UmlType::from_kind(&element.kind);
                        let is_interface = semantic_type == Some(UmlType::Interface);
                        let contents = classifier_contents(&snapshot.section, &element.id);
                        if is_interface { local_height = local_height.max(contents.minimum_interface_height() as f32); }
                        let quick_top = if is_interface { interface_layout(f64::from(local_width), f64::from(local_height), contents).label_top as f32 * zoom } else { 0.0 };
                        let selected_target = self.selected_element.as_deref().and_then(|id| {
                            if id == subject { Some(element) }
                            else if snapshot.section.relations.iter().any(|relation| relation.kind == "owns" && relation.from == element.id && relation.to == id) {
                                snapshot.section.elements.iter().find(|element| element.id == id)
                            } else { None }
                        });
                        let is_selected = selected_target.is_some();
                        let editing = selected_target.filter(|selected| self.editing_subject.as_deref() == Some(selected.id.as_str()) && self.inline_editor.is_some());
                        let (quick_actions, left_actions, visibility_picker) = if let Some(selected) = editing { quick_edit_for_element(selected, cx.entity()) } else { (None, None, None) };
                        let title_id = element.id.clone(); let title_owner = cx.weak_entity();
                        let select_owner = cx.weak_entity(); let edit_owner = cx.weak_entity();
                        let mut classifier = UmlClassifier::new(element.name.clone()).model_kind(&element.kind)
                            .view_size(f64::from(local_width), f64::from(local_height), zoom)
                            .class_id(element.id.clone()).selected(is_selected)
                            .on_title_edit(move |window, cx| {
                                let id = title_id.clone(); let _ = title_owner.update(cx, |item, cx| item.begin_inline_edit(id, window, cx));
                            })
                            .on_member_click(move |member_id, window, cx| {
                                let id = member_id.to_owned(); let _ = select_owner.update(cx, |item, cx| { item.select_element(id, cx); cx.focus_self(window); });
                            })
                            .on_member_edit(move |member_id, window, cx| {
                                let id = member_id.to_owned(); let _ = edit_owner.update(cx, |item, cx| item.begin_inline_edit(id, window, cx));
                            });
                        if let Some(value) = element.properties.get("stereotype").and_then(serde_json::Value::as_str).filter(|value| !value.trim().is_empty()) {
                            classifier = classifier.stereotype(format!("«{value}»"));
                        }
                        for relation in &snapshot.section.relations {
                            if relation.kind != "owns" || relation.from != element.id { continue; }
                            let Some(member) = snapshot.section.elements.iter().find(|candidate| candidate.id == relation.to) else { continue };
                            let visibility = member.properties.get("visibility").and_then(serde_json::Value::as_str).unwrap_or("public");
                            let prefix = match visibility { "private" => "-", "protected" => "#", "package" => "~", _ => "+" };
                            let suffix = if matches!(member.kind.as_str(), "rust.function" | "uml.operation" | "uml.reception") && !member.name.ends_with(')') { "()" } else { "" };
                            let text = format!("{prefix}{}{suffix}", member.name);
                            match member.kind.as_str() {
                                "rust.field" | "uml.attribute" => classifier = classifier.attribute_with_id(member.id.clone(), text),
                                "rust.function" | "uml.operation" => classifier = classifier.operation_with_id(member.id.clone(), text),
                                "uml.reception" => classifier = classifier.reception_with_id(member.id.clone(), text),
                                "uml.template_parameter" => classifier = classifier.template_parameter_with_id(member.id.clone(), format!("T: {}", member.name)),
                                _ => {}
                            }
                        }
                        if editing.is_some() {
                            if let (Some(id), Some(field)) = (self.editing_subject.clone(), self.inline_editor.clone()) {
                                let accept = cx.weak_entity(); let cancel = cx.weak_entity();
                                classifier = classifier.inline_edit(id, field,
                                    move |window, cx| { let _ = accept.update(cx, |item, cx| item.commit_inline_edit(window, cx)); },
                                    move |window, cx| { let _ = cancel.update(cx, |item, cx| item.cancel_inline_edit(window, cx)); });
                            }
                        }
                        let figure: gpui::AnyElement = if UmlFigure::supports(&element.kind) {
                            let edit_owner = cx.weak_entity(); let edit_id = element.id.clone();
                            let label = if element.kind == "uml.constraint" { format!("{{{}}}", element.name) } else { element.name.clone() };
                            let mut figure = UmlFigure::new(element.kind.clone(), label).selected(is_selected).view_scale(zoom)
                                .on_title_edit(move |window, cx| { let id = edit_id.clone(); let _ = edit_owner.update(cx, |item, cx| item.begin_inline_edit(id, window, cx)); });
                            if editing.is_some() {
                                if let Some(field) = self.inline_editor.clone() {
                                    let confirm = cx.weak_entity(); let cancel = cx.weak_entity();
                                    figure = figure.inline_edit(field,
                                        move |window, cx| { let _ = confirm.update(cx, |item, cx| item.commit_inline_edit(window, cx)); },
                                        move |window, cx| { let _ = cancel.update(cx, |item, cx| item.cancel_inline_edit(window, cx)); });
                                }
                            }
                            figure.into_any_element()
                        } else if semantic_type.is_some_and(UmlType::is_classifier) { classifier.into_any_element() }
                        else { div().size_full().border_1().border_color(border).child(Label::new(format!("Unsupported notation: {}", element.kind))).into_any_element() };
                        let drag_owner = cx.weak_entity(); let drag_id = view.id.clone();
                        let connection_mode = matches!(self.selected_tool, Some(ModelingTool::Relation(_)));
                        let connect_owner = cx.weak_entity();
                        let connect_target = subject.to_owned();
                        let resize_owner = cx.weak_entity(); let resize_id = view.id.clone(); let element_id = subject.to_owned();
                        cards.push(div().id(format!("mde-view-{}", view.id)).absolute()
                            .left(px(local_x * zoom + pan.0)).top(px(local_y * zoom + pan.1)).w(px(local_width * zoom)).h(px(local_height * zoom))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, window, cx| {
                                let _ = drag_owner.update(cx, |item, cx| { item.start_move(drag_id.clone(), element_id.clone(), revision, event.position, cx); cx.focus_self(window); });
                                cx.stop_propagation();
                            }).child(figure)
                            .when_some(quick_actions, |card, actions| card.child(div().id(format!("mde-quick-actions-{}", view.id)).absolute()
                                .left(px(local_width * zoom + 6.0)).top(px(quick_top)).child(actions)))
                            .when_some(left_actions, |card, actions| card.child(div().id(format!("mde-quick-left-actions-{}", view.id)).absolute()
                                .left(px(-78.0)).top(px(quick_top + 28.0)).child(actions)))
                            .when_some(visibility_picker, |card, picker| card.child(div().id(format!("mde-quick-visibility-{}", view.id)).absolute()
                                .left(px(-34.0)).top(px(quick_top)).child(picker)))
                            .when(is_selected && !connection_mode && semantic_type != Some(UmlType::Port), |card| card.child(div().id(format!("mde-resize-{}", view.id)).absolute().bottom_0().right_0()
                                .w(px(10.0)).h(px(10.0)).bg(selection)
                                .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                                    let _ = resize_owner.update(cx, |item, cx| item.start_resize(resize_id.clone(), width, height, revision, event.position, cx));
                                    cx.stop_propagation();
                                })))
                            .when(connection_mode, |card| card.child(div()
                                .id(format!("mde-connection-target-{}", view.id)).absolute().top_0().right_0().bottom_0().left_0().cursor_crosshair()
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    let _ = connect_owner.update(cx, |item, cx| {
                                        if let Some(ModelingTool::Relation(kind)) = item.selected_tool { item.link_endpoint(kind, connect_target.clone(), cx); cx.focus_self(window); }
                                    });
                                    cx.stop_propagation();
                                }))));
                    }
                }
            }
            Err(error) => details = error,
        }
        let owner = cx.weak_entity(); let move_owner = cx.weak_entity(); let up_owner = cx.weak_entity(); let out_owner = cx.weak_entity();
        let dragging_owner = cx.weak_entity(); let dropping_owner = cx.weak_entity(); let wheel_owner = cx.weak_entity();
        let pan_start_owner = cx.weak_entity(); let pan_end_owner = cx.weak_entity(); let pan_out_owner = cx.weak_entity();
        let delete_owner = cx.weak_entity(); let f2_owner = cx.weak_entity(); let member_shortcut_owner = cx.weak_entity();
        let zoom_in_owner = cx.weak_entity(); let zoom_out_owner = cx.weak_entity(); let reset_owner = cx.weak_entity();
        let cancel_relation_owner = cx.weak_entity(); let escape_relation_owner = cx.weak_entity();
        div().id(format!("mde-workbench-{}", self.project_id().as_str())).size_full().flex().flex_col().track_focus(&self.focus_handle)
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("f2") {
                    let _ = f2_owner.update(cx, |item, cx| { if let Some(id) = item.selected_element.clone() { item.begin_inline_edit(id, window, cx); } });
                    cx.stop_propagation();
                } else if event.keystroke.key.eq_ignore_ascii_case("enter") && event.keystroke.modifiers.control {
                    let kind = if event.keystroke.modifiers.shift { ModelingMemberKind::Operation } else { ModelingMemberKind::Attribute };
                    let _ = member_shortcut_owner.update(cx, |item, cx| item.add_member_to_selected(kind, cx)); cx.stop_propagation();
                } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    let _ = escape_relation_owner.update(cx, |item, cx| {
                        if item.editing_subject.is_none() && item.selected_tool.is_some() { item.cancel_tool(cx); }
                    });
                } else if matches!(event.keystroke.key.as_str(), "delete" | "backspace") {
                    let _ = delete_owner.update(cx, |item, cx| { if item.editing_subject.is_none() { item.delete_selected(cx); } });
                    cx.stop_propagation();
                }
            }).bg(background).text_color(palette.text)
            .child(div().h(px(39.0)).flex().items_center().justify_between().px_3().border_b_1().border_color(border)
                .child(Label::new(title)).child(div().flex().gap_2().items_center().child(Label::new(details))
                    .when_some(relation_hint, |header, hint| header.child(div().flex().gap_1().items_center()
                        .child(Label::new(hint)).child(ui::Button::new("mde-cancel-relation", "Cancel")
                            .on_click(move |_, _, cx| { let _ = cancel_relation_owner.update(cx, |item, cx| item.cancel_tool(cx)); }))))
                    .child(ui::Button::new("mde-zoom-out", "-").on_click(move |_, _, cx| { let _ = zoom_out_owner.update(cx, |item, cx| item.set_zoom(item.zoom / 1.2, cx)); }))
                    .child(Label::new(format!("{:.0}%", zoom * 100.0)))
                    .child(ui::Button::new("mde-zoom-in", "+").on_click(move |_, _, cx| { let _ = zoom_in_owner.update(cx, |item, cx| item.set_zoom(item.zoom * 1.2, cx)); }))
                    .child(ui::Button::new("mde-reset-view", "100%").on_click(move |_, _, cx| { let _ = reset_owner.update(cx, |item, cx| item.reset_viewport(cx)); }))))
            .child(div().id("mde-canvas").relative().flex_1().overflow_hidden()
                .on_scroll(move |event: &ScrollEvent, _, cx| { let _ = wheel_owner.update(cx, |item, cx| item.scroll_viewport(event, cx)); cx.stop_propagation(); })
                .on_mouse_down(MouseButton::Middle, move |event: &MouseDownEvent, _, cx| { let _ = pan_start_owner.update(cx, |item, _| item.start_pan(event.position)); cx.stop_propagation(); })
                .on_mouse_up(MouseButton::Middle, move |_: &MouseUpEvent, _, cx| { let _ = pan_end_owner.update(cx, |item, _| item.pan_origin = None); })
                .on_mouse_up_out(MouseButton::Middle, move |_: &MouseUpEvent, _, cx| { let _ = pan_out_owner.update(cx, |item, _| item.pan_origin = None); })
                .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _, cx| { let _ = owner.update(cx, |item, cx| { if let Some(ModelingTool::Element(kind)) = item.selected_tool { item.create_in_next_slot(kind, cx); } }); })
                .on_mouse_move(move |event: &MouseMoveEvent, _, cx| { let _ = move_owner.update(cx, |item, cx| { item.update_pan(event, cx); item.update_gesture(event, cx); }); })
                .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, _, cx| { let _ = up_owner.update(cx, |item, cx| item.finish_gesture(cx)); })
                .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, _, cx| { let _ = out_owner.update(cx, |item, cx| item.finish_gesture(cx)); })
                .on_drag_move(move |event: &DragMoveEvent<MdeToolDrag>, _, cx| {
                    let delta_x = f64::from(event.event.position.x - event.bounds.origin.x);
                    let delta_y = f64::from(event.event.position.y - event.bounds.origin.y);
                    let _ = dragging_owner.update(cx, |item, _| { item.drop_point = Some(((delta_x - f64::from(item.pan.0)) / f64::from(item.zoom), (delta_y - f64::from(item.pan.1)) / f64::from(item.zoom))); });
                })
                .on_drop(move |drag: &MdeToolDrag, _, cx| {
                    let _ = dropping_owner.update(cx, |item, cx| {
                        if &drag.project_id != item.project_id() { return; }
                        let (x, y) = item.drop_point.take().unwrap_or((40.0, 40.0));
                        match drag.kind { ModelingTool::Element(kind) => item.create_element(kind, x, y, cx), ModelingTool::Relation(kind) => item.arm_drag_tool(ModelingTool::Relation(kind), cx) }
                    });
                }).children(edges).children(cards))
            .child(div().h(px(25.0)).px_2().flex().items_center().border_t_1().border_color(border)
                .text_color(muted).child(Label::new(self.feedback.clone())))
    }
}

#[cfg(test)]
mod relationship_interaction_tests {
    use super::*;
    #[test]
    fn two_clicks_connect_distinct_model_elements() {
        let mut pending = None;
        assert_eq!(choose_relation_endpoint(&mut pending, "type-list"), RelationClick::First);
        assert_eq!(pending.as_deref(), Some("type-list"));
        assert_eq!(choose_relation_endpoint(&mut pending, "type-task"), RelationClick::Connect("type-list".to_owned()));
        assert_eq!(pending, None);
    }
    #[test]
    fn clicking_source_again_does_not_drop_armed_tool() {
        let mut pending = None;
        assert_eq!(choose_relation_endpoint(&mut pending, "type-list"), RelationClick::First);
        assert_eq!(choose_relation_endpoint(&mut pending, "type-list"), RelationClick::Repeat);
        assert_eq!(pending.as_deref(), Some("type-list"));
        assert_eq!(choose_relation_endpoint(&mut pending, "type-task"), RelationClick::Connect("type-list".into()));
    }
}
