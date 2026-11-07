use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document,
};
use bellatrix_mde::contracts::protocol::{Command, ModelingElementKind, ModelingRelationKind};
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
#[derive(Clone, Deserialize)]
struct Case {
    id: String,
    file: String,
    expected: String,
    diagram_kind: Option<String>,
    diagram_id: Option<String>,
    #[serde(default)]
    diagrams: Vec<Diagram>,
}
#[derive(Clone, Deserialize)]
struct Diagram {
    diagram_id: String,
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

fn apply_command(envelope: &Value, command: Command, revision: u64) -> FlowResult<Value> {
    let document = prepare_model_document(&envelope.to_string()).map_err(std::io::Error::other)?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
    )
    .map_err(|error| format!("public command {command:?}: {error}"))?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")
        .map_err(std::io::Error::other)?;
    Ok(serde_json::from_str(
        &decode_model_document(&bytes, "json").map_err(std::io::Error::other)?,
    )?)
}

fn save_authored_state(
    case: &Case,
    envelope: &Value,
    directory: &Path,
    stage: &str,
) -> FlowResult<Vec<Value>> {
    let mut reopened_models = Vec::new();
    for (format, extension) in [("json", "json"), ("protobuf", "blx")] {
        let destination = directory.join(format!("{stage}.{extension}"));
        fs::write(
            &destination,
            encode_model_document(&envelope.to_string(), format).map_err(std::io::Error::other)?,
        )?;
        let reopened: Value = serde_json::from_str(
            &decode_model_document(&fs::read(&destination)?, format)
                .map_err(std::io::Error::other)?,
        )?;
        if reopened != *envelope {
            return Err(format!("{}: {stage}/{format} changed the document", case.id).into());
        }
        reopened_models.push(reopened);
    }
    Ok(reopened_models)
}

fn added_identity(before: &Value, after: &Value, collection: &str) -> FlowResult<String> {
    let identities = |envelope: &Value| -> FlowResult<BTreeSet<String>> {
        envelope["model"][collection]
            .as_array()
            .ok_or("missing model collection")?
            .iter()
            .map(|record| {
                record["id"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "missing record identity".into())
            })
            .collect()
    };
    let before = identities(before)?;
    let after = identities(after)?;
    let added: Vec<_> = after.difference(&before).collect();
    match added.as_slice() {
        [identifier] => Ok(identifier.to_string()),
        _ => Err(format!("expected exactly one new {collection} identity, got {added:?}").into()),
    }
}

