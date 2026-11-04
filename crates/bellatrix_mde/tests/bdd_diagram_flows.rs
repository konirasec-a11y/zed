use bellatrix_mde::{
    Format, Project, decode_model_document, encode, validate_uml_diagram, validate_uml_model,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

type FlowResult<T> = Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    file: String,
    expected: String,
    diagram_kind: Option<String>,
    diagram_id: Option<String>,
    #[serde(default)]
    diagrams: Vec<Diagram>,
    diagnostics: Vec<ExpectedDiagnostic>,
}
#[derive(Deserialize)]
struct Diagram {
    diagram_id: String,
    diagram_kind: String,
}
#[derive(Deserialize)]
struct ExpectedDiagnostic {
    code: String,
    subject_id: String,
    severity: String,
}

struct SavedFlow(PathBuf);
impl Drop for SavedFlow {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not clean BDD flow directory {}: {error}",
                self.0.display()
            );
        }
    }
}

fn validate(case: &Case, envelope: &Value) -> FlowResult<Value> {
    let response = match (&case.diagram_kind, &case.diagram_id) {
        (Some(kind), Some(id)) => validate_uml_diagram(
            &json!({
                "api_version":1, "model":envelope["model"], "diagram_kind":kind, "diagram_id":id
            })
            .to_string(),
        ),
        (None, None) => validate_uml_model(&envelope.to_string()),
        _ => return Err("diagram_kind and diagram_id must be present together".into()),
    }
    .map_err(std::io::Error::other)?;
    Ok(serde_json::from_str(&response)?)
}

fn check_outcome(case: &Case, response: &Value) -> FlowResult<()> {
    let expected_errors = match case.expected.as_str() {
        "valid" => false,
        "invalid" => true,
        _ => {
            return Err(format!(
                "{}: expected must be valid or invalid, never Unknown",
                case.id
            )
            .into());
        }
    };
    let diagnostics = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?;
    for diagnostic in diagnostics {
        if !matches!(diagnostic["severity"].as_str(), Some("error" | "warning")) {
            return Err(format!("Unknown diagnostic severity: {diagnostic}").into());
        }
        if diagnostic["severity"] == "warning" {
            println!(
                "{}: retained warning, not proof of conformance: {diagnostic}",
                case.id
            );
        }
    }
    let has_errors = response["has_errors"]
        .as_bool()
        .ok_or("missing boolean has_errors")?;
    if has_errors
        != diagnostics
            .iter()
            .any(|diagnostic| diagnostic["severity"] == "error")
    {
        return Err(format!(
            "{}: has_errors contradicts its diagnostics: {response}",
            case.id
        )
        .into());
    }
    if has_errors != expected_errors {
        return Err(format!("{}: expected {}, actual {response}", case.id, case.expected).into());
    }
    if expected_errors
        && !case
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error")
    {
        return Err("Invalid flow must specify an exact error code, subject and severity".into());
    }
    for expected in &case.diagnostics {
        if !diagnostics.iter().any(|actual| {
            actual["code"] == expected.code
                && actual["subject_id"] == expected.subject_id
                && actual["severity"] == expected.severity
        }) {
            return Err(format!(
                "{}: missing {}/{}/{} in {response}",
                case.id, expected.code, expected.subject_id, expected.severity
            )
            .into());
        }
    }
    let mut expected_errors: Vec<_> = case
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == "error")
        .map(|diagnostic| {
            (
                diagnostic.code.as_str(),
                diagnostic.subject_id.as_str(),
                "error",
            )
        })
        .collect();
    let mut actual_errors = Vec::new();
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
    {
        actual_errors.push((
            diagnostic["code"]
                .as_str()
                .ok_or("diagnostic missing code")?,
            diagnostic["subject_id"]
                .as_str()
                .ok_or("diagnostic missing subject_id")?,
            "error",
        ));
    }
    expected_errors.sort_unstable();
    actual_errors.sort_unstable();
    if actual_errors != expected_errors {
        return Err(format!("{}: exact error diagnostics differ: expected {expected_errors:?}, actual {actual_errors:?}", case.id).into());
    }
    Ok(())
}

fn diagram_responses(
    case: &Case,
    envelope: &Value,
    failures: &mut Vec<String>,
) -> FlowResult<Vec<Value>> {
    case.diagrams
        .iter()
        .map(|diagram| {
            let response: Value = serde_json::from_str(
                &validate_uml_diagram(
                    &json!({
                        "api_version":1, "model":envelope["model"],
                        "diagram_id":diagram.diagram_id, "diagram_kind":diagram.diagram_kind
                    })
                    .to_string(),
                )
                .map_err(std::io::Error::other)?,
            )?;
            if let Err(error) = check_outcome(case, &response) {
                failures.push(format!("diagram {}: {error}", diagram.diagram_id));
            }
            Ok(response)
        })
        .collect()
}

fn differences(left: &Value, right: &Value, path: &str, output: &mut Vec<String>) {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            for key in left.keys().chain(right.keys()).collect::<BTreeSet<_>>() {
                let child = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => differences(left, right, &child, output),
                    _ => output.push(child),
                }
            }
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                differences(left, right, &format!("{path}/{index}"), output);
            }
        }
        _ if left != right => output.push(path.to_owned()),
        _ => {}
    }
}

