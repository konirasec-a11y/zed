//! Cordis transport and project-revision integration for the single MDE domain.
pub use bellatrix_mde_contracts::events::{
    ModelingChannels, ModelingCommand, ModelingCompleted, ModelingCompletion, ModelingOperation,
    ModelingRequested,
};
use bellatrix_mde_contracts::protocol::trace as trace_contract;
pub use trace_contract::{
    DiagnosticSeverity, ExecutionEvent, ModelingDiagnostic, TraceOutcome, TracePhase,
};
mod execution_journal;
pub use bellatrix_mde_contracts::protocol::MODELING_NAMESPACE;
use bellatrix_mde_contracts::protocol::command as command_contract;
use bellatrix_mde_contracts::protocol::tools as tool_catalog;
pub use command_contract::*;
pub use execution_journal::{ExecutionJournal, ExecutionJournalSnapshot};
pub use tool_catalog::{
    DiagramToolMetadata, ElementToolMetadata, InterfaceToolMetadata, RelationToolMetadata,
    ToolCatalog, ToolGroup,
};
mod diagrams;
mod document_codec;
mod mutation;
mod trace_output;
pub use document_codec::{ModelingConvertDocument, ModelingConvertDocumentRequested};
mod mdj_interchange;
pub use diagrams::{
    DiagramValidationResult, ModelingCreateDiagram, ModelingCreateDiagramRequested,
    ModelingCreateExplorerDiagram, ModelingCreateExplorerDiagramRequested, ModelingDeleteDiagram,
    ModelingDeleteDiagramRequested, ModelingExecutionTrace, ModelingExecutionTraceEvent,
    ModelingRenameDiagram, ModelingRenameDiagramRequested, ModelingValidateDiagram,
    ModelingValidateDiagramRequested, ModelingValidateDiagramTraced,
    ModelingValidateDiagramTracedRequested, ModelingValidateSnapshot,
    ModelingValidateSnapshotRequested, TracedDiagramValidationResult,
};
pub use mdj_interchange::{
    MdjExportReceipt, ModelingDocumentFormat, ModelingExportMdj, ModelingExportMdjRequested,
    ModelingPrepareDocument, ModelingPrepareDocumentRequested, PreparedModelingDocument,
};
mod artifact_publication;
mod headless_operations;
mod rust_generation;
pub use headless_operations::ModelingExecute;
mod html_export;
pub use html_export::{
    ModelingPublishHtml, ModelingPublishHtmlRequested, ModelingRenderHtml,
    ModelingRenderHtmlRequested,
};
mod task_events;
pub use task_events::{
    ModelingRun, ModelingRunRequested, ModelingTaskCompleted, ModelingTaskCompletedEvent,
    ModelingTaskKind, ModelingTaskSummary,
};
mod name_expression;
mod project_integration;
pub use name_expression::{ModelingSetNameExpression, ModelingSetNameExpressionRequested};
mod quick_edit_commands;
pub use quick_edit_commands::{ModelingQuickEdit, ModelingQuickEditRequested};
mod create_relation;
pub use create_relation::{
    ModelingCreateRelation, ModelingCreateRelationAtViewsOrVisualDraft,
    ModelingCreateRelationAtViewsRequested, ModelingCreateRelationOrVisualDraft,
    ModelingCreateRelationRequested,
};
mod association_ends;
pub use association_ends::{ModelingEditAssociationEnds, ModelingEditAssociationEndsRequested};
mod add_member;
mod create_element;
pub use add_member::{
    ModelingAddMember, ModelingAddMemberRequested, ModelingMoveParameter,
    ModelingMoveParameterRequested,
};
mod delete_element;
mod resize_view;
mod set_property;
mod view_edits;
mod view_style;
pub use view_style::{
    ModelingReorderView, ModelingReorderViewRequested, ModelingSetViewLabel,
    ModelingSetViewLabelRequested, ModelingSetViewPath, ModelingSetViewPathRequested,
    ModelingSetViewRouting, ModelingSetViewRoutingRequested, ModelingSetViewStyle,
    ModelingSetViewStyleRequested, ModelingSetViewTiming, ModelingSetViewTimingRequested,
};
mod views;
pub use delete_element::{ModelingDeleteElement, ModelingDeleteElementRequested};
pub use resize_view::{ModelingResizeView, ModelingResizeViewRequested};
pub use set_property::{ModelingSetProperty, ModelingSetPropertyRequested};
pub use views::{
    ModelingCreateVisualDraft, ModelingCreateVisualDraftRequested, ModelingPlaceElementView,
    ModelingPlaceElementViewRequested, ModelingRemoveView, ModelingRemoveViewRequested,
};
mod ui_session;
pub use ui_session::MdeSession;
mod ui_bridge;
use cordis::event::{ListenerRegistrationError, observer_sync};
use cordis::{Context, Plugin};
pub use create_element::{
    ModelingCreateElement, ModelingCreateElementOrVisualDraft, ModelingCreateElementRequested,
    ModelingCreateModelElement, ModelingCreateModelElementRequested,
};
pub use project_integration::{
    MdePlugin, ModelingRename, ModelingRenameRequested, ModelingSetExplorerParent,
    ModelingSetExplorerParentRequested, ModelingSetOwner, ModelingSetOwnerRequested,
};
use std::convert::Infallible;
use tokio::sync::mpsc;
pub use ui_bridge::{DiagramValidationFailure, MdeUiBridge, MdeUiCommand};
pub use view_edits::{ModelingMoveView, ModelingMoveViewRequested};

