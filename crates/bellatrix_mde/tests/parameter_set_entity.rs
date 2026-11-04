use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "ParameterSet-same_parameterized_entity:";
const ERROR: &str = "UML_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY";
const UNKNOWN: &str = "MDE_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
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
    Ok(serde_json::from_value(value)?)
}
fn author(project: &mut Project, id: &str, key: &str, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn reparent(project: &mut Project, id: &str, owner: &str) -> Result {
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == format!("owns-{id}"))
        .ok_or("owner")?
        .from = owner.into();
    Ok(())
}
fn issues(project: &Project) -> Result<Vec<Value>> {
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
fn expect(project: &Project, code: &str) -> Result {
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
            project
                .elements
                .iter_mut()
                .find(|element| element.id == "operation")
                .ok_or("operation")?
                .kind = kind.into();
            for direction in ["in", "out", "inout", "return"] {
                author(&mut project, "first", "direction", json!(direction))?;
                author(&mut project, "second", "direction", json!(direction))?;
                assert!(issues(&project)?.is_empty(), "{kind}/{direction}");
                for format in [Format::Json, Format::Protobuf] {
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
    let before = encode(&project, Format::Json)?;
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
    assert_eq!(encode(&project, Format::Json)?, before);
    let mut invalid = project;
    reparent(&mut invalid, "second", "other")?;
    expect(&invalid, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
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
