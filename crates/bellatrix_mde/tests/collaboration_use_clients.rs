use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_COLLABORATION_USE_CLIENT_ELEMENTS";
const UNKNOWN: &str = "MDE_COLLABORATION_USE_CLIENT_ELEMENTS_UNVERIFIED";
fn document(explicit: bool) -> Result<Project> {
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Collaboration roles","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":"uml.class","name":"Owner"},
            {"id":"collaboration","kind":"uml.collaboration","name":"Collaboration","properties":{"collaborationRole":[{"$ref":"role1"}]}},
            {"id":"usage","kind":"uml.collaboration_use","name":"Use","properties":{"type":{"$ref":"collaboration"}}},
            {"id":"client","kind":"uml.property","name":"Client"},
            {"id":"role1","kind":"uml.property","name":"First role"},
            {"id":"role2","kind":"uml.property","name":"Second role"}
        ],"relations":[
            {"id":"owns-owner","kind":"owns","from":"root","to":"owner"},
            {"id":"owns-collaboration","kind":"owns","from":"root","to":"collaboration"},
            {"id":"owns-usage","kind":"owns","from":"owner","to":"usage"},
            {"id":"owns-client","kind":"owns","from":"owner","to":"client"},
            {"id":"owns-role1","kind":"owns","from":"collaboration","to":"role1"},
            {"id":"owns-role2","kind":"owns","from":"collaboration","to":"role2"},
            {"id":"binding","kind":"uml.dependency","from":"client","to":"role1","properties":{"visibility":null}},
            {"id":"owns-binding","kind":"owns","from":"usage","to":"binding"}
        ]}),
    )?;
    if explicit {
        author(
            &mut model,
            "usage",
            "roleBinding",
            json!([{"$ref":"binding"}]),
        )?;
    }
    Ok(model)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) -> Result {
    if let Some(element) = model.elements.iter_mut().find(|element| element.id == id) {
        element.properties.insert(key.into(), value);
    } else {
        model
            .relations
            .iter_mut()
            .find(|relation| relation.id == id)
            .ok_or("subject")?
            .properties
            .insert(key.into(), value);
    }
    Ok(())
}
fn has(model: &Project, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .any(|issue| issue["subject_id"] == "usage" && issue["code"] == code))
}
fn paired() -> Result<Project> {
    let mut model = document(false)?;
    model.elements.push(serde_json::from_value(
        json!({"id":"client2","kind":"uml.property","name":"Other client"}),
    )?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-client2","kind":"owns","from":"owner","to":"client2"}),
    )?);
    author(
        &mut model,
        "binding",
        "client",
        json!([{"$ref":"client"},{"$ref":"client2"}]),
    )?;
    author(
        &mut model,
        "binding",
        "supplier",
        json!([{"$ref":"role1"},{"$ref":"role2"}]),
    )?;
    author(
        &mut model,
        "collaboration",
        "collaborationRole",
        json!([{"$ref":"role1"},{"$ref":"role2"}]),
    )?;
    Ok(model)
}
fn pass(model: &Project) -> Result {
    assert!(!has(model, ERROR)?);
    assert!(!has(model, UNKNOWN)?);
    Ok(())
}
#[test]
fn matching_collections_and_single_endpoint_roundtrip() -> Result {
    for model in [document(false)?, document(true)?, paired()?] {
        pass(&model)?;
        for format in [Format::Json, Format::Protobuf] {
            pass(&decode(&encode(&model, format)?, format)?)?;
        }
    }
    Ok(())
}

#[test]
fn moving_one_client_to_different_context_is_atomic() -> Result {
    let model = paired()?;
    let before = serde_json::to_value(&model)?;
    let error = apply(
        &model,
        &Command::SetOwner {
            element_id: "client2".into(),
            owner_id: Some("collaboration".into()),
        },
        1,
    )
    .expect_err("must reject");
    assert!(
        error.contains("CollaborationUse-client_elements:"),
        "{error}"
    );
    assert_eq!(before, serde_json::to_value(&model)?);
    Ok(())
}
