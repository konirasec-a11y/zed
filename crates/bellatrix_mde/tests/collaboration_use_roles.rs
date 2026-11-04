use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_COLLABORATION_USE_EVERY_ROLE";
const UNKNOWN: &str = "MDE_COLLABORATION_USE_EVERY_ROLE_UNVERIFIED";
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
#[test]
fn given_all_roles_bound_then_explicit_and_inferred_binding_pass_roundtrip() -> Result {
    for explicit in [false, true] {
        let model = document(explicit)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}

#[test]
fn given_only_binding_deleted_then_unbound_role_rejection_is_atomic() -> Result {
    let model = document(false)?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::DeleteElement {
            element_id: "binding".into(),
        },
        1,
    )
    .expect_err("required role binding");
    assert!(error.to_string().contains(ERROR), "{error}");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}

#[test]
fn given_two_roles_then_one_or_two_bindings_can_cover_both() -> Result {
    for split in [false, true] {
        let mut model = document(false)?;
        author(
            &mut model,
            "collaboration",
            "collaborationRole",
            json!([{"$ref":"role1"},{"$ref":"role2"}]),
        )?;
        if split {
            model.relations.push(serde_json::from_value(json!({"id":"binding2","kind":"uml.dependency","from":"client","to":"role2","properties":{"visibility":null}}))?);
            model.relations.push(serde_json::from_value(
                json!({"id":"owns-binding2","kind":"owns","from":"usage","to":"binding2"}),
            )?);
        } else {
            author(
                &mut model,
                "binding",
                "supplier",
                json!([{"$ref":"role1"},{"$ref":"role2"}]),
            )?;
        }
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}

#[test]
fn given_dependency_object_then_explicit_suppliers_provide_canonical_witnesses() -> Result {
    for explicit in [false, true] {
        let mut model = document(explicit)?;
        model.relations.retain(|relation| relation.id != "binding");
        model.elements.push(serde_json::from_value(json!({"id":"binding","kind":"uml.dependency","name":"Binding","properties":{"visibility":null,"client":[{"$ref":"client"}],"supplier":[{"$ref":"role1"}]}}))?);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}
