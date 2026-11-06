//! Engine-MDE as a native Zed Workspace Item.
//! Zed owns presentation; bellatrix_mde owns semantics; Cordis publishes revisions.
#[path = "../view-model/workbench_state.rs"]
mod workbench_state;
use workbench_state::{PendingRelationEndpoint, SelectionChange, WorkbenchState};

#[path = "../view-model/canvas_presentation.rs"]
mod canvas_presentation;
#[path = "../layouts/body.rs"]
mod diagram_canvas;
#[path = "../components/execution_dashboard.rs"]
mod execution_dashboard;
#[path = "../view-model/explorer.rs"]
mod explorer;
#[path = "../components/model_explorer.rs"]
mod hierarchy;
#[path = "../components/inline_editor.rs"]
mod inline_editor;
#[path = "../view-model/inspector_drafts.rs"]
mod inspector_drafts;
#[path = "../routes/workspace.rs"]
mod integration;
#[path = "../api/model_editing.rs"]
mod model_editing;
#[path = "../api/native_document.rs"]
mod native_document;
#[path = "../view-model/native_document_guard.rs"]
mod native_document_guard;
#[path = "../layouts/sidebar.rs"]
pub mod panels;
#[path = "../components/inspector.rs"]
mod property_editor;
#[path = "../components/quick_tools.rs"]
mod quick_edit_view;
#[path = "../api/relationship_creation.rs"]
mod relationship_creation;
#[path = "../view-model/selection.rs"]
mod selection;
#[path = "../view-model/snapshot.rs"]
mod snapshot;
#[path = "../api/source.rs"]
mod source;
#[path = "../layouts/footer.rs"]
mod status_entry;
#[path = "../view-model/style_drafts.rs"]
mod style_drafts;
#[path = "../components/style_editor.rs"]
mod style_editor;
#[path = "../components/symbol_library.rs"]
mod symbol_library;
#[path = "../assets/theme.rs"]
mod theme;
#[path = "../components/timing_editor.rs"]
mod timing_editor;
#[path = "../api/validation.rs"]
mod validation;
#[path = "../components/visual_draft_canvas.rs"]
mod visual_draft_canvas;
pub use bellatrix_mde_cordis::MODELING_NAMESPACE;
pub use integration::register_mde_workspace;
pub use native_document::{NativeDocumentImporter, NativeDocumentLease};
pub use snapshot::MdeSnapshot;
pub use source::MdeUiSource;
pub use symbol_library::MdeSymbolLibraryItem;
pub use theme::WorkbenchPalette;

use crate::elements::uml::{UmlClassifier, UmlFigure, UmlLineSegment, UmlRelationGlyph};
use bellatrix_mde_cordis::{Geometry, ModelingRelationKind, PathSegment};
use bellatrix_mde_cordis::{ModelingElementKind, ModelingMemberKind, ModelingTool};
use bellatrix_project::ProjectId;
use gpui::{
    App, AppContext, Context, DragMoveEvent, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Render, ScrollEvent, SharedString, Task, Window, div, prelude::*, px,
};
use quick_edit_view::{ParameterAction, QuickEditTarget, quick_edit_for_element};
use ui::{Clickable, Label, LabelCommon};
use ui_input::InputField;
use workspace::{Workspace, item::Item};

#[derive(Clone)]
pub struct MdeToolDrag {
    pub project_id: ProjectId,
    pub kind: ModelingTool,
    pub label: String,
}
impl Render for MdeToolDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = WorkbenchPalette::current(cx);
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
            .child(Label::new(self.label.clone()))
    }
}
#[derive(Clone, Copy)]
enum PathHandle {
    Vertex(usize),
    Control1(usize),
    Control2(usize),
    Segment(usize),
}

