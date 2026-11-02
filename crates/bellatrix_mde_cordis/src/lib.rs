//! Cordis transport and project-revision integration for the single MDE domain.
pub use bellatrix_mde::MODELING_NAMESPACE;
pub use bellatrix_mde::commands::{ModelingElementKind, ModelingTool};
pub use bellatrix_mde::{ModelingRelationKind, ModelingSection, UmlQuickCommand};
mod diagrams;
mod mutation;
pub use diagrams::{
    ModelingCreateDiagram, ModelingCreateDiagramRequested, ModelingDeleteDiagram,
    ModelingDeleteDiagramRequested, ModelingRenameDiagram, ModelingRenameDiagramRequested,
    ModelingValidateDiagram, ModelingValidateDiagramRequested,
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
mod interface_quick_edit;
pub use interface_quick_edit::{
    InterfaceQuickAction, ModelingInterfaceQuickEdit, ModelingInterfaceQuickEditRequested,
};
mod create_relation;
pub use create_relation::{
    ModelingCreateRelation, ModelingCreateRelationRequested, projected_automatic_routes,
};
mod add_member;
mod create_element;
pub use add_member::{ModelingAddMember, ModelingAddMemberRequested, ModelingMemberKind};
mod delete_element;
mod resize_view;
mod set_property;
mod view_edits;
pub use delete_element::{ModelingDeleteElement, ModelingDeleteElementRequested};
pub use resize_view::{ModelingResizeView, ModelingResizeViewRequested};
pub use set_property::{ModelingSetProperty, ModelingSetPropertyRequested};
mod ui_bridge;
use bellatrix_mde::Scope;
use cordis::event::{ListenerRegistrationError, observer_sync};
use cordis::{Context, Event, Plugin};
pub use create_element::{ModelingCreateElement, ModelingCreateElementRequested};
pub use project_integration::{MdePlugin, ModelingRename, ModelingRenameRequested, with_model};
use std::convert::Infallible;
use tokio::sync::mpsc;
pub use ui_bridge::{MdeUiBridge, MdeUiCommand};
pub use view_edits::{ModelingMoveView, ModelingMoveViewRequested};

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
        scope: Scope,
        skeleton: bool,
    },
    Generate {
        source: std::path::PathBuf,
        scope: Scope,
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
impl Event for ModelingRequested {
    const NAME: &'static str = "bellatrix.mde.requested.v1";
    type Args = ModelingCommand;
    type Output = ();
}
#[derive(Clone, Debug)]
pub struct ModelingCompletion {
    pub correlation_id: u64,
    pub success: bool,
    pub summary: String,
}
pub struct ModelingCompleted;
impl Event for ModelingCompleted {
    const NAME: &'static str = "bellatrix.mde.completed.v1";
    type Args = ModelingCompletion;
    type Output = ();
}
#[derive(Clone)]
pub struct ModelingChannels {
    pub requests: mpsc::Sender<ModelingCommand>,
    pub completions: mpsc::Sender<ModelingCompletion>,
}
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
#[cfg(test)]
mod tests {
    use super::*;
    use cordis::Routing;
    #[tokio::test]
    async fn cordis_routes_command_through_tokio_and_disposes() {
        let ctx = Context::new();
        let (tx, mut rx) = mpsc::channel(4);
        let (done_tx, mut done_rx) = mpsc::channel(4);
        let plugin = ModelingPlugin::new(tx, done_tx);
        let input = plugin.prepare(()).unwrap();
        let prepared = cordis::PreparedPlugin::from_input(plugin, input);
        let handle = ctx.spawn(prepared).await.unwrap();
        ctx.emit::<ModelingRequested>(
            Routing::Unscoped,
            ModelingCommand {
                correlation_id: 42,
                operation: ModelingOperation::Check {
                    source: "todo.json".into(),
                },
            },
        )
        .await
        .unwrap();
        let command = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(command.correlation_id, 42);
        ctx.emit::<ModelingCompleted>(
            Routing::Unscoped,
            ModelingCompletion {
                correlation_id: 42,
                success: true,
                summary: "done".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(5), done_rx.recv())
                .await
                .unwrap()
                .unwrap()
                .correlation_id,
            42
        );
        handle.dispose().await.unwrap();
    }
}
