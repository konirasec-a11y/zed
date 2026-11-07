use crate::api_contract::{API_VERSION, decode_model, error_response, invalid_request};
use crate::{GenerationGraphError, Graph, analyze_generation_graph};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    selected_ids: Vec<String>,
}

/// Analyze explicit generation dependencies without rendering or publishing files.
///
/// Input fields: `api_version:1`, `model`, `selected_ids` (source element IDs).
/// Selection is exact: duplicates collapse, no ownership/dependency closure is
/// added, empty selection succeeds, and unknown/non-element IDs are errors.
/// Graph integrity (not presentation or full UML conformance) is required.
/// Output fields: `api_version`, `coverage:"generation_dependency_analysis"`,
/// and `units` in prerequisite-first order, with independent ties ordered by ID.
/// Each unit has `id`, `kind`, `name`, and `requires` entries containing `id`,
/// `kind`, `name`, `relation_kind`. Requirements include unselected dependencies.
/// Only selected-to-selected ordering edges constrain scheduling; cycles outside
/// that selected subgraph are not a scheduling error for this call.
///
/// # Errors
/// Versioned JSON errors: `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`,
/// `INVALID_MODEL`, `GRAPH_INTEGRITY`, `UNKNOWN_SELECTION`, `GENERATION_CYCLE`,
/// or `GENERATION_UNSUPPORTED_TARGET` (ordering dependency targets a non-element).
/// This analysis does not certify target-language generation or UML conformance.
///
/// # Effects and determinism
/// No I/O, clock, sink, external callback or retained state. Input is unchanged;
/// output is independent of call history. No internal compiler type is exposed.
pub fn analyze_generation_dependencies(input_json: &str) -> Result<String, String> {
    let request: AnalysisRequest = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    Graph::new(&model)
        .validate_integrity()
        .map_err(|error| error_response("GRAPH_INTEGRITY", error.to_string()))?;
    for id in &request.selected_ids {
        if model.element(id).is_none() {
            return Err(error_response(
                "UNKNOWN_SELECTION",
                format!("unknown source element: {id}"),
            ));
        }
    }
    let selected: HashSet<String> = request.selected_ids.into_iter().collect();
    let analysis = analyze_generation_graph(&model, &selected)
        // A cycle witness can depend on insertion order. The contract reports
        // the invariant failure, not that implementation-specific witness.
        .map_err(|error| match error {
            GenerationGraphError::Cycle(_) => error_response("GENERATION_CYCLE", "selected generation dependencies contain a cycle"),
            GenerationGraphError::UnsupportedDependencyTarget { relation, target } => error_response(
                "GENERATION_UNSUPPORTED_TARGET",
                format!("generation dependency {relation} targets {target}, which is not a source element"),
            ),
        })?;
    let units = analysis
        .units
        .iter()
        .map(|unit| {
            let requires = unit
                .requires
                .iter()
                .map(|reference| {
                    json!({
                        "id":reference.id, "kind":reference.kind, "name":reference.name,
                        "relation_kind":reference.relation_kind,
                    })
                })
                .collect::<Vec<_>>();
            json!({"id":unit.id,"kind":unit.kind,"name":unit.name,"requires":requires})
        })
        .collect::<Vec<_>>();
    Ok(json!({"api_version":API_VERSION,"coverage":"generation_dependency_analysis","units":units}).to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    element_id: String,
}

/// Lower one UML Class to a standalone Rust library artifact in memory.
///
/// Accepts `{api_version:1, model, element_id}` and returns `artifacts` with
/// deterministic relative `path`, UTF-8 `contents` and source `origin`, plus
/// source-ID `trace` and an explicit Rust library target.
/// Checks graph integrity and applies the target mapping, path policy, artifact
/// plan and Rust renderer. Unsupported mappings return a versioned
/// `RUST_STRUCT_UNSUPPORTED` error.
/// No project placement/dependency resolution, persisted target model, I/O or
/// compiler invocation. Source names are preserved, not validated as Rust;
/// only an external `rustc --crate-type lib` can establish target compilation.
pub fn generate_rust_struct(input_json: &str) -> Result<String, String> {
    let request: StructRequest = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    Graph::new(&model)
        .validate_integrity()
        .map_err(|error| error_response("GRAPH_INTEGRITY", error.to_string()))?;
    let projection = crate::uml_rust_struct::project(&model, &request.element_id)
        .map_err(|error| error_response("RUST_STRUCT_UNSUPPORTED", error))?;
    let rendered = crate::rust_renderer::render(
        &json!({
            "api_version": API_VERSION, "emission": projection["emission"],
        })
        .to_string(),
    )
    .map_err(|error| error_response("RUST_EMISSION_FAILED", error))?;
    let rendered: serde_json::Value = serde_json::from_str(&rendered)
        .map_err(|error| error_response("RUST_EMISSION_FAILED", error.to_string()))?;
    let artifacts = rendered["artifacts"]
        .as_array()
        .ok_or_else(|| error_response("RUST_EMISSION_FAILED", "missing rendered artifacts"))?;
    let [artifact] = artifacts.as_slice() else {
        return Err(error_response(
            "RUST_EMISSION_FAILED",
            "expected one struct artifact",
        ));
    };
    let path = artifact["path"]
        .as_str()
        .ok_or_else(|| error_response("RUST_EMISSION_FAILED", "missing artifact path"))?;
    let bytes: Vec<u8> = serde_json::from_value(artifact["contents"].clone())
        .map_err(|error| error_response("RUST_EMISSION_FAILED", error.to_string()))?;
    let contents = String::from_utf8(bytes)
        .map_err(|error| error_response("RUST_EMISSION_FAILED", error.to_string()))?;
    let mut plan = crate::Plan::new();
    plan.file(path, contents.as_bytes().to_vec(), &request.element_id)
        .map_err(|error| error_response("INVALID_ARTIFACT_PLAN", error.to_string()))?;
    Ok(json!({
        "api_version": API_VERSION,
        "coverage": "uml_class_to_rust_struct_subset",
        "target": {"language": "rust", "edition": "2024", "crate_type": "lib"},
        "artifacts": [{"path": path, "contents": contents, "origin": request.element_id}],
        "trace": projection["trace"],
        "diagnostics": rendered["diagnostics"],
    })
    .to_string())
}