#[derive(Clone)]
enum CanvasGesture {
    Move {
        view_id: String,
        start: Point<Pixels>,
        dx: f64,
        dy: f64,
        revision: u64,
        began: bool,
        submitted: bool,
    },
    Resize {
        view_id: String,
        start: Point<Pixels>,
        width: f64,
        height: f64,
        dx: f64,
        dy: f64,
        revision: u64,
        began: bool,
        submitted: bool,
    },
    EditPath {
        diagram_id: String,
        view_id: String,
        geometry: Geometry,
        handle: PathHandle,
        start: Point<Pixels>,
        dx: f64,
        dy: f64,
        revision: u64,
        began: bool,
        submitted: bool,
    },
}
impl CanvasGesture {
    fn submission(&self) -> (u64, bool) {
        match self {
            Self::Move {
                revision,
                submitted,
                ..
            }
            | Self::Resize {
                revision,
                submitted,
                ..
            }
            | Self::EditPath {
                revision,
                submitted,
                ..
            } => (*revision, *submitted),
        }
    }

    fn mark_submitted(&mut self) {
        match self {
            Self::Move { submitted, .. }
            | Self::Resize { submitted, .. }
            | Self::EditPath { submitted, .. } => *submitted = true,
        }
    }
}

pub struct MdeWorkbenchItem {
    source: MdeUiSource,
    presentation: WorkbenchState,
    focus_handle: FocusHandle,
    inline_editor: Option<Entity<InputField>>,
    inline_draft: Option<inspector_drafts::DraftState>,
    gesture: Option<CanvasGesture>,
    drop_point: Option<(f64, f64)>,
    zoom: f32,
    pan: (f32, f32),
    pan_origin: Option<(Point<Pixels>, (f32, f32))>,
    feedback: String,
    validation_mode: validation::ValidationMode,
    validation_status: String,
    _watch_task: Task<()>,
}
pub(crate) fn diagram_kind_label(
    kind: bellatrix_mde_cordis::StructuralDiagramKind,
) -> &'static str {
    use bellatrix_mde_cordis::StructuralDiagramKind::*;
    match kind {
        Class => "Class",
        Object => "Object",
        Package => "Package",
        Component => "Component",
        CompositeStructure => "Composite structure",
        Deployment => "Deployment",
        Profile => "Profile",
        UseCase => "Use case",
        Activity => "Activity",
        StateMachine => "State machine",
        Sequence => "Sequence",
        Communication => "Communication",
        InteractionOverview => "Interaction overview",
        Timing => "Timing",
    }
}

fn selected_view_target(diagram: Option<&str>, view: Option<&str>) -> Option<(String, String)> {
    Some((diagram?.to_owned(), view?.to_owned()))
}

fn canvas_delete_action_allowed(canvas_focused: bool, editing: bool) -> bool {
    canvas_focused && !editing
}

fn next_diagram_grid_position(view_count: usize) -> (f64, f64) {
    (
        24.0 + (view_count % 4) as f64 * 200.0,
        36.0 + (view_count / 4) as f64 * 148.0,
    )
}

#[derive(Debug, PartialEq)]
enum ElementViewPlacement {
    Existing(String),
    NewAt(f64, f64),
}

fn element_view_placement(
    diagram: &bellatrix_mde_cordis::projection::Diagram,
    element_id: &str,
) -> ElementViewPlacement {
    if let Some(view) = diagram
        .all_views()
        .find(|view| view.subject.as_deref() == Some(element_id))
    {
        ElementViewPlacement::Existing(view.id.clone())
    } else {
        let (x, y) = next_diagram_grid_position(diagram.views.len());
        ElementViewPlacement::NewAt(x, y)
    }
}

fn canvas_model_position(
    position: (f64, f64),
    origin: (f64, f64),
    pan: (f32, f32),
    zoom: f32,
) -> (f64, f64) {
    (
        (position.0 - origin.0 - f64::from(pan.0)) / f64::from(zoom),
        (position.1 - origin.1 - f64::from(pan.1)) / f64::from(zoom),
    )
}

