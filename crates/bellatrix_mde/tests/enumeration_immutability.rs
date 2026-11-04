use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "UML_ENUMERATION_IMMUTABLE";

fn document(read_only: Option<Value>, rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project", "name":"Enumeration immutability", "root_model":"root",
    "elements":[
        {"id":"root", "kind":"uml.model", "name":"Root"},
        {"id":"enumeration", "kind":"uml.enumeration", "name":"Color"},
        {"id":"attribute", "kind":"uml.attribute", "name":"rgb"}
    ],
    "relations":[
        {"id":"root-enumeration", "kind":"owns", "from":"root", "to":"enumeration"},
        {"id":"enumeration-attribute", "kind":"owns", "from":"enumeration", "to":"attribute"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    let mut project: Project = serde_json::from_value(value)?;
    if let Some(value) = read_only {
        author(&mut project, "attribute", "isReadOnly", value)?;
    }
    Ok(project)
}

fn author(project: &mut Project, id: &str, property: &str, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(property.into(), value);
    Ok(())
}

fn diagnostics(project: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1, "model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}

fn relevant(issue: &Value) -> bool {
    issue["message"]
        .as_str()
        .is_some_and(|message| message.starts_with("Enumeration-immutable:"))
}

#[test]
fn given_owned_attributes_then_only_true_is_immutable_in_rooted_and_rootless_models() -> Result {
    for rooted in [true, false] {
        for value in [None, Some(json!(false)), Some(json!(true))] {
            let project = document(value.clone(), rooted)?;
            let issues = diagnostics(&project)?;
            let immutable = value == Some(json!(true));
            assert_eq!(
                issues.iter().filter(|issue| issue["code"] == RULE).count(),
                usize::from(!immutable),
                "{issues:#?}"
            );
            if immutable {
                assert!(!issues.iter().any(relevant), "{issues:#?}");
            }
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&project, format)?, format)?;
                assert_eq!(diagnostics(&reopened)?, issues);
            }
        }
    }
    Ok(())
}

#[test]
fn when_attribute_becomes_mutable_then_edit_rolls_back_and_invalid_draft_can_be_repaired() -> Result
{
    let valid = document(Some(json!(true)), true)?;
    let before = encode(&valid, Format::Json)?;
    let error = apply(
        &valid,
        &Command::SetProperty {
            element_id: "attribute".into(),
            path: "properties.isReadOnly".into(),
            value: json!(false),
        },
        1,
    )
    .expect_err("Enumeration attribute must remain immutable");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&valid, Format::Json)?, before);
    let invalid = document(None, true)?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "attribute".into(),
            path: "properties.isReadOnly".into(),
            value: json!(true),
        },
        2,
    )?;
    assert!(!diagnostics(&repaired)?.iter().any(relevant));
    Ok(())
}
