use bellatrix_mde_cordis::{ModelingExecute, ModelingOperation};
use cordis::{Context, Routing, event::QueryOutcome};

pub async fn run(context: &Context, operation: ModelingOperation) -> Result<String, String> {
    match context
        .query::<ModelingExecute>(Routing::Unscoped, operation)
        .await
        .map_err(|error| error.to_string())?
    {
        QueryOutcome::Answer(result) => result,
        QueryOutcome::Miss => Err("MDE operation handler unavailable".into()),
    }
}
