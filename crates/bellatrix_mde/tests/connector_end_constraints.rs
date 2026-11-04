use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn document() -> Result<Project> {
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
    for (owner, child) in [("root","owner"),("root","parent"),("root","foreign"),("root","type"),("parent","r1"),("owner","r2"),("foreign","outside"),("type","port"),("owner","connector"),("connector","e1"),("connector","e2")] {
        value["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, property: &str, value: Value) {
    model.elements.iter_mut().find(|e| e.id == id).expect("element").properties.insert(property.into(), value);
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    let value: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(value["diagnostics"].as_array().ok_or("diagnostics")?.clone())
}
fn has(model: &Project, rule: &str, severity: &str) -> Result<bool> {
    Ok(issues(model)?.iter().any(|diagnostic| diagnostic["subject_id"] == "e1"
        && diagnostic["severity"] == severity
        && diagnostic["message"].as_str().is_some_and(|message| message.starts_with(&format!("ConnectorEnd-{rule}:")))))
}
fn attach_to_part(model: &mut Project) {
    author(model, "e1", "role", json!({"$ref":"port"}));
    author(model, "e1", "partWithPort", json!({"$ref":"r2"}));
}

#[test]
fn given_port_on_typed_part_when_validated_then_namespace_membership_is_required() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        for rule in ["role_and_part_with_port", "part_with_port_empty", "self_part_with_port"] {
            assert!(!has(&reopened, rule, "error")? && !has(&reopened, rule, "warning")?, "{:?}", issues(&reopened)?);
        }
    }
    author(&mut model, "r2", "type", json!({"$ref":"foreign"}));
    assert!(has(&model, "role_and_part_with_port", "error")?);
    author(&mut model, "r2", "type", json!({"href":"external.uml#Type"}));
    assert!(has(&model, "role_and_part_with_port", "warning")?);
    author(&mut model, "e1", "role", json!({"$ref":"outside"}));
    assert!(has(&model, "role_and_part_with_port", "error")?, "non-Port is decisive beside unknown Type");
    Ok(())
}

#[test]
fn given_valid_part_port_when_port_moves_outside_type_then_command_rejects_atomically() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    let before = model.clone();
    let error = apply(&model, &Command::SetOwner { element_id:"port".into(), owner_id:Some("foreign".into()) }, 1)
        .expect_err("port must remain a member of the part Type");
    assert!(error.contains("ConnectorEnd-role_and_part_with_port"), "{error}");
    assert_eq!(model, before);
    Ok(())
}