impl MdeWorkbenchItem {
    pub(crate) fn tool_label(&self, tool: ModelingTool) -> String {
        self.source
            .presentation_snapshot()
            .ok()
            .and_then(|snapshot| snapshot.section.tools.label(tool).map(str::to_owned))
            .unwrap_or_else(|| "Tool".into())
    }

    pub fn new(source: MdeUiSource, cx: &mut Context<Self>) -> Self {
        let validation_mode = validation::ValidationMode::load(source.project_id.as_str());
        let initial_validation_feedback = validation_mode
            .as_ref()
            .err()
            .map(|error| format!("Could not load validation preference: {error}"))
            .unwrap_or_default();
        let initial_diagram = source.snapshot().ok().and_then(|snapshot| {
            snapshot
                .section
                .diagrams
                .first()
                .map(|diagram| diagram.id.clone())
        });
        let mut revisions = source.revisions.clone();
        let mut projections = source.projections.clone();
        let mut move_previews = source.move_previews.as_ref().map(|client| client.changes());
        let mut feedback = source.feedback.clone();
        let mut validations = source.validations.clone();
        let mut saves = source.saves.clone();
        let mut activity = source.activity();
        let task = cx.spawn(async move |this, cx| {
            let mut projections_open = true;
            let mut activity_open = true;
            let mut revisions_open = true;
            let mut saves_open = true;
            let mut validations_open = true;
            let mut feedback_open = true;
            loop {
                tokio::select! {
                    changed = activity.changed(), if activity_open => {
                        if changed.is_err() { activity_open = false; continue; }
                        if this.update(cx, |_, cx| cx.notify()).is_err() { break; }
                    }
                    changed = async {
                        match &mut move_previews {
                            Some(changes) => changes.changed().await,
                            None => std::future::pending().await,
                        }
                    } => {
                        if changed.is_err() { move_previews = None; }
                        if this.update(cx, |_, cx| cx.notify()).is_err() { break; }
                    }
                    changed = projections.changed(), if projections_open => {
                        if changed.is_err() { projections_open = false; continue; }
                        if this.update(cx, |item, cx| { item.refresh(cx); item.validate_after_change(cx); }).is_err() { break; }
                    }
                    changed = revisions.changed(), if revisions_open => {
                        if changed.is_err() { revisions_open = false; continue; }
                        if this.update(cx, |item, cx| { item.refresh(cx); item.validate_after_change(cx); }).is_err() { break; }
                    }
                    changed = saves.changed(), if saves_open => {
                        if changed.is_err() { saves_open = false; continue; }
                        if this.update(cx, |item, cx| item.validate_after_save(cx)).is_err() { break; }
                    }
                    changed = validations.changed(), if validations_open => {
                        if changed.is_err() { validations_open = false; continue; }
                        let result = validations.borrow().clone();
                        if this.update(cx, |item, cx| item.accept_validation(result, cx)).is_err() { break; }
                    }
                    changed = feedback.changed(), if feedback_open => {
                        if changed.is_err() { feedback_open = false; continue; }
                        let text = feedback.borrow().clone();
                        if this.update(cx, |item, cx| {
                            item.feedback = text; cx.notify();
                        }).is_err() { break; }
                    }
                }
            }
        });
        Self {
            source,
            presentation: WorkbenchState::new(initial_diagram),
            focus_handle: cx.focus_handle(),
            inline_editor: None,
            inline_draft: None,
            gesture: None,
            drop_point: None,
            zoom: 1.0,
            pan: (0.0, 0.0),
            pan_origin: None,
            feedback: initial_validation_feedback,
            validation_mode: validation_mode.unwrap_or(validation::ValidationMode::Manual),
            validation_status: "Not yet validated".into(),
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
        self.presentation.selected_element()
    }
    pub fn selected_tool(&self) -> Option<ModelingTool> {
        self.presentation.selected_tool()
    }
    pub fn active_diagram(&self) -> Option<&str> {
        self.presentation.active_diagram()
    }
    pub fn select_element(&mut self, id: String, cx: &mut Context<Self>) {
        self.source.record_interaction(
            "selection.element",
            Some(&id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        let change = self.presentation.select_element(id);
        self.apply_selection_change(change, cx);
    }

    pub fn select_explorer_element(
        &mut self,
        id: String,
        key: String,
        occurrence: Option<bellatrix_mde_cordis::ViewOccurrence>,
        cx: &mut Context<Self>,
    ) {
        self.source.record_interaction(
            "selection.explorer_member",
            Some(&key),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        let change = self
            .presentation
            .select_explorer_element(id, key, occurrence);
        self.apply_selection_change(change, cx);
    }

    fn apply_selection_change(&mut self, change: SelectionChange, cx: &mut Context<Self>) {
        if !change.preserve_editor {
            self.inline_editor = None;
            self.inline_draft = None;
        }
        if change.selection_changed {
            self.gesture = None;
            cx.notify();
        }
    }

    fn select_view(&mut self, subject: String, view_id: String, cx: &mut Context<Self>) {
        self.source.record_interaction(
            "selection.view",
            Some(&view_id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        let change = self.presentation.select_view(Some(subject), view_id);
        self.apply_selection_change(change, cx);
        cx.notify();
    }

    pub fn select_diagram_view(
        &mut self,
        diagram_id: String,
        view_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_presentation_suspended() {
            self.source.record_interaction(
                "selection.view.suspended",
                Some(&view_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return;
        }
        let selection = self.source.snapshot().and_then(|snapshot| {
            snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .and_then(|diagram| diagram.all_views().find(|view| view.id == view_id))
                .map(|view| view.subject.clone())
                .ok_or_else(|| "The original diagram View no longer exists".to_owned())
        });
        let subject = match selection {
            Ok(subject) => subject,
            Err(error) => {
                self.source.record_interaction(
                    "selection.view.missing",
                    Some(&view_id),
                    bellatrix_mde_cordis::TraceOutcome::Invalid,
                );
                self.feedback = error;
                cx.notify();
                return;
            }
        };
        self.source.record_interaction(
            "selection.view",
            Some(&view_id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        self.select_diagram(diagram_id, cx);
        let change = self.presentation.select_view(subject, view_id);
        self.apply_selection_change(change, cx);
        self.feedback = if self.presentation.selected_element().is_none() {
            "Visual View selected; no semantic subject is required".into()
        } else {
            "Diagram View selected".into()
        };
        cx.notify();
    }

    fn selected_view_is_inspection_only(&self) -> bool {
        let Some(view_id) = self.presentation.selected_view() else {
            return false;
        };
        self.source.snapshot().ok().is_none_or(|snapshot| {
            snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| Some(diagram.id.as_str()) == self.presentation.active_diagram())
                .is_none_or(|diagram| !selection::supports_view_editing(diagram, view_id))
        })
    }

    pub fn remove_selected_view(&mut self, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Some((diagram_id, view_id)) = selected_view_target(
            self.presentation.active_diagram(),
            self.presentation.selected_view(),
        ) else {
            self.feedback = "Select a view in the active diagram to remove".into();
            cx.notify();
            return;
        };
        self.remove_view(diagram_id, view_id, cx);
    }

    fn remove_view(&mut self, diagram_id: String, view_id: String, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        if !snapshot.section.diagrams.iter().any(|diagram| {
            diagram.id == diagram_id && selection::supports_view_editing(diagram, &view_id)
        }) {
            self.feedback = "This view is unavailable or supports inspection only".into();
            cx.notify();
            return;
        }
        let result = self
            .source
            .remove_view(&diagram_id, &view_id, snapshot.revision);
        self.report_command_result(result, cx);
    }

    fn view_context_menu(
        owner: Entity<Self>,
        diagram_id: String,
        view_id: String,
        subject: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<ui::ContextMenu> {
        owner.update(cx, |item, cx| {
            item.select_view(subject.clone(), view_id.clone(), cx)
        });
        if owner.read(cx).source.is_read_only() {
            return ui::ContextMenu::build(window, cx, |menu, _, _| {
                menu.header("Read-only MDE preview")
            });
        }
        ui::ContextMenu::build(window, cx, move |menu, _, _| {
            let remove_owner = owner.clone();
            menu.entry(
                "Remove from diagram",
                Some(Box::new(integration::RemoveFromDiagram)),
                move |_, cx| {
                    remove_owner.update(cx, |item, cx| {
                        item.remove_view(diagram_id.clone(), view_id.clone(), cx)
                    });
                },
            )
            .entry(
                "Delete from model",
                Some(Box::new(integration::DeleteFromModel)),
                move |_, cx| {
                    owner.update(cx, |item, cx| {
                        item.select_element(subject.clone(), cx);
                        item.delete_selected_from_model(cx);
                    });
                },
            )
        })
    }

    pub fn feedback(&self) -> &str {
        &self.feedback
    }
    pub fn validation_status(&self) -> &str {
        &self.validation_status
    }

    pub fn create_diagram(
        &mut self,
        kind: bellatrix_mde_cordis::StructuralDiagramKind,
        cx: &mut Context<Self>,
    ) {
        if !self.source.allows_creation() {
            self.feedback = "Diagram creation is not available for this document".into();
            cx.notify();
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let result = self.source.create_diagram(
            kind,
            self.presentation.selected_element(),
            snapshot.revision,
        );
        self.report_command_result(result, cx);
    }

    pub fn select_diagram(&mut self, id: String, cx: &mut Context<Self>) {
        self.source.record_interaction(
            "selection.diagram",
            Some(&id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        if self.presentation.select_diagram(id) {
            self.validation_status = "Not yet validated for this diagram".into();
        }
        self.gesture = None;
        self.inline_editor = None;
        self.inline_draft = None;
        cx.notify();
    }
    pub fn select_tool(&mut self, kind: ModelingTool, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            self.source.record_interaction(
                "tool.select.read_only",
                Some(&self.tool_label(kind)),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return;
        }
        if self.presentation.selected_tool() == Some(kind) {
            self.cancel_tool(cx);
        } else {
            self.arm_drag_tool(kind, cx);
        }
    }
    pub fn cancel_tool(&mut self, cx: &mut Context<Self>) {
        self.source.record_interaction(
            "tool.cancel",
            None,
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        self.presentation.cancel_tool();
        self.feedback = "Selection tool active".into();
        cx.notify();
    }
    pub(super) fn allows_modeling_tool(
        &mut self,
        tool: ModelingTool,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.source.allows_creation() {
            self.presentation.cancel_tool();
            self.feedback = "Creation tools are not available for this document".into();
            cx.notify();
            return false;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return false;
        };
        if !snapshot
            .section
            .tools
            .allows_tool(self.presentation.active_diagram(), tool)
        {
            self.presentation.cancel_tool();
            self.feedback = "This tool is not available for the active diagram's domain.".into();
            cx.notify();
            return false;
        }
        true
    }
    pub fn arm_drag_tool(&mut self, tool: ModelingTool, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            self.source.record_interaction(
                "tool.arm.read_only",
                Some(&self.tool_label(tool)),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return;
        }
        if !self.allows_modeling_tool(tool, cx) {
            return;
        }
        self.source.record_interaction(
            "tool.arm",
            Some(&self.tool_label(tool)),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        self.gesture = None;
        self.inline_editor = None;
        self.inline_draft = None;
        self.presentation.arm_tool(tool);
        let catalog = self
            .source
            .snapshot()
            .map(|snapshot| snapshot.section.tools)
            .unwrap_or_default();
        let label = catalog.label(tool).unwrap_or("Tool");
        self.feedback = match tool {
            ModelingTool::Relation(kind) => catalog
                .relation(kind)
                .map(|metadata| format!("{}: {}", label, metadata.endpoint_instruction))
                .unwrap_or_else(|| "Tool metadata unavailable".into()),
            ModelingTool::Element(kind) => match catalog.element(kind) {
                Some(metadata) if metadata.requires_nested_owner => format!(
                    "{label}: click to place; unsupported semantic ownership is preserved as a visual draft"
                ),
                Some(metadata) if metadata.requires_owner => {
                    format!("{label}: click to place; select an owner to create a semantic element")
                }
                Some(_) => format!("{label}: click canvas to place"),
                None => "Tool metadata unavailable".into(),
            },
            ModelingTool::ProvidedInterface | ModelingTool::RequiredInterface => {
                format!("{label}: click canvas to place")
            }
        };
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
        let delta_x = f32::from(delta.x);
        let delta_y = f32::from(delta.y);
        if event.modifiers.control {
            self.set_zoom(
                self.zoom * if delta_y > 0.0 { 1.12 } else { 1.0 / 1.12 },
                cx,
            );
        } else {
            self.pan.0 += delta_x;
            self.pan.1 += delta_y;
            cx.notify();
        }
    }
    fn start_pan(&mut self, position: Point<Pixels>) {
        self.pan_origin = Some((position, self.pan));
    }
    fn update_pan(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Middle) {
            return;
        }
        if let Some((position, (x, y))) = &self.pan_origin {
            self.pan = (
                *x + f32::from(event.position.x - position.x),
                *y + f32::from(event.position.y - position.y),
            );
            cx.notify();
        }
    }
    fn start_move(
        &mut self,
        view_id: String,
        subject: String,
        revision: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.gesture_submission_pending() {
            return;
        }
        if let Some(ModelingTool::Relation(kind)) = self.presentation.selected_tool() {
            self.connect_relation_view(kind, view_id, None, cx);
            return;
        }
        self.select_view(subject, view_id.clone(), cx);
        if self.source.is_read_only() {
            return;
        }
        self.gesture = Some(CanvasGesture::Move {
            view_id,
            start: position,
            dx: 0.0,
            dy: 0.0,
            revision,
            began: false,
            submitted: false,
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
        if self.source.is_read_only() || self.gesture_submission_pending() {
            return;
        }
        self.gesture = Some(CanvasGesture::Resize {
            view_id,
            start: position,
            width,
            height,
            dx: 0.0,
            dy: 0.0,
            revision,
            began: false,
            submitted: false,
        });
        cx.notify();
    }
    fn update_gesture(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.source.is_read_only() || self.gesture_submission_pending() {
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) || !event.dragging() {
            return;
        }
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        let (start, dx, dy, began, view_id, action) = match gesture {
            CanvasGesture::Move {
                start,
                dx,
                dy,
                began,
                view_id,
                ..
            } => (start, dx, dy, began, view_id, "canvas.move.start"),
            CanvasGesture::Resize {
                start,
                dx,
                dy,
                began,
                view_id,
                ..
            } => (start, dx, dy, began, view_id, "canvas.resize.start"),
            CanvasGesture::EditPath {
                start,
                dx,
                dy,
                began,
                view_id,
                ..
            } => (start, dx, dy, began, view_id, "canvas.path_edit.start"),
        };
        *dx = f64::from(event.position.x - start.x) / f64::from(self.zoom);
        *dy = f64::from(event.position.y - start.y) / f64::from(self.zoom);
        if !*began && (dx.abs() >= 1.0 || dy.abs() >= 1.0) {
            *began = true;
            self.source.record_interaction(
                action,
                Some(view_id),
                bellatrix_mde_cordis::TraceOutcome::Success,
            );
        }
        cx.notify();
    }
    fn gesture_submission_pending(&self) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|gesture| gesture.submission().1)
    }

    fn reconcile_gesture(&mut self, presentation_revision: Option<u64>) {
        let Some(gesture) = &self.gesture else {
            return;
        };
        let (revision, submitted) = gesture.submission();
        // Completion releases the pending count after publishing the document.
        // Read it first so a completed command cannot pair with an older document.
        let pending = self.source.is_pending();
        let retain = self.source.document().is_ok_and(|document| {
            if submitted {
                // Keep only the existing transient preview until the canonical
                // projection replaces it, including the gap after command completion.
                presentation_revision == Some(revision)
                    && (document.revision() != revision || pending)
            } else {
                document.revision() == revision
            }
        });
        if !retain {
            self.gesture = None;
        }
    }

    fn finish_gesture(&mut self, cx: &mut Context<Self>) {
        if self.gesture_submission_pending() {
            return;
        }
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        let (view_id, began) = match &gesture {
            CanvasGesture::Move { view_id, began, .. }
            | CanvasGesture::Resize { view_id, began, .. }
            | CanvasGesture::EditPath { view_id, began, .. } => (view_id, *began),
        };
        if !began {
            cx.notify();
            return;
        }
        if self.source.is_read_only() {
            self.source.record_interaction(
                "canvas.gesture.read_only",
                Some(view_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            cx.notify();
            return;
        }
        let expected_revision = match &gesture {
            CanvasGesture::Move { revision, .. }
            | CanvasGesture::Resize { revision, .. }
            | CanvasGesture::EditPath { revision, .. } => *revision,
        };
        if self
            .source
            .snapshot()
            .ok()
            .map(|snapshot| snapshot.revision)
            != Some(expected_revision)
        {
            let view_id = match &gesture {
                CanvasGesture::Move { view_id, .. }
                | CanvasGesture::Resize { view_id, .. }
                | CanvasGesture::EditPath { view_id, .. } => view_id,
            };
            self.source.record_interaction(
                "canvas.gesture.stale_revision",
                Some(view_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            self.feedback = "Project changed during drag; retry".into();
            cx.notify();
            return;
        }
        let mut submitted_gesture = gesture.clone();
        let result = match gesture {
            CanvasGesture::Move {
                view_id, dx, dy, ..
            } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    self.source.record_interaction(
                        "canvas.gesture.no_change",
                        Some(&view_id),
                        bellatrix_mde_cordis::TraceOutcome::Success,
                    );
                    cx.notify();
                    return;
                }
                self.source.move_view(&view_id, dx, dy, expected_revision)
            }
            CanvasGesture::Resize {
                view_id,
                width,
                height,
                dx,
                dy,
                ..
            } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    self.source.record_interaction(
                        "canvas.gesture.no_change",
                        Some(&view_id),
                        bellatrix_mde_cordis::TraceOutcome::Success,
                    );
                    cx.notify();
                    return;
                }
                let minimum = self
                    .source
                    .snapshot()
                    .ok()
                    .and_then(|snapshot| {
                        snapshot
                            .section
                            .diagrams
                            .iter()
                            .flat_map(|diagram| diagram.all_views())
                            .find(|view| view.id == view_id)
                            .map(|view| canvas_presentation::minimum_size(&snapshot.section, view))
                    })
                    .unwrap_or((80.0, 48.0));
                self.source.resize_view(
                    &view_id,
                    (width + dx).max(minimum.0),
                    (height + dy).max(minimum.1),
                    expected_revision,
                )
            }
            CanvasGesture::EditPath {
                diagram_id,
                view_id,
                geometry,
                handle,
                dx,
                dy,
                ..
            } => {
                let edited = diagram_canvas::edited_path_geometry(&geometry, handle, dx, dy);
                if edited == geometry {
                    cx.notify();
                    return;
                }
                self.source
                    .set_view_path(&diagram_id, &view_id, edited, expected_revision)
            }
        };
        self.feedback = match result {
            Ok(()) => {
                submitted_gesture.mark_submitted();
                self.gesture = Some(submitted_gesture);
                "Command sent to Cordis".into()
            }
            Err(error) => format!("Command not sent: {error}"),
        };
        cx.notify();
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
        self.source
            .presentation_snapshot()
            .map(|snapshot| format!("MDE · {}", snapshot.name).into())
            .unwrap_or_else(|_| "MDE Workbench".into())
    }
    fn show_toolbar(&self) -> bool {
        false
    }
}
