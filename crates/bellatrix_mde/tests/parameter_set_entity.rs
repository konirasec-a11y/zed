use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_contracts::protocol::Command;
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> Result<Value> {
    let response = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&response)?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> Result<Vec<u8>> {
    Ok(encode_model_document(&envelope(model), format)?)
}

fn import_document(model: &Value) -> Result<Value> {
    decode(&serde_json::to_vec(model)?, "json")
}

fn apply(model: &Value, command: &Command, revision: u64) -> std::result::Result<Value, String> {
    let document = prepare_model_document(&envelope(model))?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json").map_err(|error| error.to_string())
}

fn element_mut<'a>(model: &'a mut Value, id: &str) -> Result<&'a mut Value> {
    model["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or_else(|| format!("missing element {id}").into())
}

const RULE: &str = "ParameterSet-same_parameterized_entity:";
const ERROR: &str = "UML_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY";
const UNKNOWN: &str = "MDE_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY_UNVERIFIED";

fn document(rooted: bool) -> Result<Value> {
    let mut value = json!({"id":"project","name":"Parameter sets","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"class","kind":"uml.class","name":"Class"},
            {"id":"operation","kind":"uml.operation","name":"Operation"},
            {"id":"other","kind":"uml.operation","name":"Other"},
            {"id":"set","kind":"uml.parameter_set","name":"Inputs","properties":{"parameter":[{"$ref":"first"},{"$ref":"second"}]}},
            {"id":"first","kind":"uml.parameter","name":"First","properties":{"type":{"$ref":"class"},"direction":"in"}},
            {"id":"second","kind":"uml.parameter","name":"Second","properties":{"type":{"$ref":"class"},"direction":"in"}}
        ],"relations":[]});
    for (owner, child) in [
        ("root", "class"),
        ("class", "operation"),
        ("class", "other"),
        ("operation", "set"),
        ("operation", "first"),
        ("operation", "second"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    import_document(&value)
}
fn author(project: &mut Value, id: &str, key: &str, value: Value) -> Result {
    element_mut(project, id)?["properties"][key] = value;
    Ok(())
}
fn reparent(project: &mut Value, id: &str, owner: &str) -> Result {
    project["relations"]
        .as_array_mut()
        .ok_or("relations")?
        .iter_mut()
        .find(|relation| relation["id"] == format!("owns-{id}"))
        .ok_or("owner")?["from"] = json!(owner);
    Ok(())
}
fn issues(project: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "set"
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with(RULE))
        })
        .cloned()
        .collect())
}
fn expect(project: &Value, code: &str) -> Result {
    let issues = issues(project)?;
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0]["code"], code, "{issues:#?}");
    assert_eq!(
        issues[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}

#[test]
fn given_same_canonical_owner_and_direction_then_parameter_set_entity_rule_is_satisfied() -> Result
{
    for rooted in [true, false] {
        for kind in ["uml.operation", "uml.opaque_behavior"] {
            let mut project = document(rooted)?;
            element_mut(&mut project, "operation")?["kind"] = json!(kind);
            for direction in ["in", "out", "inout", "return"] {
                author(&mut project, "first", "direction", json!(direction))?;
                author(&mut project, "second", "direction", json!(direction))?;
                assert!(issues(&project)?.is_empty(), "{kind}/{direction}");
                for format in ["json", "protobuf"] {
                    assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
                }
            }
        }
    }
    Ok(())
}

#[test]
fn when_parameter_moves_to_another_entity_then_reparent_is_rejected_atomically_and_repair_succeeds()
-> Result {
    let project = document(true)?;
    let before = encode(&project, "json")?;
    let error = apply(
        &project,
        &Command::SetOwner {
            element_id: "second".into(),
            owner_id: Some("other".into()),
        },
        1,
    )
    .expect_err("the ParameterSet cannot span distinct parameterized entities");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&project, "json")?, before);
    let mut invalid = project;
    reparent(&mut invalid, "second", "other")?;
    expect(&invalid, ERROR)?;
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetOwner {
                element_id: "second".into(),
                owner_id: Some("operation".into()),
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}
