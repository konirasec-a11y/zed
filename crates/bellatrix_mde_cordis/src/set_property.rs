//! Versioned semantic property edits validated against MDE Inspector schema.
//! The GPUI form is never permitted to mutate ProjectDocument directly.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_modeling::{Graph, set_inspected_property};
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ModelingSetPropertyRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub path: String,
    pub value: Value,
    pub expected_revision: u64,
}
pub struct ModelingSetProperty;
impl Event for ModelingSetProperty {
    const NAME: &'static str = "bellatrix.mde.set-property.v1";
    type Args = ModelingSetPropertyRequested;
    type Output = Result<u64, String>;
}

pub(crate) async fn set_property(
    ctx: &Context, projects: &ProjectService, request: ModelingSetPropertyRequested,
) -> Result<u64, String> {
    let document = projects.snapshot(&request.project_id).map_err(|e| e.to_string())?;
    if document.revision() != request.expected_revision {
        return Err("stale Inspector edit; reload project revision".into());
    }
    let mut section = from_document(&document)?;
    let namespace = namespace_for(&document);
    set_inspected_property(&mut section, &request.element_id, &request.path, request.value)?;
    Graph::new(&section.to_model(document.id().as_str(), document.name()))
        .validate().map_err(|e| e.to_string())?;
    let value = serde_json::to_value(section).map_err(|e| e.to_string())?;
    match ctx.query::<ProjectReplace>(Routing::Unscoped, ProjectReplaceRequested {
        id: request.project_id, namespace: namespace.into(),
        expected_revision: request.expected_revision, state: value,
    }).await.map_err(|e| e.to_string())? {
        QueryOutcome::Answer(Ok(updated)) => Ok(updated.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService unavailable".into()),
    }
}
