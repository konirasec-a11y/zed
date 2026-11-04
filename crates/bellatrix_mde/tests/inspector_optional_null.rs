use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, prepare_model_document,
};
use bellatrix_mde_protocol::Command;
use serde_json::{Value, json};

fn document() -> Value {
    let project = json!({
        "id":"optional-reference", "name":"Optional reference", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Owner", "properties":{"isActive":true}},
            {"id":"operation", "kind":"uml.operation", "name":"accept",
             "properties":{"bodyCondition":"unfinished condition", "raisedException":"unfinished list"}},
            {"id":"reception", "kind":"uml.reception", "name":"Receive",
             "properties":{"signal":"unfinished reference"}},
            {"id":"readonly", "kind":"uml.operation", "name":"readOnly",
             "properties":{"ownedParameter":[]}}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"},
            {"id":"owns-reception", "kind":"owns", "from":"class", "to":"reception"},
            {"id":"owns-readonly", "kind":"owns", "from":"class", "to":"readonly"}
        ]
    });
    let decoded = decode_model_document(
        &serde_json::to_vec(&project).expect("fixture bytes"),
        "json",
    )
    .expect("normalize authored drafts");
    let mut envelope: Value = serde_json::from_str(&decoded).expect("model envelope");
    envelope["model"].take()
}

fn set_property(model: &Value, owner: &str, path: &str, value: Value) -> Result<Value, String> {
    let parts = prepare_model_document(&json!({"api_version": 1, "model": model}).to_string())?;
    let section = apply_model_command(
        &parts.id,
        &parts.name,
        &parts.section_json,
        1,
        Command::SetProperty {
            element_id: owner.into(),
            path: path.into(),
            value,
        },
    )?;
    let bytes = encode_model_section(&parts.id, &parts.name, &section.to_string(), "json")?;
    let decoded = decode_model_document(&bytes, "json")?;
    let mut envelope: Value = serde_json::from_str(&decoded).map_err(|error| error.to_string())?;
    Ok(envelope["model"].take())
}

#[test]
fn optional_reference_can_be_cleared_without_unlocking_other_fields() {
    let mut section = document();
    section = set_property(
        &section,
        "operation",
        "properties.bodyCondition",
        Value::Null,
    )
    .expect("clear an optional scalar reference represented by an editable legacy string");
    assert_eq!(
        section["elements"]
            .as_array()
            .expect("canonical elements")
            .iter()
            .find(|element| element["id"] == "operation")
            .expect("operation")["properties"]["bodyCondition"],
        Value::Null
    );
    for (owner, property, expected) in [
        ("reception", "properties.signal", "expected string"),
        ("operation", "properties.raisedException", "expected string"),
        ("readonly", "properties.ownedParameter", "read-only"),
        ("operation", "properties.type", "read-only"),
    ] {
        let before = section.clone();
        let error = set_property(&section, owner, property, Value::Null)
            .expect_err("required, collection and read-only fields stay protected");
        assert!(error.contains(expected), "{owner}::{property}: {error}");
        assert_eq!(section, before);
    }
}