fn structural_authoring_flow(case: &Case, original: &Value, directory: &Path) -> FlowResult<()> {
    let property = |element: &str, key: &str, value| Command::SetProperty {
        element_id: element.into(),
        path: format!("properties.{key}"),
        value,
    };
    let owner = |element: &str, target: &str| Command::SetOwner {
        element_id: element.into(),
        owner_id: Some(target.into()),
    };
    let mut baseline = original.clone();
    let mut cleanup = Vec::new();
    let (mutation, mut repairs) = match case.diagram_kind.as_deref() {
        Some("class") => (
            property("submit-request", "direction", json!("return")),
            vec![property("submit-request", "direction", json!("in"))],
        ),
        Some("object") => (
            property("first-number", "definingFeature", json!({"$ref":"name"})),
            vec![property(
                "first-number",
                "definingFeature",
                json!({"$ref":"number"}),
            )],
        ),
        Some("package") => (
            property("order", "visibility", json!("protected")),
            vec![property("order", "visibility", json!("public"))],
        ),
        Some("component") => {
            baseline = apply_command(
                original,
                Command::CreateModelElement {
                    owner_id: Some("root".into()),
                    kind: ModelingElementKind::Class,
                    name: Some("BddContainer".into()),
                },
                1,
            )?;
            let container = added_identity(original, &baseline, "elements")?;
            cleanup.push(Command::DeleteElement {
                element_id: container.clone(),
            });

            (
                owner("checkout", &container),
                vec![owner("checkout", "root")],
            )
        }
        Some("composite_structure") => (
            property("storage-end", "role", json!({"$ref":"foreign-part"})),
            vec![property(
                "storage-end",
                "role",
                json!({"$ref":"storage-part"}),
            )],
        ),
        Some("deployment") => (
            owner("deployment", "host"),
            vec![owner("deployment", "runtime")],
        ),
        Some("profile") => (
            Command::CreateRelation {
                diagram_id: case.diagram_id.clone().ok_or("missing diagram identity")?,
                owner_id: Some("local".into()),
                kind: ModelingRelationKind::Generalization,
                from: "local".into(),
                to: "metaclass".into(),
            },
            Vec::new(),
        ),
        _ => return Err("structural authoring requires a structural case".into()),
    };
    println!(
        "Given the valid {} authoring baseline, including any public preparation commands",
        case.id
    );
    save_authored_state(case, &baseline, directory, "authored-baseline")?;
    println!("When a public command authors the specified semantic violation");
    let invalid = apply_command(&baseline, mutation, 2)?;
    if invalid == baseline {
        return Err("invalidating command did not change the model".into());
    }
    if case.diagram_kind.as_deref() == Some("profile") {
        repairs.push(Command::DeleteElement {
            element_id: added_identity(&baseline, &invalid, "relations")?,
        });
    }
    repairs.extend(cleanup);
    let reopened = save_authored_state(case, &invalid, directory, "authored-invalid")?;
    for (index, reopened) in reopened.iter().enumerate() {
        println!(
            "When I repair reopened invalid {} through public commands",
            case.id
        );
        let mut repaired = reopened.clone();
        for (revision, command) in repairs.iter().enumerate() {
            repaired = apply_command(&repaired, command.clone(), 3 + revision as u64)?;
        }
        for key in original["model"]
            .as_object()
            .ok_or("missing original model")?
            .keys()
        {
            if key != "diagrams" && repaired["model"][key] != original["model"][key] {
                return Err(
                    format!("{}: repair changed original model field {key}", case.id).into(),
                );
            }
        }
        save_authored_state(
            case,
            &repaired,
            directory,
            &format!("authored-repaired-{index}"),
        )?;
    }
    println!(
        "Then {} repairs both reopened formats and returns to the original semantic model",
        case.id
    );
    Ok(())
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
    if examples.len() != 40
        || examples.iter().collect::<BTreeSet<_>>().len() != 40
        || !examples.contains(&id)
    {
        return Err(format!(
            "Gherkin inventory must contain exactly 40 unique scenarios including {id}"
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
    if envelope["api_version"] != 1 {
        return Err(format!("{id}: unexpected document API version").into());
    }
    let diagrams = envelope
        .get("model")
        .and_then(|model| model.get("diagrams"))
        .and_then(Value::as_array)
        .ok_or("decoded envelope missing diagrams")?;
    if group != "todolist" && diagrams.len() != 1 {
        return Err("Isolated diagram cases must contain exactly one diagram".into());
    }
    if group == "todolist" {
        let actual_ids = diagrams
            .iter()
            .map(|diagram| {
                diagram
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or("diagram id missing")
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let expected_ids: BTreeSet<_> = case
            .diagrams
            .iter()
            .map(|diagram| diagram.diagram_id.as_str())
            .collect();
        if diagrams.len() < 2
            || expected_ids.len() != case.diagrams.len()
            || actual_ids.len() != diagrams.len()
            || expected_ids != actual_ids
        {
            return Err(
                "TodoList flow must include every diagram of a multi-diagram project".into(),
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
    println!("When I import through the public pure API");
    decode_model_document(&serde_json::to_vec(&envelope["model"])?, "json")
        .map_err(std::io::Error::other)?;
    let directory = std::env::temp_dir().join(format!("bellatrix-bdd-{}-{id}", std::process::id()));
    fs::create_dir(&directory)?;
    let saved = SavedFlow(directory);
    for (name, extension) in [("json", "json"), ("protobuf", "blx")] {
        println!("And I persist, close and reopen the {name} project");
        let destination = saved.0.join(format!("project.{extension}"));
        fs::write(
            &destination,
            encode_model_document(&envelope.to_string(), name).map_err(std::io::Error::other)?,
        )?;
        let reopened: Value = serde_json::from_str(
            &decode_model_document(&fs::read(destination)?, name).map_err(std::io::Error::other)?,
        )?;
        if envelope != reopened {
            return Err(format!("{id}: {name} round-trip changed the project").into());
        }
    }
    if group == "structural" && case.expected == "valid" {
        structural_authoring_flow(case, &envelope, &saved.0)?;
    }
    println!("Then {id} preserves its project after both round-trips");
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

flows!("behavioral";
    activity_initial_incoming_invalid => "activity-initial-incoming-invalid",
    activity_initial_control_invalid => "activity-initial-control-invalid",
    activity_final_outgoing_invalid => "activity-final-outgoing-invalid",
    activity_fork_incoming_invalid => "activity-fork-incoming-invalid",
    activity_join_outgoing_invalid => "activity-join-outgoing-invalid",
    activity_merge_outgoing_invalid => "activity-merge-outgoing-invalid",
    activity_decision_incoming_invalid => "activity-decision-incoming-invalid",
);

flows!("behavioral"; activity_presentation_roundtrip_valid => "activity-presentation-roundtrip-valid");

flows!("behavioral";
 activity_malformed_incidence_invalid => "activity-malformed-incidence-invalid",
 activity_external_control_valid => "activity-external-control-valid",
);
