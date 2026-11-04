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

fn document(properties: Value) -> Result<Value> {
    import_document(&json!({
        "id":"project", "name":"Scalars",
        "elements":[{"id":"parameter", "kind":"uml.parameter", "name":"P", "properties":properties}],
        "relations":[]
    }))
}
fn issues(project: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|i| {
            i["subject_id"] == "parameter"
                && i["message"].as_str().is_some_and(|m| {
                    ["in_and_out", "not_exception", "stream_and_exception"]
                        .iter()
                        .any(|r| m.starts_with(&format!("Parameter::{r}:")))
                })
        })
        .cloned()
        .collect())
}
fn has(found: &[Value], rule: &str, code: &str) -> bool {
    found.iter().any(|i| {
        i["code"] == code
            && i["message"]
                .as_str()
                .is_some_and(|m| m.starts_with(&format!("Parameter::{rule}:")))
    })
}

#[test]
fn given_three_simultaneous_violations_then_all_three_are_reported_and_roundtrip() -> Result {
    let project =
        document(json!({"direction":"in","effect":"create","isException":true,"isStream":true}))?;
    for format in ["json", "protobuf"] {
        let found = issues(&decode(&encode(&project, format)?, format)?)?;
        assert_eq!(found.len(), 3, "{found:?}");
        for rule in ["in_and_out", "not_exception", "stream_and_exception"] {
            assert!(has(&found, rule, "UML_PARAMETER_CONSTRAINT"));
        }
    }
    Ok(())
}

#[test]
fn when_scalar_edit_introduces_violation_then_command_rolls_back_and_repair_succeeds() -> Result {
    for (initial, path, value, rule) in [
        (
            json!({"effect":null}),
            "effect",
            json!("create"),
            "in_and_out",
        ),
        (
            json!({"isException":false}),
            "isException",
            json!(true),
            "not_exception",
        ),
        (
            json!({"direction":"out","isException":true,"isStream":false}),
            "isStream",
            json!(true),
            "stream_and_exception",
        ),
    ] {
        let project = document(initial)?;
        let before = encode(&project, "json")?;
        let error = apply(
            &project,
            &Command::SetProperty {
                element_id: "parameter".into(),
                path: format!("properties.{path}"),
                value: value.clone(),
            },
            1,
        )
        .expect_err("violation");
        assert!(error.contains(rule), "{error}");
        assert_eq!(before, encode(&project, "json")?);
        let mut invalid = project.clone();
        element_mut(&mut invalid, "parameter")?["properties"][path] = value;
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&invalid, format)?, format)?;
            let repaired = apply(
                &reopened,
                &Command::SetProperty {
                    element_id: "parameter".into(),
                    path: format!("properties.{path}"),
                    value: project["elements"][0]["properties"][path].clone(),
                },
                2,
            )?;
            assert!(issues(&repaired)?.is_empty());
        }
    }
    Ok(())
}
