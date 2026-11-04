use bellatrix_mde::{
    Format, Graph, Project, decode, decode_model_document, publish_new, publish_replace,
    validate_graph_model,
};
use bellatrix_mde_cordis::{
    ModelingConvertDocument, ModelingConvertDocumentRequested, ModelingOperation,
    ModelingPublishHtml, ModelingPublishHtmlRequested,
};
use cordis::{Context, Routing, event::QueryOutcome};
use serde_json::Value;
use std::{fs, path::Path};

fn format(path: &Path) -> Format {
    if matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("pb" | "bellatrix")
    ) {
        Format::Protobuf
    } else {
        Format::Json
    }
}
fn load(path: &Path) -> Result<Project, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    decode(&bytes, format(path)).map_err(|error| error.to_string())
}
fn load_validated_document(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let document_format = if matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("pb" | "bellatrix")
    ) {
        "protobuf"
    } else {
        "json"
    };
    let envelope = decode_model_document(&bytes, document_format)?;
    let validation: Value = serde_json::from_str(&validate_graph_model(&envelope)?)
        .map_err(|error| error.to_string())?;
    match validation.get("has_errors").and_then(Value::as_bool) {
        Some(false) => {}
        Some(true) => {
            let message = validation
                .get("diagnostics")
                .and_then(Value::as_array)
                .and_then(|diagnostics| diagnostics.first())
                .and_then(|diagnostic| diagnostic.get("message"))
                .and_then(Value::as_str)
                .ok_or_else(|| "invalid kernel validation response".to_string())?;
            return Err(message.to_string());
        }
        None => return Err("invalid kernel validation response".into()),
    }
    let mut document: Value = serde_json::from_str(&envelope).map_err(|error| error.to_string())?;
    document
        .get_mut("model")
        .map(Value::take)
        .ok_or_else(|| "kernel response is missing model".to_string())
}

fn document_array<'a>(document: &'a Value, field: &str) -> Result<&'a Vec<Value>, String> {
    document
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("kernel response is missing model array: {field}"))
}

fn document_text<'a>(document: &'a Value, field: &str) -> Result<&'a str, String> {
    document
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("kernel response is missing text field: {field}"))
}

pub async fn run(context: &Context, operation: ModelingOperation) -> Result<String, String> {
    match operation {
        ModelingOperation::ExportHtml { source, destination, replace } => {
            let document_format = format_name(&source).to_owned();
            let bytes = tokio::task::spawn_blocking(move || fs::read(source))
                .await.map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())?;
            let response = context.query::<ModelingPublishHtml>(Routing::Unscoped, ModelingPublishHtmlRequested {
                bytes, format: document_format, destination: destination.clone(), replace_generated: replace,
            }).await.map_err(|error| error.to_string())?;
            let count = match response {
                QueryOutcome::Answer(result) => result?,
                QueryOutcome::Miss => return Err("MDE HTML export handler unavailable".into()),
            };
            Ok(format!("Exported {count} HTML/SVG artifacts to {}", destination.display()))
        }
        ModelingOperation::Convert { source, destination } => {
            let source_format = format_name(&source).to_owned();
            let destination_format = format_name(&destination).to_owned();
            let bytes = tokio::task::spawn_blocking(move || fs::read(source))
                .await.map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())?;
            let response = context.query::<ModelingConvertDocument>(Routing::Unscoped, ModelingConvertDocumentRequested {
                bytes, source_format, destination_format,
            }).await.map_err(|error| error.to_string())?;
            let bytes = match response {
                QueryOutcome::Answer(result) => result?,
                QueryOutcome::Miss => return Err("MDE document codec handler unavailable".into()),
            };
            tokio::task::spawn_blocking(move || {
                use std::io::Write;
                let mut output = fs::OpenOptions::new().write(true).create_new(true)
                    .open(&destination).map_err(|error| error.to_string())?;
                output.write_all(&bytes).map_err(|error| error.to_string())?;
                Ok::<String, String>(format!("Wrote {}", destination.display()))
            }).await.map_err(|error| error.to_string())?
        }
        operation => tokio::task::spawn_blocking(move || run_local(operation))
            .await.map_err(|error| error.to_string())?,
    }
}

fn format_name(path: &Path) -> &'static str {
    if matches!(path.extension().and_then(|extension| extension.to_str()), Some("pb" | "bellatrix")) {
        "protobuf"
    } else {
        "json"
    }
}

fn run_local(operation: ModelingOperation) -> Result<String, String> {
    match operation {
        ModelingOperation::Check { source } => {
            let model = load_validated_document(&source)?;
            Ok(format!(
                "Valid bellatrix-mde-model: {} elements, {} relations, {} diagrams",
                document_array(&model, "elements")?.len(),
                document_array(&model, "relations")?.len(),
                document_array(&model, "diagrams")?.len()
            ))
        }
        ModelingOperation::Inspect { source } => {
            let model = load_validated_document(&source)?;
            let mut lines = vec![format!(
                "Project: {} ({})",
                document_text(&model, "name")?,
                document_text(&model, "id")?
            )];
            for element in document_array(&model, "elements")? {
                lines.push(format!(
                    "{} | {} | {}",
                    document_text(element, "id")?,
                    document_text(element, "kind")?,
                    document_text(element, "name")?
                ));
            }
            for relation in document_array(&model, "relations")? {
                lines.push(format!(
                    "{} --[{}]--> {}",
                    document_text(relation, "from")?,
                    document_text(relation, "kind")?,
                    document_text(relation, "to")?
                ));
            }
            Ok(lines.join("\n"))
        }
        ModelingOperation::Analyze {
            source,
            from,
            to,
            relation,
        } => {
            let model = load(&source)?;
            Graph::new(&model)
                .validate()
                .map_err(|error| error.to_string())?;
            crate::graph_report::report(&model, from.as_deref(), to.as_deref(), relation.as_deref())
        }
        ModelingOperation::Plan {
            source,
            scope,
            skeleton,
        } => {
            let model = load(&source)?;
            let plan = bellatrix_mde_rust_generator::plan_with_options(
                &model,
                &scope,
                bellatrix_mde_rust_generator::GenerationOptions { skeleton },
            )
            .map_err(|error| error.to_string())?;
            let mut lines = plan
                .artifacts()
                .map(|artifact| format!("{} [{}]", artifact.path.display(), artifact.origin))
                .collect::<Vec<_>>();
            for diagnostic in &plan.diagnostics {
                lines.push(format!("WARN {}: {}", diagnostic.code, diagnostic.message));
            }
            Ok(lines.join("\n"))
        }
        ModelingOperation::Generate {
            source,
            scope,
            destination,
            replace,
            skeleton,
        } => {
            let model = load(&source)?;
            let plan = bellatrix_mde_rust_generator::plan_with_options(
                &model,
                &scope,
                bellatrix_mde_rust_generator::GenerationOptions { skeleton },
            )
            .map_err(|error| error.to_string())?;
            if replace {
                publish_replace(&plan, &destination)
            } else {
                publish_new(&plan, &destination)
            }
            .map_err(|error| error.to_string())?;
            Ok(format!(
                "Generated {} artifacts in {} ({} warnings)",
                plan.count(),
                destination.display(),
                plan.diagnostics.len()
            ))
        }
        ModelingOperation::ExportHtml { .. } | ModelingOperation::Convert { .. } => {
            Err("this operation requires its Cordis handler".into())
        }
    }
}
