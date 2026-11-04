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

const RULE: &str = "Parameter-object_effect:";
const ERROR: &str = "UML_PARAMETER_OBJECT_EFFECT";
const UNKNOWN: &str = "MDE_PARAMETER_OBJECT_EFFECT_UNVERIFIED";
fn document(kind: &str, rooted: bool) -> Result<Value> {
    let mut value = json!({"id":"project","name":"Effects","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"type","kind":kind,"name":"Type"},
        {"id":"parameter","kind":"uml.parameter","name":"Argument","properties":{"type":{"$ref":"type"},"effect":"read"}}
    ],"relations":[{"id":"owns-type","kind":"owns","from":"root","to":"type"}]});
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    import_document(&value)
}
fn author(project: &mut Value, key: &str, value: Value) -> Result {
    element_mut(project, "parameter")?["properties"][key] = value;
    Ok(())
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
                && i["message"].as_str().is_some_and(|m| m.starts_with(RULE))
        })
        .cloned()
        .collect())
}
fn expect(project: &Value, code: &str) -> Result {
    let found = issues(project)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    assert_eq!(
        found[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
#[test]
fn given_data_type_or_subclass_then_every_defined_effect_is_forbidden() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.data_type", "uml.primitive_type", "uml.enumeration"] {
            for effect in ["read", "update", "create", "delete"] {
                let mut project = document(kind, rooted)?;
                author(&mut project, "effect", json!(effect))?;
                expect(&project, ERROR)?;
                for format in ["json", "protobuf"] {
                    expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn when_effect_is_added_to_data_type_parameter_then_command_rolls_back_and_repair_succeeds()
-> Result {
    let mut project = document("uml.data_type", true)?;
    author(&mut project, "effect", Value::Null)?;
    let before = encode(&project, "json")?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "parameter".into(),
            path: "properties.effect".into(),
            value: json!("read"),
        },
        1,
    )
    .expect_err("effect forbidden");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(before, encode(&project, "json")?);
    author(&mut project, "effect", json!("read"))?;
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&project, format)?, format)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "parameter".into(),
                path: "properties.effect".into(),
                value: Value::Null,
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}
