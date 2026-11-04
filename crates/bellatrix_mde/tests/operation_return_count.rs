use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_protocol::Command;
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_OPERATION_AT_MOST_ONE_RETURN";
const INVALID: &str = "MDE_OPERATION_AT_MOST_ONE_RETURN_INVALID";
const UNKNOWN: &str = "MDE_OPERATION_AT_MOST_ONE_RETURN_UNVERIFIED";

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
        false,
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json")
}

fn document(rooted: bool) -> Result<Value> {
    let mut value = json!({"id":"project","name":"Return count","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"operation","kind":"uml.operation","name":"Operation"},
        {"id":"first","kind":"uml.parameter","name":"First","properties":{"type":{"$ref":"class"},"direction":"return"}},
        {"id":"second","kind":"uml.parameter","name":"Second","properties":{"type":{"$ref":"class"}}}
    ],"relations":[]});
    for (owner, child) in [
        ("root", "class"),
        ("class", "operation"),
        ("operation", "first"),
        ("operation", "second"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
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
fn issues(model: &Value) -> Result<Vec<Value>> {
    let result: Value = serde_json::from_str(&validate_uml_model(&envelope(model))?)?;
    Ok(result["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "operation"
                && [ERROR, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| issue["code"] == *code)
        })
        .cloned()
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
fn only_return_direction_counts_and_missing_direction_defaults_to_in() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        assert!(issues(&model)?.is_empty());
        for direction in ["in", "out", "inout"] {
            author(&mut model, "second", "direction", json!(direction))?;
            assert!(issues(&model)?.is_empty());
        }
        author(&mut model, "second", "direction", json!("return"))?;
        expect(&model, ERROR)?;
        for format in ["json", "protobuf"] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
        author(&mut model, "first", "direction", json!("in"))?;
        assert!(issues(&model)?.is_empty());
        author(&mut model, "second", "direction", json!("in"))?;
        assert!(issues(&model)?.is_empty());
    }
    Ok(())
}

#[test]
fn second_return_edit_is_rejected_atomically_and_repair_succeeds() -> Result {
    let model = document(true)?;
    let before = encode(&model, "json")?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "second".into(),
            path: "properties.direction".into(),
            value: json!("return"),
        },
        1,
    )
    .expect_err("only one return");
    assert!(
        error.contains("at_most_one_return") || error.contains(ERROR),
        "{error}"
    );
    assert_eq!(encode(&model, "json")?, before);
    let mut invalid = model;
    author(&mut invalid, "second", "direction", json!("return"))?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "second".into(),
            path: "properties.direction".into(),
            value: json!("out"),
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
