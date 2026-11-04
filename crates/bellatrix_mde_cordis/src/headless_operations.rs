use crate::ModelingOperation;
use cordis::event::{ListenerRegistrationError, responder};
use cordis::{Context, Event};
use serde_json::Value;
use std::{convert::Infallible, fs, path::Path};

/// Execute a headless integration operation without exposing domain objects.
/// Sources and destinations are explicit caller-selected paths. Reports are text;
/// validation, generation and publication remain behind the Cordis boundary.
pub struct ModelingExecute;
impl Event for ModelingExecute {
    const NAME: &'static str = "bellatrix.mde.execute.v1";
    type Args = ModelingOperation;
    type Output = Result<String, String>;
}

fn format_name(path: &Path) -> &'static str {
    if matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("pb" | "bellatrix")
    ) {
        "protobuf"
    } else {
        "json"
    }
}

pub(crate) fn register(context: &Context) -> Result<(), ListenerRegistrationError> {
    let _execute = context.on::<ModelingExecute, _>(responder(
        move |_: Context, operation: ModelingOperation| async move {
            let result = tokio::task::spawn_blocking(move || execute(operation))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        },
    ))?;
    Ok(())
}

fn load(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    bellatrix_mde::decode_model_document(&bytes, format_name(path))
}

fn report(
    source: &Path,
    operation: &str,
    from: Option<String>,
    to: Option<String>,
    relation: Option<String>,
) -> Result<String, String> {
    let mut request: Value =
        serde_json::from_str(&load(source)?).map_err(|error| error.to_string())?;
    let fields = request
        .as_object_mut()
        .ok_or("model envelope must be an object")?;
    fields.insert("operation".into(), operation.into());
    for (key, value) in [("from", from), ("to", to), ("relation", relation)] {
        fields.insert(key.into(), value.map_or(Value::Null, Value::String));
    }
    let response: Value =
        serde_json::from_str(&bellatrix_mde::report_model_document(&request.to_string())?)
            .map_err(|error| error.to_string())?;
    response
        .get("report")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "headless report response missing report".to_string())
}

fn execute(operation: ModelingOperation) -> Result<String, String> {
    match operation {
        ModelingOperation::Check { source } => report(&source, "check", None, None, None),
        ModelingOperation::Inspect { source } => report(&source, "inspect", None, None, None),
        ModelingOperation::Analyze {
            source,
            from,
            to,
            relation,
        } => report(&source, "analyze", from, to, relation),
        ModelingOperation::Plan {
            source,
            element_ids,
            skeleton,
        } => {
            let model = load(&source)?;
            let prepared = crate::rust_generation::prepare(&model, element_ids, skeleton)?;
            Ok(prepared.report)
        }
        ModelingOperation::Generate {
            source,
            element_ids,
            destination,
            replace,
            skeleton,
        } => {
            let model = load(&source)?;
            let prepared = crate::rust_generation::prepare(&model, element_ids, skeleton)?;
            prepared.publish(&destination, replace)?;
            Ok(format!(
                "Generated {} artifacts in {} ({} warnings)",
                prepared.artifact_count,
                destination.display(),
                prepared.warning_count
            ))
        }
        ModelingOperation::ExportHtml {
            source,
            destination,
            replace,
        } => {
            let model = load(&source)?;
            let prepared = crate::html_export::prepare(&model)?;
            prepared.publish(&destination, replace)?;
            Ok(format!(
                "Exported {} HTML/SVG artifacts to {}",
                prepared.artifact_count,
                destination.display()
            ))
        }
        ModelingOperation::Convert {
            source,
            destination,
        } => {
            use std::io::Write;
            let bytes = fs::read(&source).map_err(|error| error.to_string())?;
            let bytes = bellatrix_mde::convert_model_document(
                &bytes,
                format_name(&source),
                format_name(&destination),
            )?;
            let parent = destination
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let mut output =
                tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
            output
                .write_all(&bytes)
                .map_err(|error| error.to_string())?;
            output
                .as_file()
                .sync_all()
                .map_err(|error| error.to_string())?;
            // Publish only a complete staged file, and never overwrite a file
            // created by someone else between reading and publishing. On error
            // the owned temporary file removes only its own staging path.
            output
                .persist_noclobber(&destination)
                .map_err(|error| error.to_string())?;
            Ok(format!("Wrote {}", destination.display()))
        }
    }
}
