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

fn document(owner_kind: &str, child_kind: &str, role: Option<&str>, rooted: bool) -> Result<Value> {
    let mut value = json!({
        "id":"project", "name":"Components", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"owner", "kind":owner_kind, "name":"Owner"},
            {"id":"component", "kind":"uml.component", "name":"Component"},
            {"id":"child", "kind":child_kind, "name":"Child"}
        ],
        "relations":[
            {"id":"root-owner", "kind":"owns", "from":"root", "to":"owner"},
            {"id":"owner-component", "kind":"owns", "from":"owner", "to":"component"},
            {"id":"component-child", "kind":"owns", "from":"component", "to":"child"}
        ]
    });
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    let mut project = value;
    if let Some(role) = role {
        author(&mut project, "component", role, json!([{"$ref":"child"}]))?;
    }
    Ok(decode(&serde_json::to_vec(&project)?, "json")?)
}

fn author(project: &mut Value, id: &str, key: &str, value: Value) -> Result {
    let element = project["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or("element")?;
    element["properties"][key] = value;
    Ok(())
}

fn diagnostics(project: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(&envelope(project))?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}

fn component_issues(issues: &[Value]) -> Vec<&Value> {
    issues
        .iter()
        .filter(|issue| {
            issue["message"]
                .as_str()
                .is_some_and(|message| message.starts_with("Component-no_"))
        })
        .collect()
}

#[test]
fn given_component_then_only_nested_classifier_role_is_forbidden_in_rooted_and_rootless_models()
-> Result {
    for rooted in [true, false] {
        for role in ["nestedClassifier", "packagedElement"] {
            let project = document("uml.package", "uml.class", Some(role), rooted)?;
            let issues = diagnostics(&project)?;
            let actual = component_issues(&issues);
            if role == "nestedClassifier" {
                assert_eq!(actual.len(), 1, "{issues:#?}");
                assert_eq!(actual[0]["code"], "UML_COMPONENT_NO_NESTED_CLASSIFIERS");
                assert_eq!(actual[0]["subject_id"], "component");
                assert_eq!(actual[0]["severity"], "error");
            } else {
                assert!(actual.is_empty(), "{issues:#?}");
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
fn when_packaging_component_is_nested_in_class_then_command_rolls_back_and_invalid_draft_can_be_repaired()
-> Result {
    let mut valid = document("uml.package", "uml.package", Some("packagedElement"), true)?;
    valid["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .push(json!({"id":"class", "kind":"uml.class", "name":"Class"}));
    valid["relations"]
        .as_array_mut()
        .ok_or("relations")?
        .push(json!({"id":"root-class", "kind":"owns", "from":"root", "to":"class"}));
    let valid = decode(&serde_json::to_vec(&valid)?, "json")?;
    let before = encode(&valid, "json")?;
    let error = apply(
        &valid,
        &Command::SetOwner {
            element_id: "component".into(),
            owner_id: Some("class".into()),
        },
        1,
    )
    .expect_err("a nested Component cannot retain packaged elements");
    assert!(error.contains("Component-no_packaged_elements"), "{error}");
    assert_eq!(encode(&valid, "json")?, before);
    let invalid = document("uml.class", "uml.package", Some("packagedElement"), true)?;
    let repaired = apply(
        &invalid,
        &Command::SetOwner {
            element_id: "component".into(),
            owner_id: Some("root".into()),
        },
        2,
    )?;
    let issues = diagnostics(&repaired)?;
    assert!(component_issues(&issues).is_empty(), "{issues:#?}");
    Ok(())
}
