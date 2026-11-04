use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_protocol::Command;
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "UML_ENUMERATION_IMMUTABLE";

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> std::result::Result<Value, String> {
    let response = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> std::result::Result<Vec<u8>, String> {
    encode_model_document(&envelope(model), format)
}

fn apply(model: &Value, command: &Command, revision: u64) -> std::result::Result<Value, String> {
    let document = prepare_model_document(&envelope(model))?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
        false,
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json")
}

fn document(read_only: Option<Value>, rooted: bool) -> Result<Value> {
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
    let mut project = value;
    if let Some(value) = read_only {
        author(&mut project, "attribute", "isReadOnly", value)?;
    }
    Ok(decode(&serde_json::to_vec(&project)?, "json")?)
}

fn author(project: &mut Value, id: &str, property: &str, value: Value) -> Result {
    let element = project["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or("element")?;
    element["properties"][property] = value;
    Ok(())
}

fn diagnostics(project: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(&envelope(project))?)?;
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
            for format in ["json", "protobuf"] {
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
    let before = encode(&valid, "json")?;
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
    assert_eq!(encode(&valid, "json")?, before);
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