pub struct ModelingPlugin {
    channels: ModelingChannels,
}
impl ModelingPlugin {
    pub fn new(
        requests: mpsc::Sender<ModelingCommand>,
        completions: mpsc::Sender<ModelingCompletion>,
    ) -> Self {
        Self {
            channels: ModelingChannels {
                requests,
                completions,
            },
        }
    }
}
impl Plugin for ModelingPlugin {
    type Config = ();
    type Input = ModelingChannels;
    type PrepareError = Infallible;
    type ApplyError = ListenerRegistrationError;
    fn prepare(&self, (): ()) -> Result<Self::Input, Self::PrepareError> {
        Ok(self.channels.clone())
    }
    async fn apply(&self, ctx: Context, input: &Self::Input) -> Result<(), Self::ApplyError> {
        crate::headless_operations::register(&ctx)?;
        crate::html_export::register(&ctx)?;
        crate::document_codec::register(&ctx)?;
        let request_sender = input.requests.clone();
        let _requested =
            ctx.on::<ModelingRequested, _>(observer_sync(move |_, command: ModelingCommand| {
                request_sender
                    .try_send(command.clone())
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
                Ok::<_, std::io::Error>(())
            }))?;
        let completion_sender = input.completions.clone();
        let _completed = ctx.on::<ModelingCompleted, _>(observer_sync(
            move |_, completion: ModelingCompletion| {
                completion_sender
                    .try_send(completion.clone())
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
                Ok::<_, std::io::Error>(())
            },
        ))?;
        Ok(())
    }
}

use bellatrix_mde_contracts::protocol::explorer as explorer_projection;
use bellatrix_mde_contracts::protocol::inspector as inspector_contract;
pub use bellatrix_mde_contracts::protocol::presentation as presentation_contract;
pub use explorer_projection::{ExplorerProjection, ExplorerRow, VisualComponent};
pub use inspector_contract::{
    ElementInspector, InspectorMember, InspectorProperty, InspectorReferenceChoice, PropertyEditor,
    QuickEditAction, QuickEditAvailability, QuickEditControl, QuickEditIntent, QuickEditSide,
};
#[path = "read_projection.rs"]
pub mod projection;
pub use projection::{ModelingReadProjection, ModelingReadProjectionRequested, ModelingSnapshot};

mod move_preview;
pub use move_preview::{
    MOVE_PREVIEW_PENDING, ModelingPreviewMove, ModelingPreviewMoveRequested, MovePreviewClient,
};
