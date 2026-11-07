//! Engine-MDE as a native Zed Workspace Item.
//! Zed owns presentation; bellatrix_mde owns semantics; Cordis publishes revisions.
use crate::view_model::workbench_state::WorkbenchState;

pub use crate::api::native_document::NativeDocumentImporter;
pub use crate::api::native_document::NativeDocumentLease;
pub use crate::api::source::MdeUiSource;
pub use crate::assets::theme::WorkbenchPalette;
use crate::components::canvas_interaction::CanvasGesture;
pub use crate::components::symbol_library::MdeSymbolLibraryItem;
pub use crate::routes::workspace::register_mde_workspace;
use crate::view_model::inspector_drafts;
use crate::view_model::selection;
pub use crate::view_model::snapshot::MdeSnapshot;
pub use bellatrix_mde_cordis::MODELING_NAMESPACE;

use bellatrix_mde_cordis::ModelingTool;
use bellatrix_mde_cordis::project::ProjectId;
use gpui::App;
use gpui::AppContext;
use gpui::Context;
use gpui::Entity;
use gpui::EventEmitter;
use gpui::FocusHandle;
use gpui::Focusable;
use gpui::IntoElement;
use gpui::Pixels;
use gpui::Point;
use gpui::Render;
use gpui::SharedString;
use gpui::Task;
use gpui::Window;
use gpui::div;
use gpui::prelude::*;
use gpui::px;
use ui::Label;
use ui_input::InputField;
use workspace::Workspace;
use workspace::item::Item;

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
pub struct MdeWorkbenchItem {
    pub(crate) source: MdeUiSource,
    pub(crate) presentation: WorkbenchState,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) inline_editor: Option<Entity<InputField>>,
    pub(crate) inline_draft: Option<inspector_drafts::DraftState>,
    pub(crate) gesture: Option<CanvasGesture>,
    pub(crate) drop_point: Option<(f64, f64)>,
    pub(crate) zoom: f32,
    pub(crate) pan: (f32, f32),
    pub(crate) pan_origin: Option<(Point<Pixels>, (f32, f32))>,
    pub(crate) feedback: String,
    pub(crate) validation_mode: crate::api::validation::ValidationMode,
    pub(crate) validation_status: String,
    pub(crate) _watch_task: Task<()>,
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

pub(crate) fn canvas_delete_action_allowed(canvas_focused: bool, editing: bool) -> bool {
    canvas_focused && !editing
}

fn next_diagram_grid_position(view_count: usize) -> (f64, f64) {
    (
        24.0 + (view_count % 4) as f64 * 200.0,
        36.0 + (view_count / 4) as f64 * 148.0,
    )
}

#[derive(Debug, PartialEq)]
pub(crate) enum ElementViewPlacement {
    Existing(String),
    NewAt(f64, f64),
}

pub(crate) fn element_view_placement(
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

pub(crate) fn canvas_model_position(
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
        let validation_mode =
            crate::api::validation::ValidationMode::load(source.project_id.as_str());
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
            validation_mode: validation_mode
                .unwrap_or(crate::api::validation::ValidationMode::Manual),
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

    pub(crate) fn remove_view(
        &mut self,
        diagram_id: String,
        view_id: String,
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

    pub(crate) fn view_context_menu(
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
                Some(Box::new(crate::routes::workspace::RemoveFromDiagram)),
                move |_, cx| {
                    remove_owner.update(cx, |item, cx| {
                        item.remove_view(diagram_id.clone(), view_id.clone(), cx)
                    });
                },
            )
            .entry(
                "Delete from model",
                Some(Box::new(crate::routes::workspace::DeleteFromModel)),
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
    pub(crate) fn allows_modeling_tool(
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
