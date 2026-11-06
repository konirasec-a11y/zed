use crate::artifact_publication::{self, PreparedArtifacts};
pub use bellatrix_mde_contracts::events::{
    ModelingPublishHtml, ModelingPublishHtmlRequested, ModelingRenderHtml,
    ModelingRenderHtmlRequested,
};
use cordis::event::{ListenerRegistrationError, responder};
use cordis::Context;
use std::convert::Infallible;

fn render_document(bytes: &[u8], format: &str) -> Result<String, String> {
    let model = bellatrix_mde::decode_model_document(bytes, format)?;
    let projection = bellatrix_mde::project_documentation(&model)?;
    bellatrix_mde_html_exporter::render(&projection)
}

pub(crate) fn prepare(model_envelope: &str) -> Result<PreparedArtifacts, String> {
    let projection = bellatrix_mde::project_documentation(model_envelope)?;
    artifact_publication::prepare(&bellatrix_mde_html_exporter::render(&projection)?)
}

pub(crate) fn register(context: &Context) -> Result<(), ListenerRegistrationError> {
    let _render = context.on::<ModelingRenderHtml, _>(responder(
        move |_: Context, request: ModelingRenderHtmlRequested| async move {
            let result = tokio::task::spawn_blocking(move || {
                render_document(&request.bytes, &request.format)
            })
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        },
    ))?;
    let _publish = context.on::<ModelingPublishHtml, _>(responder(
        move |_: Context, request: ModelingPublishHtmlRequested| async move {
            let result = tokio::task::spawn_blocking(move || {
                let rendered = render_document(&request.bytes, &request.format)?;
                let prepared = artifact_publication::prepare(&rendered)?;
                prepared.publish(&request.destination, request.replace_generated)?;
                Ok::<usize, String>(prepared.artifact_count)
            })
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        },
    ))?;
    Ok(())
}
