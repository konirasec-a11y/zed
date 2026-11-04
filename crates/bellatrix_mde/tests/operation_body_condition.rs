use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_protocol::Command;
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_OPERATION_ONLY_BODY_FOR_QUERY";
const INVALID: &str = "MDE_OPERATION_ONLY_BODY_FOR_QUERY_INVALID";
const UNKNOWN: &str = "MDE_OPERATION_ONLY_BODY_FOR_QUERY_UNVERIFIED";

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> std::result::Result<Value, String> {
    let response = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> std::result::Result<Vec<u8>, String> {
    encode_model_document(&envelope(model), format)
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
    decode(&bytes, "json")
}

fn document(rooted: bool) -> Result<Value> {
    let mut value = json!({"id":"project","name":"Operation condition","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"operation","kind":"uml.operation","name":"Operation","properties":{"bodyCondition":{"$ref":"condition"},"isQuery":true}},
        {"id":"condition","kind":"uml.constraint","name":"Condition"}
    ],"relations":[
        {"id":"owns-class","kind":"owns","from":"root","to":"class"},
        {"id":"owns-operation","kind":"owns","from":"class","to":"operation"},
        {"id":"owns-condition","kind":"owns","from":"operation","to":"condition"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("model")?.remove("root_model");
    }
    Ok(decode(&serde_json::to_vec(&value)?, "json")?)
}
fn author(model: &mut Value, id: &str, key: &str, value: Value) -> Result {
    let element = model["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or("element")?;
    element["properties"][key] = value;
    Ok(())
}
fn diagnostics(model: &Value) -> Result<Vec<Value>> {
    let result: Value = serde_json::from_str(&validate_uml_model(&envelope(model))?)?;
    Ok(result["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| issue["subject_id"] == "operation")
        .cloned()
        .collect())
}
fn issues(model: &Value) -> Result<Vec<Value>> {
    Ok(diagnostics(model)?
        .into_iter()
        .filter(|issue| {
            [ERROR, INVALID, UNKNOWN]
                .iter()
                .any(|code| issue["code"] == *code)
        })
        .collect())
}
fn expect(model: &Value, code: &str) -> Result {
    let found = issues(model)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    assert_eq!(
        found[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
#[test]
fn a_body_condition_requires_query_true_with_false_as_the_default() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        assert!(issues(&model)?.is_empty());
        author(&mut model, "operation", "isQuery", json!(false))?;
        expect(&model, ERROR)?;
        for format in ["json", "protobuf"] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
        let operation = model["elements"]
            .as_array_mut()
            .ok_or("elements")?
            .iter_mut()
            .find(|element| element["id"] == "operation")
            .ok_or("operation")?;
        operation["properties"]
            .as_object_mut()
            .ok_or("operation properties")?
            .remove("isQuery");
        expect(&model, ERROR)?;
    }
    Ok(())
}

#[test]
fn nonquery_edit_is_rejected_atomically_and_imported_violation_can_be_repaired() -> Result {
    let model = document(true)?;
    let before = encode(&model, "json")?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "operation".into(),
            path: "properties.isQuery".into(),
            value: json!(false),
        },
        1,
    )
    .expect_err("bodyCondition requires query");
    assert!(
        error.contains("UML_OPERATION_BODY_CONDITION") || error.contains(ERROR),
        "{error}"
    );
    assert_eq!(encode(&model, "json")?, before);
    let mut invalid = model;
    author(&mut invalid, "operation", "isQuery", json!(false))?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "operation".into(),
            path: "properties.isQuery".into(),
            value: json!(true),
        },
        2,
    )?;
    assert!(issues(&repaired)?.is_empty());
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&repaired, format)?, format)?;
        assert_eq!(reopened, repaired);
        assert!(issues(&reopened)?.is_empty());
    }
    Ok(())
}
