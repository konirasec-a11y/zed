use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
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

const ERROR: &str = "UML_CONNECTOR_TYPES";
fn document() -> Result<Value> {
    let mut value = json!({"id":"model","name":"Connectors","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":"uml.class","name":"Owner"},
            {"id":"left","kind":"uml.class","name":"Left"},
            {"id":"right","kind":"uml.class","name":"Right"},
            {"id":"leaf","kind":"uml.class","name":"Leaf"},
            {"id":"middle","kind":"uml.class","name":"Middle"},
            {"id":"association","kind":"uml.nary_association","name":"Association","properties":{"memberEnd":[{"$ref":"a1"},{"$ref":"a2"}]}},
            {"id":"a1","kind":"uml.attribute","name":"leftEnd","properties":{"type":{"$ref":"left"}}},
            {"id":"a2","kind":"uml.attribute","name":"rightEnd","properties":{"type":{"$ref":"right"}}},
            {"id":"r1","kind":"uml.attribute","name":"leftRole","properties":{"type":{"$ref":"leaf"}}},
            {"id":"r2","kind":"uml.attribute","name":"rightRole","properties":{"type":{"$ref":"right"}}},
            {"id":"connector","kind":"uml.connector","name":"Connection","properties":{"type":{"$ref":"association"},"end":[{"$ref":"e1"},{"$ref":"e2"}]}},
            {"id":"e1","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r1"}}},
            {"id":"e2","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r2"}}}
        ],"relations":[{"id":"specialization","kind":"uml.generalization","from":"leaf","to":"middle"},
            {"id":"middle-specialization","kind":"uml.generalization","from":"middle","to":"left"}]
    });
    for (owner, child) in [
        ("root", "owner"),
        ("root", "left"),
        ("root", "right"),
        ("root", "leaf"),
        ("root", "middle"),
        ("root", "association"),
        ("association", "a1"),
        ("association", "a2"),
        ("owner", "r1"),
        ("owner", "r2"),
        ("owner", "connector"),
        ("connector", "e1"),
        ("connector", "e2"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    import_document(&value)
}
fn author(model: &mut Value, id: &str, key: &str, value: Value) {
    model["elements"]
        .as_array_mut()
        .expect("elements")
        .iter_mut()
        .find(|element| element["id"] == id)
        .expect("element")["properties"][key] = value;
}
fn diagnostics(model: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}
fn has(issues: &[Value], code: &str) -> bool {
    issues
        .iter()
        .any(|d| d["subject_id"] == "connector" && d["code"] == code)
}

#[test]
fn connector_type_flow_checks_position_conformance_rollback_and_reopening() -> Result {
    let model = document()?;
    assert!(
        !diagnostics(&model)?
            .iter()
            .any(|d| d["severity"] == "error"),
        "{:?}",
        diagnostics(&model)?
    );
    let before = model.clone();
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "r1".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"right"}),
        },
        1,
    )
    .expect_err("wrong role Type must reject");
    assert!(error.contains("Connector-types"), "{error}");
    assert_eq!(model, before);
    let mut invalid = model;
    author(
        &mut invalid,
        "connector",
        "end",
        json!([{"$ref":"e2"},{"$ref":"e1"}]),
    );
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        assert!(has(&diagnostics(&reopened)?, ERROR));
        let mut repaired = reopened;
        author(
            &mut repaired,
            "connector",
            "end",
            json!([{"$ref":"e1"},{"$ref":"e2"}]),
        );
        assert!(!has(&diagnostics(&repaired)?, ERROR));
    }
    Ok(())
}
