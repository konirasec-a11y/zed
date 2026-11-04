use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const MULTIPLICITY: &str = "UML_EXTENSION_END_MULTIPLICITY";
const UNKNOWN: &str = "MDE_EXTENSION_END_MULTIPLICITY_UNVERIFIED";

fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Extension bounds","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"profile","kind":"uml.profile","name":"Profile"},
            {"id":"class","kind":"uml.class","name":"Metaclass"},
            {"id":"stereotype","kind":"uml.stereotype","name":"Stereotype"},
            {"id":"extension","kind":"uml.extension","name":"Extension","properties":{"memberEnd":[{"$ref":"base"},{"$ref":"end"}],"ownedEnd":{"$ref":"end"}}},
            {"id":"base","kind":"uml.attribute","name":"base_Class","properties":{"type":{"$ref":"class"}}},
            {"id":"end","kind":"uml.extension_end","name":"extension_Stereotype","properties":{"type":{"$ref":"stereotype"},"aggregation":"composite"}}
        ],"relations":[]});
    for (owner, child) in [
        ("root", "profile"),
        ("root", "class"),
        ("profile", "stereotype"),
        ("profile", "extension"),
        ("stereotype", "base"),
        ("extension", "end"),
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
fn has(model: &Project, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .any(|issue| issue["subject_id"] == "end" && issue["code"] == code))
}

#[test]
fn given_explicit_ranges_then_only_zero_to_one_or_one_are_allowed_and_roundtrip_is_stable() -> Result
{
    for (range, valid) in [
        ("0..1", true),
        ("1", true),
        ("0..0", false),
        ("2..2", false),
        ("0..2", false),
        ("0..*", false),
        ("-1..1", false),
        ("2..1", false),
    ] {
        let mut model = document()?;
        author(&mut model, "end", "multiplicity", json!(range));
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert_eq!(has(&reopened, MULTIPLICITY)?, !valid, "{range}");
            assert!(!has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}

#[test]
fn given_supported_association_container_when_extension_bound_changes_then_edit_is_atomic() -> Result
{
    let mut model = document()?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "extension")
        .ok_or("association")?
        .kind = "uml.nary_association".into();
    author(&mut model, "extension", "ownedEnd", json!([{"$ref":"end"}]));
    author(&mut model, "end", "multiplicity", json!("0..1"));
    let before = model.clone();
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "end".into(),
            path: "properties.multiplicity".into(),
            value: json!("0..2"),
        },
        1,
    )
    .expect_err("ExtensionEnd upper must be one");
    assert!(error.contains("ExtensionEnd-multiplicity"), "{error}");
    assert_eq!(model, before);
    author(&mut model, "end", "multiplicity", json!("0..2"));
    let repaired = apply(
        &model,
        &Command::SetProperty {
            element_id: "end".into(),
            path: "properties.multiplicity".into(),
            value: json!("1"),
        },
        2,
    )?;
    assert!(!has(&repaired, MULTIPLICITY)?);
    Ok(())
}

#[test]
fn given_full_extension_container_then_existing_unsupported_gate_is_not_claimed_as_implemented()
-> Result {
    let mut model = document()?;
    author(&mut model, "end", "multiplicity", json!("0..1"));
    let before = model.clone();
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "end".into(),
            path: "properties.multiplicity".into(),
            value: json!("1"),
        },
        1,
    )
    .expect_err("full Extension rules remain unsupported");
    assert!(error.contains("MDE_ASSOCIATION_UNSUPPORTED"), "{error}");
    assert_eq!(model, before);
    Ok(())
}
