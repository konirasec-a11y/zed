use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "ParameterSet-input:";
const ERROR: &str = "UML_PARAMETER_SET_INPUT";
const UNKNOWN: &str = "MDE_PARAMETER_SET_INPUT_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Parameter sets","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"class","kind":"uml.class","name":"Class"},
            {"id":"operation","kind":"uml.operation","name":"Operation"},
            {"id":"other","kind":"uml.operation","name":"Other"},
            {"id":"set","kind":"uml.parameter_set","name":"Inputs","properties":{"parameter":[{"$ref":"first"}]}},
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
fn unbound_input_and_output_parameters_must_stream_in_rooted_and_rootless_models() -> Result {
    for rooted in [true, false] {
        for direction in ["in", "out"] {
            let mut project = document(rooted)?;
            for id in ["first", "second"] {
                author(&mut project, id, "direction", json!(direction))?;
            }
            expect(&project, ERROR)?;
            author(&mut project, "second", "isStream", json!(true))?;
            assert!(issues(&project)?.is_empty());
            for format in [Format::Json, Format::Protobuf] {
                assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
            }
        }
    }
    Ok(())
}

fn additional_set(project: &mut Project, id: &str, parameters: Value) -> Result {
    project.elements.push(serde_json::from_value(
        json!({"id":id,"kind":"uml.parameter_set","name":id,"properties":{"parameter":parameters}}),
    )?);
    project.relations.push(serde_json::from_value(
        json!({"id":format!("owns-{id}"),"kind":"owns","from":"operation","to":id}),
    )?);
    Ok(())
}
#[test]
fn unrelated_uncertain_set_does_not_mask_resolved_inverse_contradiction() -> Result {
    for reverse in [false, true] {
        let mut project = document(true)?;
        additional_set(&mut project, "other-set", json!([{"$ref":"second"}]))?;
        additional_set(
            &mut project,
            "external-set",
            json!([{"href":"external.uml#p"}]),
        )?;
        author(
            &mut project,
            "first",
            "parameterSet",
            json!([{"$ref":"set"},{"$ref":"other-set"}]),
        )?;
        if reverse {
            project.elements.reverse();
        }
        expect(&project, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}

#[test]
fn making_an_unbound_parameter_nonstreaming_is_rejected_atomically() -> Result {
    let mut project = document(true)?;
    author(&mut project, "second", "isStream", json!(true))?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "second".into(),
            path: "properties.isStream".into(),
            value: json!(false),
        },
        1,
    )
    .expect_err("nonstreaming unbound input");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&project, Format::Json)?, before);
    Ok(())
}

#[test]
fn inverse_with_unknown_local_type_cannot_omit_a_known_forward_membership() -> Result {
    let mut project = document(true)?;
    author(&mut project, "second", "isStream", json!(true))?;
    project.elements.push(serde_json::from_value(
        json!({"id":"unknown-local","kind":"uml.unknown_set","name":"Unknown set"}),
    )?);
    author(
        &mut project,
        "first",
        "parameterSet",
        json!([{"$ref":"unknown-local"}]),
    )?;
    expect(&project, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
    }
    author(
        &mut project,
        "first",
        "parameterSet",
        json!([{"$ref":"unknown-local"},{"$ref":"set"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "first",
        "parameterSet",
        json!([{"href":"external.uml#set"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
