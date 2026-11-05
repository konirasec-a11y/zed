use crate::project::{ProjectDocument, ProjectId, ProjectPrepareEncodedSaveRequested};
use crate::protocol::command::*;
use crate::protocol::projection::ModelingProjection;
use crate::protocol::trace::{ExecutionEvent, ModelingDiagnostic};
#[cfg(feature = "cordis")]
use cordis::Event;
use serde_json::Value;
use std::{collections::VecDeque, path::PathBuf, sync::Arc};
#[cfg(feature = "cordis")]
use tokio::sync::{mpsc, watch};

#[derive(Clone, Debug)]
pub struct ModelingAddMemberRequested {
    pub project_id: ProjectId,
    pub owner_id: String,
    pub kind: ModelingMemberKind,
    pub expected_revision: u64,
}

pub struct ModelingAddMember;

#[derive(Clone, Debug)]
pub struct ModelingMoveParameterRequested {
    pub project_id: ProjectId,
    pub parameter_id: String,
    pub up: bool,
    pub expected_revision: u64,
}

pub struct ModelingMoveParameter;

#[cfg(feature = "cordis")]
impl Event for ModelingAddMember {
    const NAME: &'static str = "bellatrix.mde.add-member.v1";
    type Args = ModelingAddMemberRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingMoveParameter {
    const NAME: &'static str = "bellatrix.mde.move-parameter.v1";
    type Args = ModelingMoveParameterRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingEditAssociationEndsRequested {
    pub project_id: ProjectId,
    pub association_id: String,
    pub edits: Vec<AssociationEndEdit>,
    pub expected_revision: u64,
}

pub struct ModelingEditAssociationEnds;

#[cfg(feature = "cordis")]
impl Event for ModelingEditAssociationEnds {
    const NAME: &'static str = "bellatrix.mde.edit-association-ends.v1";
    type Args = ModelingEditAssociationEndsRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingCreateElementRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub kind: ModelingElementKind,
    pub name: Option<String>,
    pub owner_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub expected_revision: u64,
}

pub struct ModelingCreateElement;

pub struct ModelingCreateElementOrVisualDraft;

#[derive(Clone, Debug)]
pub struct ModelingCreateModelElementRequested {
    pub project_id: ProjectId,
    pub kind: ModelingElementKind,
    pub name: Option<String>,
    pub owner_id: Option<String>,
    pub expected_revision: u64,
}

pub struct ModelingCreateModelElement;

#[cfg(feature = "cordis")]
impl Event for ModelingCreateElement {
    const NAME: &'static str = "bellatrix.mde.create-element.v1";
    type Args = ModelingCreateElementRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingCreateElementOrVisualDraft {
    const NAME: &'static str = "bellatrix.mde.create-element-or-visual-draft.v1";
    type Args = ModelingCreateElementRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingCreateModelElement {
    const NAME: &'static str = "bellatrix.mde.create-model-element.v1";
    type Args = ModelingCreateModelElementRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingCreateRelationRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub owner_id: Option<String>,
    pub kind: ModelingRelationKind,
    pub from: String,
    pub to: String,
    pub expected_revision: u64,
}

#[derive(Clone, Debug)]
pub struct ModelingCreateRelationAtViewsRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub owner_id: Option<String>,
    pub kind: ModelingRelationKind,
    pub source_view_id: String,
    pub target_view_id: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub expected_revision: u64,
}

pub struct ModelingCreateRelationAtViewsOrVisualDraft;

pub struct ModelingCreateRelation;

pub struct ModelingCreateRelationOrVisualDraft;

#[cfg(feature = "cordis")]
impl Event for ModelingCreateRelationAtViewsOrVisualDraft {
    const NAME: &'static str = "bellatrix.mde.create-relation-at-views-or-visual-draft.v1";
    type Args = ModelingCreateRelationAtViewsRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingCreateRelation {
    const NAME: &'static str = "bellatrix.mde.create-relation.v1";
    type Args = ModelingCreateRelationRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingCreateRelationOrVisualDraft {
    const NAME: &'static str = "bellatrix.mde.create-relation-or-visual-draft.v1";
    type Args = ModelingCreateRelationRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingDeleteElementRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub expected_revision: u64,
}

pub struct ModelingDeleteElement;

#[cfg(feature = "cordis")]
impl Event for ModelingDeleteElement {
    const NAME: &'static str = "bellatrix.mde.delete-element.v1";
    type Args = ModelingDeleteElementRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingCreateDiagramRequested {
    pub project_id: ProjectId,
    pub kind: StructuralDiagramKind,
    pub name: String,
    pub model_elements: Vec<String>,
    pub expected_revision: u64,
}

pub struct ModelingCreateDiagram;

#[derive(Clone, Debug)]
pub struct ModelingCreateExplorerDiagramRequested {
    pub diagram: ModelingCreateDiagramRequested,
    pub parent_id: Option<String>,
    pub parent_occurrence: Option<ViewOccurrence>,
}

pub struct ModelingCreateExplorerDiagram;

#[derive(Clone, Debug)]
pub struct ModelingRenameDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub name: String,
    pub expected_revision: u64,
}

pub struct ModelingRenameDiagram;

#[derive(Clone, Debug)]
pub struct ModelingDeleteDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub expected_revision: u64,
}

pub struct ModelingDeleteDiagram;

/// Validation uses the required stored UMLDI type; callers cannot override it.
#[derive(Clone, Debug)]
pub struct ModelingValidateDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub expected_revision: u64,
}

#[derive(Clone, Debug)]
pub struct DiagramValidationResult {
    pub diagram_id: String,
    pub revision: u64,
    pub diagnostics: Vec<ModelingDiagnostic>,
}

pub struct ModelingValidateDiagram;

#[derive(Clone)]
pub struct ModelingValidateSnapshotRequested {
    pub document: Arc<ProjectDocument>,
    pub diagram_id: String,
}

pub struct ModelingValidateSnapshot;

/// Events produced by an MDE run, suitable for a UI observer or JSONL export.
#[derive(Clone, Debug)]
pub struct ModelingExecutionTrace {
    pub project_id: ProjectId,
    pub run_id: String,
    pub events: Vec<ExecutionEvent>,
    pub dropped: u64,
}

pub struct ModelingExecutionTraceEvent;

/// Opt-in validation request that returns bounded, correlated execution events.
#[derive(Clone, Debug)]
pub struct ModelingValidateDiagramTracedRequested {
    pub request: ModelingValidateDiagramRequested,
    pub run_id: String,
    pub capacity: usize,
}

#[derive(Debug)]
pub struct TracedDiagramValidationResult {
    pub result: Result<DiagramValidationResult, String>,
    pub events: Vec<ExecutionEvent>,
    pub dropped: u64,
}

pub struct ModelingValidateDiagramTraced;

#[cfg(feature = "cordis")]
impl Event for ModelingCreateDiagram {
    const NAME: &'static str = "bellatrix.mde.create-diagram.v1";
    type Args = ModelingCreateDiagramRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingCreateExplorerDiagram {
    const NAME: &'static str = "bellatrix.mde.create-explorer-diagram.v1";
    type Args = ModelingCreateExplorerDiagramRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingRenameDiagram {
    const NAME: &'static str = "bellatrix.mde.rename-diagram.v1";
    type Args = ModelingRenameDiagramRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingDeleteDiagram {
    const NAME: &'static str = "bellatrix.mde.delete-diagram.v1";
    type Args = ModelingDeleteDiagramRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingValidateDiagram {
    const NAME: &'static str = "bellatrix.mde.validate-diagram.v1";
    type Args = ModelingValidateDiagramRequested;
    type Output = Result<DiagramValidationResult, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingValidateSnapshot {
    const NAME: &'static str = "bellatrix.mde.validate-snapshot.v1";
    type Args = ModelingValidateSnapshotRequested;
    type Output = Result<DiagramValidationResult, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingExecutionTraceEvent {
    const NAME: &'static str = "bellatrix.mde.execution-trace.v1";
    type Args = ModelingExecutionTrace;
    type Output = ();
}

#[cfg(feature = "cordis")]
impl Event for ModelingValidateDiagramTraced {
    const NAME: &'static str = "bellatrix.mde.validate-diagram-traced.v1";
    type Args = ModelingValidateDiagramTracedRequested;
    type Output = TracedDiagramValidationResult;
}

#[derive(Clone, Debug)]
pub struct ModelingConvertDocumentRequested {
    pub bytes: Vec<u8>,
    pub source_format: String,
    pub destination_format: String,
}

pub struct ModelingConvertDocument;

#[cfg(feature = "cordis")]
impl Event for ModelingConvertDocument {
    const NAME: &'static str = "bellatrix.mde.convert-document.v1";
    type Args = ModelingConvertDocumentRequested;
    type Output = Result<Vec<u8>, String>;
}

#[derive(Clone, Default)]
pub struct ExecutionJournalSnapshot {
    pub version: u64,
    pub events: VecDeque<ExecutionEvent>,
    pub evicted_events: u64,
    pub producer_dropped_events: u64,
    pub producer_loss: VecDeque<(String, u64)>,
}

/// Execute a headless integration operation without exposing domain objects.
/// Sources and destinations are explicit caller-selected paths. Reports are text;
/// validation, generation and publication remain behind the Cordis boundary.
pub struct ModelingExecute;

#[cfg(feature = "cordis")]
impl Event for ModelingExecute {
    const NAME: &'static str = "bellatrix.mde.execute.v1";
    type Args = ModelingOperation;
    type Output = Result<String, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingRenderHtmlRequested {
    pub bytes: Vec<u8>,
    pub format: String,
}

pub struct ModelingRenderHtml;

#[derive(Clone, Debug)]
pub struct ModelingPublishHtmlRequested {
    pub bytes: Vec<u8>,
    pub format: String,
    pub destination: PathBuf,
    pub replace_generated: bool,
}

pub struct ModelingPublishHtml;

#[cfg(feature = "cordis")]
impl Event for ModelingRenderHtml {
    const NAME: &'static str = "bellatrix.mde.render-html.v1";
    type Args = ModelingRenderHtmlRequested;
    type Output = Result<String, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingPublishHtml {
    const NAME: &'static str = "bellatrix.mde.publish-html.v1";
    type Args = ModelingPublishHtmlRequested;
    type Output = Result<usize, String>;
}

#[derive(Clone, Debug)]
pub enum ModelingOperation {
    Check {
        source: std::path::PathBuf,
    },
    Inspect {
        source: std::path::PathBuf,
    },
    Analyze {
        source: std::path::PathBuf,
        from: Option<String>,
        to: Option<String>,
        relation: Option<String>,
    },
    Plan {
        source: std::path::PathBuf,
        /// None selects the project; Some([]) preserves an explicitly empty selection.
        element_ids: Option<Vec<String>>,
        skeleton: bool,
    },
    Generate {
        source: std::path::PathBuf,
        /// None selects the project; Some([]) preserves an explicitly empty selection.
        element_ids: Option<Vec<String>>,
        destination: std::path::PathBuf,
        replace: bool,
        skeleton: bool,
    },
    ExportHtml {
        source: std::path::PathBuf,
        destination: std::path::PathBuf,
        replace: bool,
    },
    Convert {
        source: std::path::PathBuf,
        destination: std::path::PathBuf,
    },
}

#[derive(Clone, Debug)]
pub struct ModelingCommand {
    pub correlation_id: u64,
    pub operation: ModelingOperation,
}

pub struct ModelingRequested;

#[derive(Clone, Debug)]
pub struct ModelingCompletion {
    pub correlation_id: u64,
    pub success: bool,
    pub summary: String,
}

pub struct ModelingCompleted;

#[cfg(feature = "cordis")]
#[derive(Clone)]
pub struct ModelingChannels {
    pub requests: mpsc::Sender<ModelingCommand>,
    pub completions: mpsc::Sender<ModelingCompletion>,
}

#[cfg(feature = "cordis")]
impl Event for ModelingRequested {
    const NAME: &'static str = "bellatrix.mde.requested.v1";
    type Args = ModelingCommand;
    type Output = ();
}

#[cfg(feature = "cordis")]
impl Event for ModelingCompleted {
    const NAME: &'static str = "bellatrix.mde.completed.v1";
    type Args = ModelingCompletion;
    type Output = ();
}

#[derive(Clone, Copy, Debug)]
pub enum ModelingDocumentFormat {
    ProjectJson,
    ModelJson,
    ModelProtobuf,
    StarUml,
}

#[derive(Clone, Debug)]
pub struct ModelingPrepareDocumentRequested {
    pub bytes: Vec<u8>,
    pub format: ModelingDocumentFormat,
}

#[derive(Clone, Debug)]
pub struct PreparedModelingDocument {
    pub document: ProjectDocument,
    pub diagnostics: Vec<String>,
}

pub struct ModelingPrepareDocument;

#[derive(Clone)]
pub struct ModelingExportMdjRequested {
    pub document: Arc<ProjectDocument>,
}

pub struct ModelingExportMdj;

/// Verified bytes for one immutable snapshot, not a write reservation or a save acknowledgement.
/// Native adapters must still guard source/destination changes and acknowledge actual I/O.
#[derive(Clone, Debug)]
pub struct MdjExportReceipt {
    document: Arc<ProjectDocument>,
    bytes: Arc<[u8]>,
}

#[cfg(feature = "cordis")]
impl Event for ModelingPrepareDocument {
    const NAME: &'static str = "bellatrix.mde.prepare-document.v1";
    type Args = ModelingPrepareDocumentRequested;
    type Output = Result<PreparedModelingDocument, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingExportMdj {
    const NAME: &'static str = "bellatrix.mde.export-mdj.v1";
    type Args = ModelingExportMdjRequested;
    type Output = Result<MdjExportReceipt, String>;
}

impl MdjExportReceipt {
    /// The producer is responsible for validating this encoding against its snapshot.
    pub fn from_encoded(document: Arc<ProjectDocument>, bytes: Arc<[u8]>) -> Self {
        Self { document, bytes }
    }

    pub fn document(&self) -> &Arc<ProjectDocument> {
        &self.document
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn matches_snapshot(&self, snapshot: &Arc<ProjectDocument>) -> bool {
        Arc::ptr_eq(&self.document, snapshot)
    }

    pub fn prepare_save(&self, path: std::path::PathBuf) -> ProjectPrepareEncodedSaveRequested {
        ProjectPrepareEncodedSaveRequested {
            document: Arc::clone(&self.document),
            path,
            bytes: Arc::clone(&self.bytes),
        }
    }
}

#[derive(Clone)]
pub struct ModelingPreviewMoveRequested {
    pub document: Arc<ProjectDocument>,
    pub subject_id: String,
    pub parent_id: Option<String>,
    pub parent_occurrence: Option<ViewOccurrence>,
    pub diagram: bool,
}

pub struct ModelingPreviewMove;

#[cfg(feature = "cordis")]
impl Event for ModelingPreviewMove {
    const NAME: &'static str = "bellatrix.mde.preview-move.v1";
    type Args = ModelingPreviewMoveRequested;
    type Output = Result<(), String>;
}

#[derive(Clone, Debug)]
pub struct ModelingSetNameExpressionRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub expression: String,
    pub expected_revision: u64,
}

pub struct ModelingSetNameExpression;

#[cfg(feature = "cordis")]
impl Event for ModelingSetNameExpression {
    const NAME: &'static str = "bellatrix.mde.set-name-expression.v1";
    type Args = ModelingSetNameExpressionRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone)]
pub struct ModelingRenameRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub new_name: String,
    pub expected_revision: u64,
}

pub struct ModelingRename;

#[derive(Clone, Debug)]
pub struct ModelingSetOwnerRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub owner_id: Option<String>,
    pub expected_revision: u64,
}

pub struct ModelingSetOwner;

#[derive(Clone, Debug)]
pub struct ModelingSetExplorerParentRequested {
    pub project_id: ProjectId,
    pub subject_id: String,
    pub parent_id: Option<String>,
    pub parent_occurrence: Option<ViewOccurrence>,
    pub expected_revision: u64,
}

pub struct ModelingSetExplorerParent;

#[cfg(feature = "cordis")]
impl Event for ModelingRename {
    const NAME: &'static str = "bellatrix.mde.rename.v1";
    type Args = ModelingRenameRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingSetOwner {
    const NAME: &'static str = "bellatrix.mde.set-owner.v1";
    type Args = ModelingSetOwnerRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingSetExplorerParent {
    const NAME: &'static str = "bellatrix.mde.set-explorer-parent.v1";
    type Args = ModelingSetExplorerParentRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingQuickEditRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub element_id: String,
    pub view_id: Option<String>,
    pub action: UmlQuickCreate,
    pub expected_revision: u64,
}

pub struct ModelingQuickEdit;

#[cfg(feature = "cordis")]
impl Event for ModelingQuickEdit {
    const NAME: &'static str = "bellatrix.mde.quick-edit.v1";
    type Args = ModelingQuickEditRequested;
    type Output = Result<u64, String>;
}

/// A derived read result, never a document, command or persistence input.
#[derive(Clone, Debug)]
pub struct ModelingSnapshot {
    pub project_id: ProjectId,
    pub name: String,
    pub revision: u64,
    pub section: ModelingProjection,
    document: Arc<ProjectDocument>,
}

#[derive(Clone)]
pub struct ModelingReadProjectionRequested {
    pub document: Arc<ProjectDocument>,
}

pub struct ModelingReadProjection;

#[cfg(feature = "cordis")]
impl Event for ModelingReadProjection {
    const NAME: &'static str = "bellatrix.mde.read-projection.v1";
    type Args = ModelingReadProjectionRequested;
    type Output = Result<ModelingSnapshot, String>;
}

impl ModelingSnapshot {
    pub fn new(document: Arc<ProjectDocument>, section: ModelingProjection) -> Self {
        Self {
            project_id: document.id().clone(),
            name: document.name().to_owned(),
            revision: document.revision(),
            section,
            document,
        }
    }

    pub fn matches_document(&self, document: &Arc<ProjectDocument>) -> bool {
        Arc::ptr_eq(&self.document, document)
    }

    pub fn matches_session(&self, document: &Arc<ProjectDocument>) -> bool {
        Arc::ptr_eq(
            &self.document.session_identity(),
            &document.session_identity(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct ModelingResizeViewRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub width: f64,
    pub height: f64,
    pub expected_revision: u64,
}

pub struct ModelingResizeView;

#[cfg(feature = "cordis")]
impl Event for ModelingResizeView {
    const NAME: &'static str = "bellatrix.mde.resize-view.v1";
    type Args = ModelingResizeViewRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingSetPropertyRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub path: String,
    pub value: Value,
    pub expected_revision: u64,
}

pub struct ModelingSetProperty;

#[cfg(feature = "cordis")]
impl Event for ModelingSetProperty {
    const NAME: &'static str = "bellatrix.mde.set-property.v1";
    type Args = ModelingSetPropertyRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub enum ModelingTaskKind {
    /// None selects the project; Some([]) must not widen an empty selection.
    GenerateRust {
        element_ids: Option<Vec<String>>,
        skeleton: bool,
    },
    ExportHtml,
}

#[derive(Clone, Debug)]
pub struct ModelingRunRequested {
    pub project_id: ProjectId,
    pub correlation_id: u64,
    pub expected_revision: u64,
    pub destination: PathBuf,
    pub replace_generated: bool,
    pub task: ModelingTaskKind,
}

pub struct ModelingRun;

#[derive(Clone, Debug)]
pub struct ModelingTaskSummary {
    pub project_id: ProjectId,
    pub correlation_id: u64,
    pub revision: u64,
    pub artifact_count: usize,
    pub destination: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ModelingTaskCompleted {
    pub project_id: ProjectId,
    pub correlation_id: u64,
    pub outcome: Result<ModelingTaskSummary, String>,
}

pub struct ModelingTaskCompletedEvent;

#[cfg(feature = "cordis")]
impl Event for ModelingRun {
    const NAME: &'static str = "bellatrix.mde.run.v1";
    type Args = ModelingRunRequested;
    type Output = Result<ModelingTaskSummary, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingTaskCompletedEvent {
    const NAME: &'static str = "bellatrix.mde.task-completed.v1";
    type Args = ModelingTaskCompleted;
    type Output = ();
}

#[derive(Clone, Debug)]
pub struct DiagramValidationFailure {
    pub diagram_id: String,
    pub revision: u64,
    pub message: String,
}

#[cfg(feature = "cordis")]
#[derive(Clone)]
pub enum MdeUiCommand {
    SetViewLabel(ModelingSetViewLabelRequested),
    SetViewLabelTracked {
        request: ModelingSetViewLabelRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    SetViewTimingTracked {
        request: ModelingSetViewTimingRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    SetViewStyle(ModelingSetViewStyleRequested),
    SetViewStyleTracked {
        request: ModelingSetViewStyleRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    SetViewPath(ModelingSetViewPathRequested),
    SetViewPathTracked {
        request: ModelingSetViewPathRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    SetViewRouting(ModelingSetViewRoutingRequested),
    SetViewRoutingTracked {
        request: ModelingSetViewRoutingRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    ReorderViewTracked {
        request: ModelingReorderViewRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    CreateDiagram(ModelingCreateDiagramRequested),
    CreateExplorerDiagram(ModelingCreateExplorerDiagramRequested),
    RenameDiagram(ModelingRenameDiagramRequested),
    DeleteDiagram(ModelingDeleteDiagramRequested),
    ValidateDiagram(ModelingValidateDiagramRequested),
    CreateVisualDraft(ModelingCreateVisualDraftRequested),
    CreateElementOrVisualDraft(ModelingCreateElementRequested),
    CreateElementVisualDraft(ModelingCreateElementRequested),
    CreateElement(ModelingCreateElementRequested),
    CreateModelElement(ModelingCreateModelElementRequested),
    CreateRelationOrVisualDraft(ModelingCreateRelationRequested),
    CreateRelationAtViewsOrVisualDraft(ModelingCreateRelationAtViewsRequested),
    CreateRelation(ModelingCreateRelationRequested),
    MoveView(ModelingMoveViewRequested),
    ResizeView(ModelingResizeViewRequested),
    RemoveView(ModelingRemoveViewRequested),
    PlaceElementView(ModelingPlaceElementViewRequested),
    Rename(ModelingRenameRequested),
    SetOwner(ModelingSetOwnerRequested),
    SetExplorerParent(ModelingSetExplorerParentRequested),
    DeleteElement(ModelingDeleteElementRequested),
    SetProperty(ModelingSetPropertyRequested),
    SetPropertyTracked {
        request: ModelingSetPropertyRequested,
        completion: watch::Sender<Option<Result<u64, String>>>,
    },
    AddMember(ModelingAddMemberRequested),
    MoveParameter(ModelingMoveParameterRequested),
    QuickEdit(ModelingQuickEditRequested),
    SetNameExpression(ModelingSetNameExpressionRequested),
    SaveProject(crate::project::ProjectSaveRequested),
}

#[derive(Clone, Debug)]
pub struct ModelingMoveViewRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub dx: f64,
    pub dy: f64,
    pub expected_revision: u64,
}

pub struct ModelingMoveView;

#[cfg(feature = "cordis")]
impl Event for ModelingMoveView {
    const NAME: &'static str = "bellatrix.mde.move-view.v1";
    type Args = ModelingMoveViewRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingSetViewLabelRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub label: Option<String>,
    pub expected_revision: u64,
}

pub struct ModelingSetViewLabel;

#[derive(Clone, Debug)]
pub struct ModelingSetViewTimingRequested {
    pub project_id: ProjectId,
    pub view_id: String,
    pub timing: Option<TimingPresentation>,
    pub expected_revision: u64,
}

pub struct ModelingSetViewTiming;

#[derive(Clone, Debug)]
pub struct ModelingSetViewStyleRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub style: Style,
    pub expected_revision: u64,
}

pub struct ModelingSetViewStyle;

#[derive(Clone, Debug)]
pub struct ModelingSetViewPathRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub geometry: Geometry,
    pub expected_revision: u64,
}

pub struct ModelingSetViewPath;

#[derive(Clone, Debug)]
pub struct ModelingSetViewRoutingRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub routing: RelationshipRouting,
    pub expected_revision: u64,
}

pub struct ModelingSetViewRouting;

#[derive(Clone, Debug)]
pub struct ModelingReorderViewRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub order: ViewOrder,
    pub expected_revision: u64,
}

pub struct ModelingReorderView;

#[cfg(feature = "cordis")]
impl Event for ModelingSetViewLabel {
    const NAME: &'static str = "bellatrix.mde.set-view-label.v1";
    type Args = ModelingSetViewLabelRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingSetViewTiming {
    const NAME: &'static str = "bellatrix.mde.set-view-timing.v1";
    type Args = ModelingSetViewTimingRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingSetViewStyle {
    const NAME: &'static str = "bellatrix.mde.set-view-style.v1";
    type Args = ModelingSetViewStyleRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingSetViewPath {
    const NAME: &'static str = "bellatrix.mde.set-view-path.v1";
    type Args = ModelingSetViewPathRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingSetViewRouting {
    const NAME: &'static str = "bellatrix.mde.set-view-routing.v1";
    type Args = ModelingSetViewRoutingRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingReorderView {
    const NAME: &'static str = "bellatrix.mde.reorder-view.v1";
    type Args = ModelingReorderViewRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingCreateVisualDraftRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub notation_kind: String,
    pub label: Option<String>,
    pub geometry: Geometry,
    pub properties: std::collections::BTreeMap<String, serde_json::Value>,
    pub parent_view_id: Option<String>,
    pub source_view_id: Option<String>,
    pub target_view_id: Option<String>,
    pub expected_revision: u64,
}

pub struct ModelingCreateVisualDraft;

#[derive(Clone, Debug)]
pub struct ModelingRemoveViewRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub view_id: String,
    pub expected_revision: u64,
}

pub struct ModelingRemoveView;

#[derive(Clone, Debug)]
pub struct ModelingPlaceElementViewRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
    pub expected_revision: u64,
}

pub struct ModelingPlaceElementView;

#[cfg(feature = "cordis")]
impl Event for ModelingCreateVisualDraft {
    const NAME: &'static str = "bellatrix.mde.create-visual-draft.v1";
    type Args = ModelingCreateVisualDraftRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingRemoveView {
    const NAME: &'static str = "bellatrix.mde.remove-view.v1";
    type Args = ModelingRemoveViewRequested;
    type Output = Result<u64, String>;
}

#[cfg(feature = "cordis")]
impl Event for ModelingPlaceElementView {
    const NAME: &'static str = "bellatrix.mde.place-element-view.v1";
    type Args = ModelingPlaceElementViewRequested;
    type Output = Result<u64, String>;
}
