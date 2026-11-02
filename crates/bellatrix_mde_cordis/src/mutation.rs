use crate::project_integration::from_document;
use bellatrix_mde::{
    MODELING_NAMESPACE, ModelingSection,
    commands::{self, Command},
};
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::{Context, Routing, event::QueryOutcome};

pub(crate) async fn execute(
    context: &Context,
    projects: &ProjectService,
    project_id: ProjectId,
    expected_revision: u64,
    command: Command,
) -> Result<u64, String> {
    let document = projects
        .snapshot(&project_id)
        .map_err(|error| error.to_string())?;
    if document.revision() != expected_revision {
        return Err("stale MDE command; refresh project snapshot".into());
    }
    let section = from_document(&document)?;
    let source = section.to_model(document.id().as_str(), document.name());
    let updated = commands::apply(&source, &command, expected_revision)?;
    let state = serde_json::to_value(ModelingSection::from_model(&updated))
        .map_err(|error| error.to_string())?;
    match context
        .query::<ProjectReplace>(
            Routing::Unscoped,
            ProjectReplaceRequested {
                id: project_id,
                namespace: MODELING_NAMESPACE.into(),
                expected_revision,
                state,
            },
        )
        .await
        .map_err(|error| error.to_string())?
    {
        QueryOutcome::Answer(Ok(document)) => Ok(document.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService unavailable".into()),
    }
}
