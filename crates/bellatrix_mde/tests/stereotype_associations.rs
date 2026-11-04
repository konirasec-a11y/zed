use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STEREOTYPE_BINARY_ASSOCIATIONS";
const UNKNOWN: &str = "MDE_STEREOTYPE_BINARY_ASSOCIATIONS_UNVERIFIED";

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
fn author(model: &mut Project, id: &str, key: &str, value: Value) {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .expect("element")
        .properties
        .insert(key.into(), value);
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
fn nary(model: &mut Project) {
    author(
        model,
        "association",
        "memberEnd",
        json!([{"$ref":"a"},{"$ref":"b"},{"$ref":"c"}]),
    );
}

#[test]
fn given_direct_attribute_in_association_then_only_binary_cardinality_passes_after_roundtrip()
-> Result {
    let mut model = document()?;
    assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    nary(&mut model);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert!(has(&reopened, "stereotype", ERROR)? && !has(&reopened, "stereotype", UNKNOWN)?);
    }
    Ok(())
}

#[test]
fn when_nary_end_moves_to_stereotype_then_existing_association_gate_also_rejects_atomically()
-> Result {
    let mut model = document()?;
    nary(&mut model);
    for id in ["a", "c"] {
        model
            .relations
            .iter_mut()
            .find(|relation| relation.id == format!("owns-{id}"))
            .ok_or("ownership")?
            .from = "association".into();
    }
    author(
        &mut model,
        "association",
        "ownedEnd",
        json!([{"$ref":"a"},{"$ref":"b"},{"$ref":"c"}]),
    );
    let before = model.clone();
    assert!(
        apply(
            &model,
            &Command::SetOwner {
                element_id: "a".into(),
                owner_id: Some("stereotype".into())
            },
            1
        )
        .is_err()
    );
    assert_eq!(model, before);
    Ok(())
}
