use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "ParameterSet-same_parameterized_entity:";
const ERROR: &str = "UML_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY";
const INVALID: &str = "MDE_PARAMETER_SET_SAME_PARAMETERIZED_ENTITY_INVALID";
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
fn given_missing_direction_then_normative_in_default_is_used_not_unknown() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "first")
        .ok_or("first")?
        .properties
        .remove("direction");
    assert!(issues(&project)?.is_empty());
    author(&mut project, "second", "direction", json!("out"))?;
    expect(&project, ERROR)?;
    Ok(())
}

#[test]
fn given_different_direction_or_owner_then_local_disagreement_is_reported() -> Result {
    for direction in ["out", "inout", "return"] {
        let mut project = document(true)?;
        author(&mut project, "second", "direction", json!(direction))?;
        expect(&project, ERROR)?;
    }
    for id in ["set", "second"] {
        let mut project = document(true)?;
        reparent(&mut project, id, "other")?;
        author(&mut project, "set", "owner", json!({"$ref":"operation"}))?;
        author(
            &mut project,
            "second",
            "operation",
            json!({"$ref":"operation"}),
        )?;
        expect(&project, ERROR)?;
    }
    let mut project = document(true)?;
    author(&mut project, "set", "parameter", json!([{"$ref":"second"}]))?;
    reparent(&mut project, "second", "other")?;
    expect(&project, ERROR)?;
    Ok(())
}

#[test]
fn given_external_member_then_known_owner_or_direction_disagreement_still_decides_false() -> Result
{
    for reverse in [true, false] {
        let mut project = document(true)?;
        let mut parameters = vec![
            json!({"href":"external.uml#parameter"}),
            json!({"$ref":"first"}),
            json!({"$ref":"second"}),
        ];
        if reverse {
            parameters.reverse();
        }
        author(&mut project, "set", "parameter", json!(parameters))?;
        expect(&project, UNKNOWN)?;
        author(&mut project, "second", "direction", json!("out"))?;
        expect(&project, ERROR)?;
        author(&mut project, "second", "direction", json!("in"))?;
        reparent(&mut project, "first", "other")?;
        expect(&project, ERROR)?;
        author(&mut project, "second", "direction", json!(42))?;
        expect(&project, INVALID)?;
    }
    Ok(())
}

#[test]
fn given_invalid_references_or_direction_then_invalid_dominates_false_and_unknown() -> Result {
    for value in [
        Value::Null,
        json!(42),
        json!([]),
        json!([{"$ref":"first"},{"$ref":"first"}]),
        json!([{"href":"external.uml#p"},{"$ref":"missing"}]),
        json!([{"href":"external.uml#p"},{"$ref":"class"}]),
        json!([{"href":""}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "set", "parameter", value)?;
        expect(&project, INVALID)?;
    }
    for value in [
        Value::Null,
        json!(42),
        json!("invalid"),
        json!({"href":"external.uml#direction"}),
    ] {
        let mut project = document(true)?;
        author(
            &mut project,
            "set",
            "parameter",
            json!([{"href":"external.uml#p"},{"$ref":"first"},{"$ref":"second"}]),
        )?;
        reparent(&mut project, "second", "other")?;
        author(&mut project, "first", "direction", value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}

#[test]
fn given_unmaterialized_members_or_unknown_metaclass_then_inverse_and_ownership_do_not_invent_membership()
-> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "set")
        .ok_or("set")?
        .properties
        .remove("parameter");
    author(
        &mut project,
        "first",
        "parameterSet",
        json!([{"$ref":"set"}]),
    )?;
    expect(&project, UNKNOWN)?;
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "first")
        .ok_or("first")?
        .kind = "uml.unknown_parameter".into();
    expect(&project, UNKNOWN)?;
    Ok(())
}

#[test]
fn given_ambiguous_ownership_then_kernel_invalid_and_api_graph_error_precede_unknown() -> Result {
    for id in ["set", "second"] {
        let mut project = document(true)?;
        author(
            &mut project,
            "set",
            "parameter",
            json!([{"href":"external.uml#p"},{"$ref":"first"},{"$ref":"second"}]),
        )?;
        project.relations.push(serde_json::from_value(
            json!({"id":"second-owner","kind":"owns","from":"other","to":id}),
        )?);
        let kernel = bellatrix_mde::uml::root::Model::from_project(&project).validate();
        assert!(
            kernel
                .iter()
                .any(|issue| issue.subject_id == "set" && issue.code == INVALID),
            "{kernel:#?}"
        );
        let response: Value = serde_json::from_str(&validate_uml_model(
            &json!({"api_version":1,"model":project}).to_string(),
        )?)?;
        assert!(
            response["diagnostics"]
                .as_array()
                .ok_or("diagnostics")?
                .iter()
                .any(|issue| issue["code"] == "GRAPH_INTEGRITY" && issue["severity"] == "error")
        );
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

#[test]
fn given_unknown_member_metaclass_then_known_owner_disagreement_still_dominates_unknown() -> Result
{
    for reverse in [true, false] {
        let mut project = document(true)?;
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "first")
            .ok_or("first")?
            .kind = "uml.unknown_parameter".into();
        if reverse {
            author(
                &mut project,
                "set",
                "parameter",
                json!([{"$ref":"second"},{"$ref":"first"}]),
            )?;
        }
        expect(&project, UNKNOWN)?;
        reparent(&mut project, "first", "other")?;
        expect(&project, ERROR)?;
        author(&mut project, "second", "direction", json!(42))?;
        expect(&project, INVALID)?;
    }
    Ok(())
}

#[test]
fn given_unknown_member_metaclass_then_ambiguous_local_owner_is_still_invalid() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "first")
        .ok_or("first")?
        .kind = "uml.unknown_parameter".into();
    project.relations.push(serde_json::from_value(
        json!({"id":"second-owner","kind":"owns","from":"other","to":"first"}),
    )?);
    let kernel = bellatrix_mde::uml::root::Model::from_project(&project).validate();
    assert!(
        kernel
            .iter()
            .any(|issue| issue.subject_id == "set" && issue.code == INVALID),
        "{kernel:#?}"
    );
    assert!(
        !kernel
            .iter()
            .any(|issue| issue.subject_id == "set" && issue.code == UNKNOWN),
        "{kernel:#?}"
    );
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    assert!(
        response["diagnostics"]
            .as_array()
            .ok_or("diagnostics")?
            .iter()
            .any(|issue| issue["code"] == "GRAPH_INTEGRITY" && issue["severity"] == "error")
    );
    Ok(())
}
