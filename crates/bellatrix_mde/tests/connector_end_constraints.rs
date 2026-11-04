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

fn document() -> Result<Value> {
    let mut value = json!({"id":"model","name":"Roles","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":"uml.class","name":"Owner"},
            {"id":"parent","kind":"uml.class","name":"Parent"},
            {"id":"foreign","kind":"uml.class","name":"Foreign"},
            {"id":"type","kind":"uml.class","name":"Type"},
            {"id":"r1","kind":"uml.attribute","name":"inherited","properties":{"type":{"$ref":"type"},"visibility":"public"}},
            {"id":"r2","kind":"uml.attribute","name":"direct","properties":{"type":{"$ref":"type"}}},
            {"id":"outside","kind":"uml.attribute","name":"outside","properties":{"type":{"$ref":"type"}}},
            {"id":"port","kind":"uml.port","name":"Port","properties":{"type":{"$ref":"type"}}},
            {"id":"connector","kind":"uml.connector","name":"Connector","properties":{"end":[{"$ref":"e1"},{"$ref":"e2"}]}},
            {"id":"e1","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r1"}}},
            {"id":"e2","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r2"}}}
        ],
        "relations":[{"id":"general","kind":"uml.generalization","from":"owner","to":"parent"}]
    });
    for (owner, child) in [
        ("root", "owner"),
        ("root", "parent"),
        ("root", "foreign"),
        ("root", "type"),
        ("parent", "r1"),
        ("owner", "r2"),
        ("foreign", "outside"),
        ("type", "port"),
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
fn author(model: &mut Value, id: &str, property: &str, value: Value) {
    model["elements"]
        .as_array_mut()
        .expect("elements")
        .iter_mut()
        .find(|element| element["id"] == id)
        .expect("element")["properties"][property] = value;
}
fn issues(model: &Value) -> Result<Vec<Value>> {
    let value: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(value["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}
fn has(model: &Value, rule: &str, severity: &str) -> Result<bool> {
    Ok(issues(model)?.iter().any(|diagnostic| {
        diagnostic["subject_id"] == "e1"
            && diagnostic["severity"] == severity
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.starts_with(&format!("ConnectorEnd-{rule}:")))
    }))
}
fn attach_to_part(model: &mut Value) {
    author(model, "e1", "role", json!({"$ref":"port"}));
    author(model, "e1", "partWithPort", json!({"$ref":"r2"}));
}

#[test]
fn given_port_on_typed_part_when_validated_then_namespace_membership_is_required() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&model, format)?, format)?;
        for rule in [
            "role_and_part_with_port",
            "part_with_port_empty",
            "self_part_with_port",
        ] {
            assert!(
                !has(&reopened, rule, "error")? && !has(&reopened, rule, "warning")?,
                "{:?}",
                issues(&reopened)?
            );
        }
    }
    author(&mut model, "r2", "type", json!({"$ref":"foreign"}));
    assert!(has(&model, "role_and_part_with_port", "error")?);
    author(
        &mut model,
        "r2",
        "type",
        json!({"href":"external.uml#Type"}),
    );
    assert!(has(&model, "role_and_part_with_port", "warning")?);
    author(&mut model, "e1", "role", json!({"$ref":"outside"}));
    assert!(
        has(&model, "role_and_part_with_port", "error")?,
        "non-Port is decisive beside unknown Type"
    );
    Ok(())
}

#[test]
fn given_valid_part_port_when_port_moves_outside_type_then_command_rejects_atomically() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    let before = model.clone();
    let error = apply(
        &model,
        &Command::SetOwner {
            element_id: "port".into(),
            owner_id: Some("foreign".into()),
        },
        1,
    )
    .expect_err("port must remain a member of the part Type");
    assert!(
        error.contains("ConnectorEnd-role_and_part_with_port"),
        "{error}"
    );
    assert_eq!(model, before);
    Ok(())
}
