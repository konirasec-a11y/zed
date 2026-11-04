use crate::artifact_publication::{self, PreparedArtifacts};
use cordis::event::{ListenerRegistrationError, responder};
use cordis::{Context, Event};
use std::{convert::Infallible, path::PathBuf};

#[derive(Clone, Debug)]
pub struct ModelingRenderHtmlRequested {
    pub bytes: Vec<u8>,
    pub format: String,
}

pub struct ModelingRenderHtml;
impl Event for ModelingRenderHtml {
    const NAME: &'static str = "bellatrix.mde.render-html.v1";
    type Args = ModelingRenderHtmlRequested;
    type Output = Result<String, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingPublishHtmlRequested {
    pub bytes: Vec<u8>,
    pub format: String,
    pub destination: PathBuf,
    pub replace_generated: bool,
}

pub struct ModelingPublishHtml;
impl Event for ModelingPublishHtml {
    const NAME: &'static str = "bellatrix.mde.publish-html.v1";
    type Args = ModelingPublishHtmlRequested;
    type Output = Result<usize, String>;
}

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
            let result = tokio::task::spawn_blocking(move || render_document(&request.bytes, &request.format))
                .await.map_err(|error| error.to_string()).and_then(|result| result);
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
            }).await.map_err(|error| error.to_string()).and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        },
    ))?;
    Ok(())
}
