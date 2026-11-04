use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, project_model_section, validate_uml_model,
};
use bellatrix_mde_protocol::Command;
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> Result<Value> {
    let response = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&response)?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> Result<Vec<u8>> {
    Ok(encode_model_document(&envelope(model), format)?)
}

fn import_document(model: &Value) -> Result<Value> {
    decode(&serde_json::to_vec(model)?, "json")
}

fn apply(model: &Value, command: &Command, revision: u64) -> std::result::Result<Value, String> {
    let document = prepare_model_document(&envelope(model))?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json").map_err(|error| error.to_string())
}

const RULE: &str = "CommunicationPath-association_ends:";
const ERROR: &str = "UML_COMMUNICATION_PATH_ASSOCIATION_ENDS";
const UNKNOWN: &str = "MDE_COMMUNICATION_PATH_ASSOCIATION_ENDS_UNVERIFIED";

fn document(rooted: bool) -> Result<Value> {
    let mut value = json!({"id":"project","name":"Communication path","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"left","kind":"uml.node","name":"Left"},
            {"id":"right","kind":"uml.device","name":"Right"},
            {"id":"class","kind":"uml.class","name":"Class"},
            {"id":"path","kind":"uml.communication_path","name":"Path","properties":{"memberEnd":[{"$ref":"end-left"},{"$ref":"end-right"}]}},
            {"id":"end-left","kind":"uml.attribute","name":"left","properties":{"type":{"$ref":"left"}}},
            {"id":"end-right","kind":"uml.attribute","name":"right","properties":{"type":{"$ref":"right"}}}
        ],"relations":[]});
    for (owner, child) in [
        ("root", "left"),
        ("root", "right"),
        ("root", "class"),
        ("root", "path"),
        ("path", "end-left"),
        ("path", "end-right"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    import_document(&value)
}
fn author(project: &mut Value, id: &str, key: &str, value: Value) -> Result {
    project["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or("element")?["properties"][key] = value;
    Ok(())
}

fn inspected_property(model: &Value, id: &str, path: &str) -> Result<Value> {
    let document = prepare_model_document(&envelope(model))?;
    let projection = project_model_section(&document.id, &document.name, &document.section_json)?;
    Ok(projection
        .inspect_element(id)
        .ok_or("inspector")?
        .properties
        .into_iter()
        .find(|property| property.path == path)
        .ok_or("inspector property")?
        .value)
}
fn diagnostics(project: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}
fn relevant(issue: &&Value) -> bool {
    issue["subject_id"] == "path"
        && issue["message"]
            .as_str()
            .is_some_and(|message| message.starts_with(RULE))
}
fn expect(project: &Value, code: &str) -> Result {
    let issues = diagnostics(project)?;
    let specific = issues.iter().filter(relevant).collect::<Vec<_>>();
    assert_eq!(specific.len(), 1, "{issues:#?}");
    assert_eq!(specific[0]["code"], code, "{issues:#?}");
    assert_eq!(
        specific[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}

#[test]
fn given_canonical_communication_path_then_deployment_target_end_types_are_valid() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.node", "uml.device", "uml.execution_environment"] {
            let mut project = document(rooted)?;
            project["elements"]
                .as_array_mut()
                .ok_or("elements")?
                .iter_mut()
                .find(|element| element["id"] == "left")
                .ok_or("left")?["kind"] = json!(kind);
            let issues = diagnostics(&project)?;
            assert_eq!(issues.iter().filter(relevant).count(), 0, "{issues:#?}");
            assert!(
                !issues.iter().any(|issue| issue["subject_id"] == "path"
                    && (issue["severity"] == "error"
                        || issue["code"] == "MDE_ASSOCIATION_UNSUPPORTED")),
                "{issues:#?}"
            );
            // The legacy authoring adapter still lacks this metaclass; this
            // kernel-only implementation does not claim to add a visual tool.
            assert!(
                issues.iter().any(|issue| issue["subject_id"] == "path"
                    && issue["code"] == "MDE_UML_METACLASS_UNSUPPORTED"),
                "{issues:#?}"
            );
            assert_eq!(
                inspected_property(&project, "path", "uml.endType")?
                    .as_array()
                    .ok_or("association end Types")?
                    .len(),
                2
            );
            for format in ["json", "protobuf"] {
                assert_eq!(
                    diagnostics(&decode(&encode(&project, format)?, format)?)?,
                    issues
                );
            }
        }
    }
    Ok(())
}

#[test]
fn when_end_type_becomes_class_then_edit_rejects_atomically_and_repair_succeeds() -> Result {
    let project = document(true)?;
    let before = encode(&project, "json")?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "end-left".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"class"}),
        },
        1,
    )
    .expect_err("Class is not a DeploymentTarget");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&project, "json")?, before);
    let mut invalid = project;
    author(&mut invalid, "end-left", "type", json!({"$ref":"class"}))?;
    expect(&invalid, ERROR)?;
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "end-left".into(),
                path: "properties.type".into(),
                value: json!({"$ref":"left"}),
            },
            2,
        )?;
        assert_eq!(diagnostics(&repaired)?.iter().filter(relevant).count(), 0);
    }
    Ok(())
}
