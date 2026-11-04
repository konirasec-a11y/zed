use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STEREOTYPE_ASSOCIATION_END_OWNERSHIP";
const UNKNOWN: &str = "MDE_STEREOTYPE_ASSOCIATION_END_OWNERSHIP_UNVERIFIED";

fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Stereotype associations","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"profile","kind":"uml.profile","name":"Profile"},
            {"id":"stereotype","kind":"uml.stereotype","name":"Stereotype"},
            {"id":"other","kind":"uml.class","name":"Other"},
            {"id":"third","kind":"uml.class","name":"Third"},
            {"id":"association","kind":"uml.nary_association","name":"Association","properties":{"memberEnd":[{"$ref":"a"},{"$ref":"b"}],"ownedEnd":[{"$ref":"b"}]}},
            {"id":"a","kind":"uml.attribute","name":"toOther","properties":{"type":{"$ref":"other"}}},
            {"id":"b","kind":"uml.attribute","name":"toStereotype","properties":{"type":{"$ref":"stereotype"}}},
            {"id":"c","kind":"uml.attribute","name":"extra","properties":{"type":{"$ref":"stereotype"}}}
        ],"relations":[]});
    for (owner, child) in [
        ("root", "profile"),
        ("profile", "stereotype"),
        ("root", "other"),
        ("root", "third"),
        ("root", "association"),
        ("stereotype", "a"),
        ("association", "b"),
        ("third", "c"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}

fn has(model: &Project, subject: &str, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .any(|issue| issue["subject_id"] == subject && issue["code"] == code))
}

fn move_end(model: &mut Project, owner: &str) {
    model
        .relations
        .iter_mut()
        .find(|relation| relation.id == "owns-b")
        .expect("owner")
        .from = owner.into();
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "association")
        .expect("association")
        .properties
        .remove("ownedEnd");
}
#[test]
fn given_non_stereotype_type_then_opposite_requires_association_ownership_after_roundtrip() -> Result
{
    let mut model = document()?;
    assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    move_end(&mut model, "other");
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert!(has(&reopened, "stereotype", ERROR)?);
    }
    Ok(())
}

#[test]
fn given_reparent_opposite_then_command_rejects_atomically() -> Result {
    let mut model = document()?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "association")
        .expect("association")
        .properties
        .remove("ownedEnd");
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetOwner {
            element_id: "b".into(),
            owner_id: Some("other".into()),
        },
        1,
    )
    .expect_err("bad owner");
    assert!(error.to_string().contains(ERROR), "{error}");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}
