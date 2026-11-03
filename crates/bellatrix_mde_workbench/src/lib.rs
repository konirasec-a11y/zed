//! Engine-MDE as a native Zed Workspace Item.
//! Zed owns presentation; bellatrix_mde owns semantics; Cordis publishes revisions.
mod diagram_canvas;
mod hierarchy;
mod inspector_drafts;
mod integration;
mod model_editing;
mod native_document;
mod native_document_guard;
pub mod panels;
mod property_editor;
mod quick_edit_view;
mod snapshot;
mod source;
mod status_entry;
mod style_drafts;
mod style_editor;
mod timing_editor;
mod symbol_library;
mod theme;
mod visual_draft_canvas;
mod validation;
pub use bellatrix_mde::MODELING_NAMESPACE;
pub use integration::register_mde_workspace;
pub use native_document::{NativeDocumentImporter, NativeDocumentLease};
pub use snapshot::{MdeSnapshot, from_document};
pub use source::MdeUiSource;
pub use symbol_library::MdeSymbolLibraryItem;
pub use theme::WorkbenchPalette;

use bellatrix_mde::commands::{ModelingElementKind, ModelingMemberKind, ModelingTool};
use bellatrix_mde::{
    Geometry, ModelingRelationKind, PathSegment, RelationshipGraph, UmlQuickCreate, UmlType,
    classifier_contents, relationship_presentation,
};
use bellatrix_mde_cordis::{
    MdeUiCommand, ModelingAddMemberRequested, ModelingCreateElementRequested,
    ModelingCreateRelationRequested, ModelingDeleteElementRequested,
    ModelingMoveParameterRequested, ModelingMoveViewRequested, ModelingQuickEditRequested,
    ModelingResizeViewRequested, ModelingSetNameExpressionRequested,
    ModelingSetPropertyRequested,
};
use bellatrix_project::ProjectId;
use bellatrix_ui_uml::{UmlClassifier, UmlFigure, UmlLineSegment, UmlRelationGlyph};
use gpui::{
    App, AppContext, Context, DragMoveEvent, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Render, ScrollEvent, SharedString, Task, Window, div, prelude::*, px,
};
use quick_edit_view::{ParameterAction, quick_edit_for_element};
use ui::{Clickable, Label, LabelCommon};
use ui_input::InputField;
use workspace::{Workspace, item::Item};

