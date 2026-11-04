//! Engine-MDE as a native Zed Workspace Item.
//! Zed owns presentation; bellatrix_mde owns semantics; Cordis publishes revisions.
#[path = "../view-model/workbench_state.rs"]
mod workbench_state;
use workbench_state::{PendingRelationEndpoint, SelectionChange, WorkbenchState};

#[path = "diagram_canvas.rs"]
mod diagram_canvas;
#[path = "../view-model/imported_preview.rs"]
mod imported_preview;
#[path = "execution_dashboard.rs"]
mod execution_dashboard;
#[path = "../view-model/explorer.rs"]
mod explorer;
#[path = "hierarchy.rs"]
mod hierarchy;
#[path = "../view-model/inspector_drafts.rs"]
mod inspector_drafts;
#[path = "integration.rs"]
mod integration;
#[path = "model_editing.rs"]
mod model_editing;
#[path = "native_document.rs"]
mod native_document;
#[path = "../view-model/native_document_guard.rs"]
mod native_document_guard;
#[path = "panels.rs"]
pub mod panels;
#[path = "property_editor.rs"]
mod property_editor;
#[path = "quick_edit_view.rs"]
mod quick_edit_view;
#[path = "relationship_creation.rs"]
mod relationship_creation;
#[path = "../view-model/selection.rs"]
mod selection;
#[path = "../view-model/snapshot.rs"]
mod snapshot;
#[path = "source.rs"]
mod source;
#[path = "status_entry.rs"]
mod status_entry;
#[path = "../view-model/style_drafts.rs"]
mod style_drafts;
#[path = "style_editor.rs"]
mod style_editor;
#[path = "timing_editor.rs"]
mod timing_editor;
#[path = "symbol_library.rs"]
mod symbol_library;
#[path = "theme.rs"]
mod theme;
#[path = "visual_draft_canvas.rs"]
mod visual_draft_canvas;
#[path = "validation.rs"]
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
    ModelingDeleteElementRequested,
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
        began: bool,
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
    },
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
        self.source.record_interaction("selection.element", Some(&id), bellatrix_mde::TraceOutcome::Success);
        let change = self.presentation.select_element(id);
        self.apply_selection_change(change, cx);
    }

    pub fn select_explorer_element(
        &mut self, id: String, key: String, occurrence: Option<bellatrix_mde::ViewOccurrence>,
        cx: &mut Context<Self>,
    ) {
        self.source.record_interaction("selection.explorer_member", Some(&key), bellatrix_mde::TraceOutcome::Success);
        let change = self.presentation.select_explorer_element(id, key, occurrence);
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
        self.source.record_interaction("selection.view", Some(&view_id), bellatrix_mde::TraceOutcome::Success);
        let change = self.presentation.select_view(Some(subject), view_id);
        self.apply_selection_change(change, cx);
        cx.notify();
    }

    pub fn select_diagram_view(&mut self, diagram_id: String, view_id: String, cx: &mut Context<Self>) {
        if self.source.is_presentation_suspended() {
            self.source.record_interaction("selection.view.suspended", Some(&view_id), bellatrix_mde::TraceOutcome::Invalid);
            return;
        }
        let selection = self.source.snapshot().and_then(|snapshot| {
            snapshot.section.diagrams.iter().find(|diagram| diagram.id == diagram_id)
                .and_then(|diagram| diagram.all_views().find(|view| view.id == view_id))
                .map(|view| view.subject.clone())
                .ok_or_else(|| "The original diagram View no longer exists".to_owned())
        });
        let subject = match selection {
            Ok(subject) => subject,
            Err(error) => {
                self.source.record_interaction("selection.view.missing", Some(&view_id), bellatrix_mde::TraceOutcome::Invalid);
                self.feedback = error; cx.notify(); return;
            }
        };
        self.source.record_interaction("selection.view", Some(&view_id), bellatrix_mde::TraceOutcome::Success);
        self.select_diagram(diagram_id, cx);
        let change = self.presentation.select_view(subject, view_id);
        self.apply_selection_change(change, cx);
        self.feedback = if self.presentation.selected_element().is_none() {
            "Visual View selected; no semantic subject is required".into()
        } else { "Diagram View selected".into() };
        cx.notify();
    }

    fn selected_view_is_inspection_only(&self) -> bool {
        let Some(view_id) = self.presentation.selected_view() else {
            return false;
        };
        self.source.snapshot().ok().is_none_or(|snapshot| {
            snapshot.section.diagrams.iter()
                .find(|diagram| Some(diagram.id.as_str()) == self.presentation.active_diagram())
                .is_none_or(|diagram| {
                    if self.source.is_mdj() {
                        !diagram.views.iter().any(|view| view.id == view_id && view.presentation.source.is_some())
                    } else {
                        !selection::supports_view_editing(diagram, view_id)
                    }
                })
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
        let Some(element_id) = self.presentation.selected_element().map(str::to_owned) else {
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
        let Some(diagram_id) = self.presentation.active_diagram().map(str::to_owned).or_else(|| {
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
        if self.source.is_read_only() || self.selected_view_is_inspection_only() {
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
        if self.presentation.editing_element() == Some(id.as_str()) {
            if let Some(field) = &self.inline_editor {
                field.read(cx).focus_handle(cx).focus(window, cx);
                self.gesture = None;
                return;
            }
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
        if self.selected_element() != Some(id.as_str()) {
            self.select_element(id.clone(), cx);
        }
        self.presentation.begin_element_edit(id);
        self.inline_editor = Some(field.clone());
        self.inline_draft = Some(inspector_drafts::DraftState::new(display, snapshot.revision));
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
        let (Some(id), Some(field)) = (self.presentation.editing_element().map(str::to_owned), self.inline_editor.clone())
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
        let canonical = if comment {
            bellatrix_mde::element_display_label(element)
        } else if classifier {
            Self::name_expression_text(element)
        } else {
            bellatrix_mde::template_parametered_element(&snapshot.section, element)
                .unwrap_or(element)
                .name
                .clone()
        };
        let Some(draft) = self.inline_draft.as_mut() else {
            self.feedback = "Edit baseline unavailable; cancel and reopen the editor".into();
            cx.notify();
            return;
        };
        let reconciled = match draft.reconcile_for_commit(&text, &canonical, snapshot.revision) {
            Ok(text) => text,
            Err(error) => {
                self.feedback = error;
                cx.notify();
                return;
            }
        };
        let name = if comment {
            reconciled
        } else {
            reconciled.trim().to_owned()
        };
        let changed = name != canonical;
        self.presentation.clear_edit();
        self.inline_editor = None;
        self.inline_draft = None;
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
        self.presentation.clear_edit();
        self.inline_editor = None;
        self.inline_draft = None;
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
        let model_elements = diagram_creation_context(kind, self.presentation.selected_element());
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
        self.source.record_interaction("selection.diagram", Some(&id), bellatrix_mde::TraceOutcome::Success);
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
            self.source.record_interaction("tool.select.read_only", Some(kind.label()), bellatrix_mde::TraceOutcome::Invalid);
            return;
        }
        if self.presentation.selected_tool() == Some(kind) {
            self.cancel_tool(cx);
        } else {
            self.arm_drag_tool(kind, cx);
        }
    }
    pub fn cancel_tool(&mut self, cx: &mut Context<Self>) {
        self.source.record_interaction("tool.cancel", None, bellatrix_mde::TraceOutcome::Success);
        self.presentation.cancel_tool();
        self.feedback = "No relationship tool selected".into();
        cx.notify();
    }
    pub fn arm_drag_tool(&mut self, tool: ModelingTool, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            self.source.record_interaction("tool.arm.read_only", Some(tool.label()), bellatrix_mde::TraceOutcome::Invalid);
            return;
        }
        self.source.record_interaction("tool.arm", Some(tool.label()), bellatrix_mde::TraceOutcome::Success);
        self.gesture = None;
        self.inline_editor = None;
        self.inline_draft = None;
        self.presentation.arm_tool(tool);
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
            ModelingTool::ProvidedInterface | ModelingTool::RequiredInterface => format!("{}: click canvas to place", tool.label()),
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
        if let Some(ModelingTool::Relation(kind)) = self.presentation.selected_tool() {
            self.connect_relation_view(kind, view_id, None, cx);
            return;
        }
        self.select_view(subject, view_id.clone(), cx);
        if self.source.is_read_only() || self.source.is_mdj() {
            return;
        }
        self.gesture = Some(CanvasGesture::Move {
            view_id,
            start: position,
            dx: 0.0,
            dy: 0.0,
            revision,
            began: false,
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
        if self.source.is_read_only() || self.source.is_mdj() {
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
        });
        cx.notify();
    }
    fn update_gesture(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.source.is_read_only() || self.source.is_mdj() {
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) || !event.dragging() {
            return;
        }
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        let (start, dx, dy, began, view_id, action) = match gesture {
            CanvasGesture::Move { start, dx, dy, began, view_id, .. } =>
                (start, dx, dy, began, view_id, "canvas.move.start"),
            CanvasGesture::Resize { start, dx, dy, began, view_id, .. } =>
                (start, dx, dy, began, view_id, "canvas.resize.start"),
        };
        *dx = f64::from(event.position.x - start.x) / f64::from(self.zoom);
        *dy = f64::from(event.position.y - start.y) / f64::from(self.zoom);
        if !*began && (dx.abs() >= 1.0 || dy.abs() >= 1.0) {
            *began = true;
            self.source.record_interaction(action, Some(view_id), bellatrix_mde::TraceOutcome::Success);
        }
        cx.notify();
    }
    fn finish_gesture(&mut self, cx: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        let (view_id, began) = match &gesture {
            CanvasGesture::Move { view_id, began, .. } | CanvasGesture::Resize { view_id, began, .. } => (view_id, *began),
        };
        if !began {
            cx.notify();
            return;
        }
        if self.source.is_read_only() || self.source.is_mdj() {
            self.source.record_interaction("canvas.gesture.read_only", Some(view_id), bellatrix_mde::TraceOutcome::Invalid);
            cx.notify();
            return;
        }
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
            let view_id = match &gesture {
                CanvasGesture::Move { view_id, .. } | CanvasGesture::Resize { view_id, .. } => view_id,
            };
            self.source.record_interaction("canvas.gesture.stale_revision", Some(view_id), bellatrix_mde::TraceOutcome::Invalid);
            self.feedback = "Project changed during drag; retry".into();
            cx.notify();
            return;
        }
        let command = match gesture {
            CanvasGesture::Move {
                view_id, dx, dy, ..
            } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    self.source.record_interaction("canvas.gesture.no_change", Some(&view_id), bellatrix_mde::TraceOutcome::Success);
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
                    self.source.record_interaction("canvas.gesture.no_change", Some(&view_id), bellatrix_mde::TraceOutcome::Success);
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
