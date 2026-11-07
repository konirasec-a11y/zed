use bellatrix_mde::contracts::model::Project;
use bellatrix_mde::{
    decode_model_document, encode_model_document, encode_model_section, prepare_model_document,
};
use serde::Deserialize;
use serde_json::Value;
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
    file: Option<String>,
    pending_reason: Option<String>,
    diagram_id: Option<String>,
    #[serde(default)]
    diagrams: Vec<DiagramRequirement>,
    #[serde(default)]
    witnesses: Vec<Witness>,
}

#[derive(Deserialize)]
struct DiagramRequirement {
    diagram_id: String,
}

#[derive(Deserialize)]
struct Witness {
    pointer: String,
    value: Value,
}

struct Workspace(PathBuf);

impl Drop for Workspace {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not remove BDD workspace {}: {error}",
                self.0.display()
            );
        }
    }
}

fn read_manifest(group: &str) -> FlowResult<(PathBuf, Manifest)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/bdd")
        .join(group);
    let manifest = serde_json::from_slice(&fs::read(root.join("cases.json"))?)?;
    Ok((root, manifest))
}

fn require_inventory() -> FlowResult<()> {
    let examples: Vec<_> = include_str!("features/diagram_lifecycle.feature")
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .filter_map(|line| line.split('|').nth(1))
        .map(str::trim)
        .filter(|identifier| *identifier != "case_id")
        .collect();
    let expected: BTreeSet<_> = examples.iter().copied().collect();
    let mut actual = BTreeSet::new();
    for group in ["structural", "behavioral", "interaction", "todolist"] {
        for case in read_manifest(group)?.1.cases {
            if !actual.insert(case.id) {
                return Err("BDD manifests contain a duplicate example".into());
            }
        }
    }
    let actual: BTreeSet<_> = actual.iter().map(String::as_str).collect();
    if examples.len() != 40 || expected.len() != 40 || expected != actual {
        return Err("BDD manifests must cover exactly the 40 Gherkin examples".into());
    }
    Ok(())
}

fn require_document(case: &Case, document: &Project, value: &Value) -> FlowResult<()> {
    if document.schema_version != 2 || document.model.registry.entries().is_empty() {
        return Err(format!(
            "{}: expected a populated canonical schema-2 registry",
            case.id
        )
        .into());
    }
    let root = document
        .model
        .root_model
        .as_ref()
        .ok_or("missing model root")?;
    if document
        .model
        .registry
        .get_entry(root)
        .map(|entry| entry.type_name())
        != Some("Model")
    {
        return Err("the root must reference a canonical UML Model contract".into());
    }
    let actual: BTreeSet<_> = document
        .model
        .diagrams
        .iter()
        .map(|diagram| diagram.id.as_str())
        .collect();
    let expected: BTreeSet<_> = case
        .diagram_id
        .iter()
        .map(String::as_str)
        .chain(
            case.diagrams
                .iter()
                .map(|diagram| diagram.diagram_id.as_str()),
        )
        .collect();
    if actual != expected || actual.len() != document.model.diagrams.len() {
        return Err(format!(
            "{}: missing, duplicate or unexpected diagram identity",
            case.id
        )
        .into());
    }
    if case.diagram_id.is_some() {
        if document.model.diagrams.len() != 1 || !case.diagrams.is_empty() {
            return Err("isolated examples must contain exactly one diagram".into());
        }
    } else if case.diagrams.len() < 2 || expected.len() != case.diagrams.len() {
        return Err("TodoList must retain every diagram of its multi-diagram application".into());
    }
    if case.witnesses.is_empty() {
        return Err(format!("{}: scenario requires explicit authored witnesses", case.id).into());
    }
    for witness in &case.witnesses {
        if value.pointer(&witness.pointer) != Some(&witness.value) {
            return Err(format!(
                "{}: authored witness changed at {}",
                case.id, witness.pointer
            )
            .into());
        }
    }
    Ok(())
}

fn require_todolist_pair(root: &Path) -> FlowResult<()> {
    let valid: Value = serde_json::from_slice(&fs::read(root.join("valid.json"))?)?;
    let mut invalid: Value = serde_json::from_slice(&fs::read(root.join("invalid.json"))?)?;
    let pointer = "/model/registry/entries/open/data/submachine";
    if valid.pointer(pointer) != Some(&Value::Null)
        || invalid.pointer(pointer) != Some(&Value::String("lifecycle-recovery".into()))
    {
        return Err("TodoList must author the specified State.submachine change".into());
    }
    *invalid
        .pointer_mut(pointer)
        .ok_or("missing TodoList mutation field")? = Value::Null;
    if invalid != valid {
        return Err("TodoList variants must differ in exactly one semantic field".into());
    }
    Ok(())
}