#[derive(Clone)]
pub struct MdeToolDrag {
    pub project_id: ProjectId,
    pub kind: ModelingTool,
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
            .child(Label::new(self.kind.label()))
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
pub struct MdeWorkbenchItem {
    source: MdeUiSource,
    focus_handle: FocusHandle,
    selected_element: Option<String>,
    selected_view: Option<String>,
    inline_editor: Option<Entity<InputField>>,
    editing_subject: Option<String>,
    draft_label_revision: Option<u64>,
    draft_label_pending: bool,
    active_diagram: Option<String>,
    selected_tool: Option<ModelingTool>,
    pending_relation_source: Option<String>,
    pending_visual_point: Option<bellatrix_mde::Point>,
    visual_draft_mode: bool,
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
#[derive(Debug, PartialEq, Eq)]
enum RelationClick {
    First,
    Repeat,
    Connect(String),
}
fn choose_relation_endpoint(pending: &mut Option<String>, clicked: &str) -> RelationClick {
    match pending.take() {
        None => {
            *pending = Some(clicked.to_owned());
            RelationClick::First
        }
        Some(from) if from == clicked => {
            *pending = Some(from);
            RelationClick::Repeat
        }
        Some(from) => RelationClick::Connect(from),
    }
}

pub(crate) fn diagram_kind_label(kind: bellatrix_mde::StructuralDiagramKind) -> &'static str {
    use bellatrix_mde::StructuralDiagramKind::*;
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

fn diagram_creation_context(
    kind: bellatrix_mde::StructuralDiagramKind,
    selected: Option<&str>,
) -> Vec<String> {
    if kind == bellatrix_mde::StructuralDiagramKind::CompositeStructure
        || kind.context_metaclass().is_some()
    {
        selected.map(str::to_owned).into_iter().collect()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
fn diagram_validation_kind(
    stored_type: &str,
) -> Result<bellatrix_mde::StructuralDiagramKind, String> {
    bellatrix_mde::StructuralDiagramKind::from_uml_di_type(stored_type)
        .ok_or_else(|| format!("Cannot validate unsupported diagram type {stored_type}"))
}

fn creation_owner(kind: ModelingElementKind, selected: Option<&str>) -> Option<String> {
    if kind.requires_owner() {
        selected.map(str::to_owned)
    } else {
        None
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
    diagram: &bellatrix_mde::Diagram,
    element_id: &str,
) -> ElementViewPlacement {
    if let Some(view) = diagram
        .views
        .iter()
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

fn switch_diagram_selection(
    active: &mut Option<String>,
    selected: &mut Option<String>,
    id: String,
) {
    if active.as_deref() != Some(id.as_str()) {
        *selected = None;
    }
    *active = Some(id);
}

impl MdeWorkbenchItem {
    pub fn new(source: MdeUiSource, cx: &mut Context<Self>) -> Self {
        let validation_mode = validation::ValidationMode::load(source.project_id.as_str());
        let initial_validation_feedback = validation_mode.as_ref().err().map(|error| format!("Could not load validation preference: {error}")).unwrap_or_default();
        let initial_diagram = source.snapshot().ok().and_then(|snapshot| {
            snapshot
                .section
                .diagrams
                .first()
                .map(|diagram| diagram.id.clone())
        });
        let mut revisions = source.revisions.clone();
        let mut feedback = source.feedback.clone();
        let mut validations = source.validations.clone();
        let mut saves = source.saves.clone();
        let task = cx.spawn(async move |this, cx| {
            loop {
                tokio::select! {
                    changed = revisions.changed() => {
                        if changed.is_err() { break; }
                        if this.update(cx, |item, cx| { item.refresh(cx); item.validate_after_change(cx); }).is_err() { break; }
                    }
                    changed = saves.changed() => {
                        if changed.is_err() { break; }
                        if this.update(cx, |item, cx| item.validate_after_save(cx)).is_err() { break; }
                    }
                    changed = validations.changed() => {
                        if changed.is_err() { break; }
                        let result = validations.borrow().clone();
                        if this.update(cx, |item, cx| item.accept_validation(result, cx)).is_err() { break; }
                    }
                    changed = feedback.changed() => {
                        if changed.is_err() { break; }
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
            focus_handle: cx.focus_handle(),
            selected_element: None,
            selected_view: None,
            inline_editor: None,
            editing_subject: None,
            draft_label_revision: None,
            draft_label_pending: false,
            active_diagram: initial_diagram,
            selected_tool: None,
            pending_relation_source: None,
            pending_visual_point: None,
            visual_draft_mode: false,
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
        self.selected_element.as_deref()
    }
    pub fn selected_tool(&self) -> Option<ModelingTool> {
        self.selected_tool
    }
    pub fn active_diagram(&self) -> Option<&str> {
        self.active_diagram.as_deref()
    }
    pub fn select_element(&mut self, id: String, cx: &mut Context<Self>) {
        if self.selected_view.take().is_some() {
            cx.notify();
        }
        if self.selected_element.as_deref() != Some(id.as_str()) {
            self.selected_element = Some(id);
            self.inline_editor = None;
            self.editing_subject = None;
            cx.notify();
        }
    }
    fn select_view(&mut self, subject: String, view_id: String, cx: &mut Context<Self>) {
        self.select_element(subject, cx);
        self.selected_view = Some(view_id);
        cx.notify();
    }

    pub fn remove_selected_view(&mut self, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Some((diagram_id, view_id)) = selected_view_target(
            self.active_diagram.as_deref(),
            self.selected_view.as_deref(),
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
            diagram.id == diagram_id && diagram.views.iter().any(|view| view.id == view_id)
        }) {
            self.feedback = "Selected diagram view no longer exists".into();
            cx.notify();
            return;
        }
        self.submit(
            MdeUiCommand::RemoveView(bellatrix_mde_cordis::ModelingRemoveViewRequested {
                project_id: self.project_id().clone(),
                diagram_id,
                view_id,
                expected_revision: snapshot.revision,
            }),
            cx,
        );
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

    fn name_expression_text(element: &bellatrix_mde::Element) -> String {
        bellatrix_mde::commands::name_expression_text(element)
    }
    pub(crate) fn run_quick_action(&mut self, action: UmlQuickCreate, cx: &mut Context<Self>) {
        let Some(element_id) = self.selected_element.clone() else {
            self.feedback = "Select a UML element first".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let Some(element) = snapshot
            .section
            .elements
            .iter()
            .find(|element| element.id == element_id)
        else {
            self.feedback = "Element was removed from the model".into();
            cx.notify();
            return;
        };
        if !action.accepts(&element.kind) {
            self.feedback = format!("{} does not support this action", element.kind);
            cx.notify();
            return;
        }
        let Some(diagram_id) = self.active_diagram.clone().or_else(|| {
            snapshot
                .section
                .diagrams
                .first()
                .map(|diagram| diagram.id.clone())
        }) else {
            self.feedback = "No active UML diagram".into();
            cx.notify();
            return;
        };
        self.submit(
            MdeUiCommand::QuickEdit(ModelingQuickEditRequested {
                project_id: self.project_id().clone(),
                element_id,
                diagram_id,
                action,
                expected_revision: snapshot.revision,
            }),
            cx,
        );
    }
    fn begin_inline_edit(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            return;
        };
        let Some(target) = snapshot
            .section
            .elements
            .iter()
            .find(|element| element.id == id)
        else {
            return;
        };
        if self.editing_subject.as_deref() == Some(id.as_str()) {
            return;
        }
        let classifier = UmlType::from_kind(&target.kind).is_some_and(UmlType::is_classifier);
        let comment = UmlType::from_kind(&target.kind) == Some(UmlType::Note);
        let hint = if comment {
            "Body"
        } else if classifier {
            "<< stereotype >> name"
        } else {
            "Name"
        };
        let field = cx.new(|cx| InputField::new(window, cx, hint).label_min_width(px(96.0)));
        let display = if comment {
            bellatrix_mde::element_display_label(target)
        } else if classifier {
            Self::name_expression_text(target)
        } else {
            bellatrix_mde::template_parametered_element(&snapshot.section, target)
                .unwrap_or(target)
                .name
                .clone()
        };
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
        if self.source.is_read_only() {
            return;
        }
        let (Some(id), Some(field)) = (self.editing_subject.clone(), self.inline_editor.clone())
        else {
            return;
        };
        let text = field.read(cx).text(cx);
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let Some(element) = snapshot
            .section
            .elements
            .iter()
            .find(|element| element.id == id)
        else {
            self.cancel_inline_edit(window, cx);
            return;
        };
        let classifier = UmlType::from_kind(&element.kind).is_some_and(UmlType::is_classifier);
        let comment = UmlType::from_kind(&element.kind) == Some(UmlType::Note);
        let name = if comment {
            text.to_string()
        } else {
            text.trim().to_owned()
        };
        let changed = name
            != if comment {
                bellatrix_mde::element_display_label(element)
            } else if classifier {
                Self::name_expression_text(element)
            } else {
                bellatrix_mde::template_parametered_element(&snapshot.section, element)
                    .unwrap_or(element)
                    .name
                    .clone()
            };
        self.editing_subject = None;
        self.inline_editor = None;
        if changed {
            if classifier {
                self.submit(
                    MdeUiCommand::SetNameExpression(ModelingSetNameExpressionRequested {
                        project_id: self.project_id().clone(),
                        element_id: id,
                        expression: name,
                        expected_revision: snapshot.revision,
                    }),
                    cx,
                );
            } else {
                self.submit(
                    MdeUiCommand::SetProperty(ModelingSetPropertyRequested {
                        project_id: self.project_id().clone(),
                        element_id: id,
                        path: if comment { "properties.body" } else { "name" }.into(),
                        value: serde_json::Value::String(name),
                        expected_revision: snapshot.revision,
                    }),
                    cx,
                );
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
    pub fn feedback(&self) -> &str {
        &self.feedback
    }
    pub fn validation_status(&self) -> &str { &self.validation_status }

    pub fn create_diagram(
        &mut self,
        kind: bellatrix_mde::StructuralDiagramKind,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let name = format!(
            "{} diagram {}",
            diagram_kind_label(kind),
            snapshot.section.diagrams.len().saturating_add(1)
        );
        let model_elements = diagram_creation_context(kind, self.selected_element.as_deref());
        self.submit(
            MdeUiCommand::CreateDiagram(bellatrix_mde_cordis::ModelingCreateDiagramRequested {
                project_id: self.project_id().clone(),
                expected_revision: snapshot.revision,
                kind,
                name,
                model_elements,
            }),
            cx,
        );
    }

    pub fn select_diagram(&mut self, id: String, cx: &mut Context<Self>) {
        if self.active_diagram.as_deref() != Some(id.as_str()) {
            self.selected_view = None;
            self.validation_status = "Not yet validated for this diagram".into();
        }

        switch_diagram_selection(&mut self.active_diagram, &mut self.selected_element, id);
        self.gesture = None;
        self.pending_relation_source = None;
        self.selected_tool = None;
        self.editing_subject = None;
        self.inline_editor = None;
        cx.notify();
    }
    pub fn select_tool(&mut self, kind: ModelingTool, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        if self.selected_tool == Some(kind) {
            self.cancel_tool(cx);
        } else {
            self.arm_drag_tool(kind, cx);
        }
    }
    pub fn cancel_tool(&mut self, cx: &mut Context<Self>) {
        self.selected_tool = None;
        self.pending_relation_source = None;
        self.feedback = "No relationship tool selected".into();
        cx.notify();
    }
    fn link_endpoint(
        &mut self,
        kind: ModelingRelationKind,
        target: String,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let from = match choose_relation_endpoint(&mut self.pending_relation_source, &target) {
            RelationClick::First => {
                self.select_element(target, cx);
                self.feedback =
                    format!("{}: source selected. Click a second element", kind.label());
                cx.notify();
                return;
            }
            RelationClick::Repeat => {
                self.feedback = format!("{}: choose another element to connect", kind.label());
                cx.notify();
                return;
            }
            RelationClick::Connect(from) => from,
        };
        self.select_element(target.clone(), cx);
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let Some(diagram_id) = self.active_diagram.clone().or_else(|| {
            snapshot
                .section
                .diagrams
                .first()
                .map(|diagram| diagram.id.clone())
        }) else {
            self.feedback = "Project has no diagram".into();
            cx.notify();
            return;
        };
        self.submit(
            MdeUiCommand::CreateRelationOrVisualDraft(ModelingCreateRelationRequested {
                project_id: self.project_id().clone(),
                diagram_id,
                kind,
                from,
                to: target,
                expected_revision: snapshot.revision,
            }),
            cx,
        );
        self.selected_tool = None;
        cx.notify();
    }
    pub fn arm_drag_tool(&mut self, tool: ModelingTool, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        self.gesture = None;
        self.inline_editor = None;
        self.editing_subject = None;
        self.selected_tool = Some(tool);
        self.pending_relation_source = None;
        self.feedback = match tool {
            ModelingTool::Relation(kind) => {
                format!("{}: {}", kind.label(), kind.endpoint_instruction())
            }
            ModelingTool::Element(kind) if kind.requires_nested_owner() => format!(
                "{}: click to place; unsupported semantic ownership is preserved as a visual draft",
                kind.label()
            ),
            ModelingTool::Element(kind) if kind.requires_owner() => format!(
                "{}: click to place; select an owner to create a semantic element",
                kind.label()
            ),
            ModelingTool::Element(kind) => format!("{}: click canvas to place", kind.label()),
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
        if let Some(ModelingTool::Relation(kind)) = self.selected_tool {
            if self.visual_draft_mode || kind.is_visual_only() {
                self.link_visual_draft_endpoint(kind, view_id, cx);
                return;
            }
            self.link_endpoint(kind, subject, cx);
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
        if self.source.is_read_only() {
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
        });
        cx.notify();
    }
    fn update_gesture(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        if !event.dragging() {
            return;
        }
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        let (start, dx, dy) = match gesture {
            CanvasGesture::Move { start, dx, dy, .. }
            | CanvasGesture::Resize { start, dx, dy, .. } => (start, dx, dy),
        };
        *dx = f64::from(event.position.x - start.x) / f64::from(self.zoom);
        *dy = f64::from(event.position.y - start.y) / f64::from(self.zoom);
        cx.notify();
    }
    fn finish_gesture(&mut self, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        let expected_revision = match &gesture {
            CanvasGesture::Move { revision, .. } | CanvasGesture::Resize { revision, .. } => {
                *revision
            }
        };
        if self
            .source
            .snapshot()
            .ok()
            .map(|snapshot| snapshot.revision)
            != Some(expected_revision)
        {
            self.feedback = "Project changed during drag; retry".into();
            cx.notify();
            return;
        }
        let command = match gesture {
            CanvasGesture::Move {
                view_id, dx, dy, ..
            } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    cx.notify();
                    return;
                }
                MdeUiCommand::MoveView(ModelingMoveViewRequested {
                    project_id: self.project_id().clone(),
                    view_id,
                    dx,
                    dy,
                    expected_revision,
                })
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
        self.source
            .snapshot()
            .map(|snapshot| format!("MDE · {}", snapshot.name).into())
            .unwrap_or_else(|_| "MDE Workbench".into())
    }
    fn show_toolbar(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod relationship_interaction_tests;
