use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_contracts::protocol::Command;
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

fn element_mut<'a>(model: &'a mut Value, id: &str) -> Result<&'a mut Value> {
    model["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or_else(|| format!("missing element {id}").into())
}

const RULE: &str = "UML_PORT_DEFAULT_VALUE";

fn document(type_id: &str, default: bool) -> Result<Value> {
    let mut project = json!({
        "id":"project", "name":"Port default", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"owner", "kind":"uml.class", "name":"Owner"},
            {"id":"class", "kind":"uml.class", "name":"Implementation"},
            {"id":"interface", "kind":"uml.interface", "name":"Contract"},
            {"id":"port", "kind":"uml.port", "name":"Endpoint", "properties":{
                "aggregation":"composite", "type":{"$ref":type_id}, "defaultValue":null
            }}
        ],
        "relations":[
            {"id":"owns-owner", "kind":"owns", "from":"root", "to":"owner"},
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-interface", "kind":"owns", "from":"root", "to":"interface"},
            {"id":"owns-port", "kind":"owns", "from":"owner", "to":"port"}
        ]
    });
    if default {
        project["elements"].as_array_mut().ok_or("elements")?.push(json!({
            "id":"value", "kind":"uml.literal_integer", "name":"", "properties":{"value":0, "visibility":null}
        }));
        project["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({
                "id":"owns-value", "kind":"owns", "from":"port", "to":"value"
            }));
        author(&mut project, "defaultValue", json!({"$ref":"value"}))?;
    }
    import_document(&project)
}

fn author(project: &mut Value, key: &str, value: Value) -> Result {
    element_mut(project, "port")?["properties"][key] = value;
    Ok(())
}

fn diagnostics(project: &Value) -> Result<Vec<Value>> {
    let input = json!({"api_version":1, "model":project}).to_string();
    let response: Value = serde_json::from_str(&validate_uml_model(&input)?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics absent")?
        .clone())
}

fn rule_diagnostic(issue: &Value) -> bool {
    issue["subject_id"] == "port"
        && issue["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("Port-default_value:"))
}

#[test]
fn given_interface_typed_port_then_only_nonempty_resolved_default_is_forbidden() -> Result {
    for project in [document("interface", false)?, document("class", true)?] {
        let issues = diagnostics(&project)?;
        assert!(!issues.iter().any(rule_diagnostic), "{issues:#?}");
        assert!(
            !issues.iter().any(|issue| issue["severity"] == "error"),
            "{issues:#?}"
        );
    }
    let invalid = document("interface", true)?;
    let issues = diagnostics(&invalid)?;
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue["code"] == RULE && issue["severity"] == "error")
            .count(),
        1,
        "{issues:#?}"
    );
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        assert_eq!(reopened, invalid);
        assert_eq!(diagnostics(&reopened)?, issues);
    }
    let mut attribute = invalid.clone();
    element_mut(&mut attribute, "port")?["kind"] = json!("uml.attribute");
    assert!(!diagnostics(&attribute)?.iter().any(rule_diagnostic));
    Ok(())
}

#[test]
fn when_type_changes_to_interface_then_default_conflict_rolls_back_and_can_be_repaired() -> Result {
    let project = document("class", true)?;
    let before = encode(&project, "json")?;
    let change = Command::SetProperty {
        element_id: "port".into(),
        path: "properties.type".into(),
        value: json!({"$ref":"interface"}),
    };
    let result = apply(&project, &change, 1);
    assert!(
        matches!(result, Err(ref error) if error.contains(RULE)),
        "{result:?}"
    );
    assert_eq!(encode(&project, "json")?, before);
    let invalid = document("interface", true)?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "port".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"class"}),
        },
        1,
    )?;
    assert!(!diagnostics(&repaired)?.iter().any(rule_diagnostic));
    Ok(())
}

#[test]
fn legacy_default_editor_rejects_reference_objects_without_partial_changes() -> Result {
    let mut project = document("interface", true)?;
    author(&mut project, "defaultValue", Value::Null)?;
    author(&mut project, "lowerValue", json!({"$ref":"value"}))?;
    assert!(
        !diagnostics(&project)?
            .iter()
            .any(|issue| issue["severity"] == "error")
    );
    let before = encode(&project, "json")?;
    let result = apply(
        &project,
        &Command::SetProperty {
            element_id: "port".into(),
            path: "properties.defaultValue".into(),
            value: json!({"$ref":"value"}),
        },
        1,
    );
    assert!(
        matches!(result, Err(ref error) if error == "expected string"),
        "{result:?}"
    );
    assert_eq!(encode(&project, "json")?, before);
    Ok(())
}
