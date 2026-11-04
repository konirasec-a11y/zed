use crate::Plan;
use crate::api_contract::{API_VERSION, check_version, error_response, invalid_request};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    api_version: u32,
    artifacts: Vec<Artifact>,
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    contents: Option<Contents>,
    origin: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Contents {
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    code: String,
    message: String,
    element: Option<String>,
}

fn decode_plan(input_json: &str) -> Result<Plan, String> {
    let manifest: Manifest = serde_json::from_str(input_json).map_err(invalid_request)?;
    check_version(manifest.api_version)?;
    let mut plan = Plan::new();
    for artifact in manifest.artifacts {
        let contents = artifact.contents.map(|contents| match contents {
            Contents::Text(text) => text.into_bytes(),
            Contents::Bytes(bytes) => bytes,
        });
        plan.insert(artifact.path.into(), contents, artifact.origin)
            .map_err(|error| error_response("INVALID_ARTIFACT_PLAN", error.to_string()))?;
    }
    for diagnostic in manifest.diagnostics {
        plan.warn(
            &diagnostic.code,
            diagnostic.message,
            diagnostic.element.as_deref(),
        );
    }
    Ok(plan)
}

/// Validate renderer artifacts against the existing kernel publication plan.
///
/// Accepts `{api_version:1, artifacts:[{path,contents,origin}], diagnostics?}`.
/// Contents are UTF-8 text, exact byte arrays, or null for directories. Returns
/// a canonical byte manifest, counts and the existing human-readable plan report.
/// Safe relative paths, case-insensitive collisions and file/directory conflicts
/// are checked by the kernel, not reimplemented by the transport or renderer.
/// No I/O, model mutation, clocks or retained state. Errors are versioned as
/// `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION` or `INVALID_ARTIFACT_PLAN`.
pub fn prepare_artifact_publication(input_json: &str) -> Result<String, String> {
    let plan = decode_plan(input_json)?;
    let artifacts = plan
        .artifacts()
        .map(|artifact| {
            let path = artifact.path.to_str().ok_or_else(|| {
                error_response("INVALID_ARTIFACT_PLAN", "artifact path is not UTF-8")
            })?;
            Ok(json!({"path": path, "contents": artifact.contents, "origin": artifact.origin}))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut report = plan
        .artifacts()
        .map(|artifact| format!("{} [{}]", artifact.path.display(), artifact.origin))
        .collect::<Vec<_>>();
    for diagnostic in &plan.diagnostics {
        report.push(format!("WARN {}: {}", diagnostic.code, diagnostic.message));
    }
    Ok(json!({
        "api_version": API_VERSION,
        "manifest": {"api_version": API_VERSION, "artifacts": artifacts, "diagnostics": plan.diagnostics},
        "artifact_count": plan.count(), "warning_count": plan.diagnostics.len(),
        "report": report.join("\n"),
    }).to_string())
}

/// Publish a renderer manifest using the existing staged artifact publisher.
///
/// This application boundary performs filesystem I/O at the caller's explicit
/// destination. The manifest is revalidated before any publication. Replacement
/// uses the existing generated-file ownership manifest and symlink safeguards;
/// false means the destination must not exist. No UML/model state is changed.
/// Errors use the plan contract or `ARTIFACT_PUBLICATION_FAILED`.
pub fn publish_artifact_manifest(
    input_json: &str,
    destination: &Path,
    replace_generated: bool,
) -> Result<(), String> {
    let plan = decode_plan(input_json)?;
    let result = if replace_generated {
        crate::publish_replace(&plan, destination)
    } else {
        crate::publish_new(&plan, destination)
    };
    result.map_err(|error| error_response("ARTIFACT_PUBLICATION_FAILED", error.to_string()))
}