fn run_flow(group: &str, id: &str) -> FlowResult<()> {
    let feature = include_str!("features/diagram_lifecycle.feature");
    let examples: Vec<_> = feature
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .filter_map(|line| line.split('|').nth(1))
        .map(str::trim)
        .filter(|id| *id != "case_id")
        .collect();
    if examples.len() != 30
        || examples.iter().collect::<BTreeSet<_>>().len() != 30
        || !examples.contains(&id)
    {
        return Err(format!(
            "Gherkin inventory must contain exactly 30 unique scenarios including {id}"
        )
        .into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/bdd")
        .join(group);
    let manifest: Manifest = serde_json::from_slice(&fs::read(root.join("cases.json"))?)?;
    let case = manifest
        .cases
        .iter()
        .find(|case| case.id == id)
        .ok_or_else(|| format!("Missing case {id}"))?;
    println!("Given the authored project for {id}");
    let bytes = fs::read(root.join(&case.file))?;
    let envelope: Value = serde_json::from_str(
        &decode_model_document(&bytes, "json").map_err(std::io::Error::other)?,
    )?;
    let project: Project = serde_json::from_value(envelope["model"].clone())?;
    if group != "todolist" && project.diagrams.len() != 1 {
        return Err("Isolated diagram cases must contain exactly one diagram".into());
    }
    if group == "todolist" {
        if project.diagrams.len() < 2 || case.diagrams.len() != project.diagrams.len() {
            return Err(
                "TodoList flow must validate every diagram of a multi-diagram project".into(),
            );
        }
        let valid = manifest
            .cases
            .iter()
            .find(|case| case.expected == "valid")
            .ok_or("missing valid TodoList")?;
        let invalid = manifest
            .cases
            .iter()
            .find(|case| case.expected == "invalid")
            .ok_or("missing invalid TodoList")?;
        let valid: Value = serde_json::from_slice(&fs::read(root.join(&valid.file))?)?;
        let invalid: Value = serde_json::from_slice(&fs::read(root.join(&invalid.file))?)?;
        let mut delta = Vec::new();
        differences(&valid, &invalid, "", &mut delta);
        if delta.len() != 1
            || !delta
                .iter()
                .all(|path| path.starts_with("/elements/") || path.starts_with("/relations/"))
        {
            return Err(format!(
                "TodoList must differ by exactly one semantic field, got {delta:?}"
            )
            .into());
        }
        println!("And the TodoList pair differs in exactly one semantic field: {delta:?}");
    }
    println!("When I import and validate through the public pure API");
    let original = validate(case, &envelope)?;
    let mut failures = Vec::new();
    if let Err(error) = check_outcome(case, &original) {
        failures.push(format!("initial validation: {error}"));
    }
    let original_diagrams = diagram_responses(case, &envelope, &mut failures)?;
    let directory = std::env::temp_dir().join(format!("bellatrix-bdd-{}-{id}", std::process::id()));
    fs::create_dir(&directory)?;
    let saved = SavedFlow(directory);
    for (format, name, extension) in [
        (Format::Json, "json", "json"),
        (Format::Protobuf, "protobuf", "blx"),
    ] {
        println!("And I persist, close and reopen the {name} project");
        let destination = saved.0.join(format!("project.{extension}"));
        fs::write(&destination, encode(&project, format)?)?;
        let reopened: Value = serde_json::from_str(
            &decode_model_document(&fs::read(destination)?, name).map_err(std::io::Error::other)?,
        )?;
        if envelope != reopened {
            return Err(format!("{id}: {name} round-trip changed the project").into());
        }
        let response = validate(case, &reopened)?;
        if let Err(error) = check_outcome(case, &response) {
            failures.push(format!("{name} reopened validation: {error}"));
        }
        if response != original
            || diagram_responses(case, &reopened, &mut failures)? != original_diagrams
        {
            return Err(format!("{id}: {name} round-trip changed diagnostics").into());
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "{id}: lifecycle completed but expectations failed:\n{}",
            failures.join("\n")
        )
        .into());
    }
    println!(
        "Then {id} preserves its specified verdict, exact diagnostics and project after both round-trips (implemented rules only)"
    );
    Ok(())
}

macro_rules! flows {
    ($group:literal; $($name:ident => $id:literal),+ $(,)?) => { $(
        #[test]
        fn $name() -> FlowResult<()> { run_flow($group, $id) }
    )+ };
}
flows!("structural";
    class_valid => "class-valid", class_invalid => "class-invalid",
    object_valid => "object-valid", object_invalid => "object-invalid",
    package_valid => "package-valid", package_invalid => "package-invalid",
    component_valid => "component-valid", component_invalid => "component-invalid",
    composite_structure_valid => "composite_structure-valid", composite_structure_invalid => "composite_structure-invalid",
    deployment_valid => "deployment-valid", deployment_invalid => "deployment-invalid",
    profile_valid => "profile-valid", profile_invalid => "profile-invalid",
);
flows!("behavioral";
    use_case_valid => "use_case-valid", use_case_invalid => "use_case-invalid",
    activity_valid => "activity-valid", activity_invalid => "activity-invalid",
    state_machine_valid => "state_machine-valid", state_machine_invalid => "state_machine-invalid",
);
flows!("interaction";
    sequence_valid => "sequence-valid", sequence_invalid => "sequence-invalid",
    communication_valid => "communication-valid", communication_invalid => "communication-invalid",
    interaction_overview_valid => "interaction_overview-valid", interaction_overview_invalid => "interaction_overview-invalid",
    timing_valid => "timing-valid", timing_invalid => "timing-invalid",
);
flows!("todolist"; todolist_valid => "todolist-valid", todolist_invalid => "todolist-invalid");