fn run_flow(group: &str, identifier: &str) -> FlowResult<()> {
    require_inventory()?;
    let (root, manifest) = read_manifest(group)?;
    let case = manifest
        .cases
        .iter()
        .find(|case| case.id == identifier)
        .ok_or_else(|| format!("Missing BDD example {identifier}"))?;
    if let Some(reason) = &case.pending_reason {
        return Err(format!("{identifier}: {reason}").into());
    }
    println!("Given the canonical authored project for {identifier}");
    let file = case
        .file
        .as_ref()
        .ok_or("executable example has no canonical document")?;
    let input = fs::read(root.join(file))?;
    let expected: Project = serde_json::from_slice(&input)?;
    let authored = serde_json::to_value(&expected)?;
    require_document(case, &expected, &authored)?;
    if group == "todolist" {
        require_todolist_pair(&root)?;
    }

    println!("When I import through the public document API");
    let envelope = decode_model_document(&input, "json")?;
    let decoded: Value = serde_json::from_str(&envelope)?;
    if decoded["api_version"] != 1 || decoded["model"] != authored {
        return Err(format!("{identifier}: import changed the authored canonical document").into());
    }
    let prepared = prepare_model_document(&envelope)?;
    if prepared.id != expected.id || prepared.name != expected.name {
        return Err("document preparation changed project identity".into());
    }
    let section: bellatrix_mde::contracts::model::ModelingSection =
        serde_json::from_str(&prepared.section_json)?;
    if section != expected.model {
        return Err("document preparation changed UML contracts or presentation".into());
    }

    let workspace = std::env::temp_dir().join(format!(
        "bellatrix-bdd-{}-{identifier}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir(&workspace)?;
    let workspace = Workspace(workspace);
    for (format, extension) in [("json", "json"), ("protobuf", "blx")] {
        println!("And I save, close and reopen the complete {format} document");
        let destination = workspace.0.join(format!("project.{extension}"));
        let encoded = encode_model_document(&envelope, format)?;
        if format == "protobuf" && !encoded.starts_with(b"BLX2") {
            return Err("binary document must use the BLX2 format".into());
        }
        fs::write(&destination, encoded)?;
        let reopened = decode_model_document(&fs::read(&destination)?, format)?;
        let reopened: Value = serde_json::from_str(&reopened)?;
        if reopened != decoded {
            return Err(format!("{identifier}: {format} changed the complete envelope").into());
        }
        let actual: Project = serde_json::from_value(reopened["model"].clone())?;
        require_document(case, &actual, &reopened["model"])?;
        if actual != expected {
            return Err(format!("{identifier}: {format} changed typed UML or presentation").into());
        }
        let section_bytes =
            encode_model_section(&prepared.id, &prepared.name, &prepared.section_json, format)?;
        let section_reopened: Value =
            serde_json::from_str(&decode_model_document(&section_bytes, format)?)?;
        if section_reopened != decoded {
            return Err(
                format!("{identifier}: section persistence changed the {format} document").into(),
            );
        }
    }
    println!("Then {identifier} preserves every authored UML contract and diagram in both formats");
    Ok(())
}

macro_rules! flows {
    ($group:literal; $($(#[$attribute:meta])* $name:ident => $id:literal),+ $(,)?) => { $(
        $(#[$attribute])*
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
    #[ignore = "Pending: ruml Message has no signature field and MessageOccurrenceSpecification requires event; moving the operation to an invented event has not been shown equivalent."]
    sequence_valid => "sequence-valid",
    #[ignore = "Pending: ruml Message has no signature field and MessageOccurrenceSpecification requires event; moving the operation to an invented event has not been shown equivalent."]
    sequence_invalid => "sequence-invalid",
    #[ignore = "Pending: ruml Message has no signature field and MessageOccurrenceSpecification requires event; moving the operation to an invented event has not been shown equivalent."]
    communication_valid => "communication-valid",
    #[ignore = "Pending: ruml Message has no signature field and MessageOccurrenceSpecification requires event; moving the operation to an invented event has not been shown equivalent."]
    communication_invalid => "communication-invalid",
    interaction_overview_valid => "interaction_overview-valid", interaction_overview_invalid => "interaction_overview-invalid",
    #[ignore = "Pending: ruml Message has no signature field and MessageOccurrenceSpecification requires event; moving the operation to an invented event has not been shown equivalent."]
    timing_valid => "timing-valid",
    #[ignore = "Pending: ruml Message has no signature field and MessageOccurrenceSpecification requires event; moving the operation to an invented event has not been shown equivalent."]
    timing_invalid => "timing-invalid",
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
 #[ignore = "Pending: authored source is JSON number 42, but ruml ActivityEdge.source requires String; a string identity is not the same malformed-incidence requirement."]
    activity_malformed_incidence_invalid => "activity-malformed-incidence-invalid",
 #[ignore = "Pending: authored source is an external href object, but ruml ActivityEdge.source requires String; external-reference equivalence has not been established."]
    activity_external_control_valid => "activity-external-control-valid",
);
