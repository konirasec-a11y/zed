use bellatrix_mde::{Plan, Project};
use cordis::event::{ListenerRegistrationError, responder};
use cordis::{Context, Event};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize)]
struct ModelRequest<'a> {
    api_version: u32,
    model: &'a Project,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderedArtifacts {
    api_version: u32,
    artifacts: Vec<RenderedArtifact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderedArtifact {
    path: String,
    contents: Option<String>,
    origin: String,
}

fn render_document(bytes: &[u8], format: &str) -> Result<String, String> {
    let model = bellatrix_mde::decode_model_document(bytes, format)?;
    let projection = bellatrix_mde::project_documentation(&model)?;
    bellatrix_mde_html_exporter::render(&projection)
}

fn plan_from_rendered(rendered: &str) -> Result<Plan, String> {
    let response: RenderedArtifacts =
        serde_json::from_str(rendered).map_err(|error| error.to_string())?;
    if response.api_version != 1 {
        return Err("unsupported HTML artifact response version".into());
    }
    let mut plan = Plan::new();
    for artifact in response.artifacts {
        plan.insert(artifact.path.into(), artifact.contents.map(String::into_bytes), artifact.origin)
            .map_err(|error| error.to_string())?;
    }
    Ok(plan)
}

pub(crate) fn plan(model: &Project) -> Result<Plan, String> {
    let request = serde_json::to_string(&ModelRequest { api_version: 1, model })
        .map_err(|error| error.to_string())?;
    let projection = bellatrix_mde::project_documentation(&request)?;
    plan_from_rendered(&bellatrix_mde_html_exporter::render(&projection)?)
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
                let plan = plan_from_rendered(&rendered)?;
                if request.replace_generated {
                    bellatrix_mde::publish_replace(&plan, &request.destination)
                } else {
                    bellatrix_mde::publish_new(&plan, &request.destination)
                }.map_err(|error| error.to_string())?;
                Ok::<usize, String>(plan.count())
            }).await.map_err(|error| error.to_string()).and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        },
    ))?;
    Ok(())
}
