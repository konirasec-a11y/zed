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
#[test]
fn direction_and_stream_defaults_are_normative_and_other_directions_are_not_inputs() -> Result {
    let mut project = document(true)?;
    for element in &mut project.elements {
        element.properties.remove("direction");
    }
    expect(&project, ERROR)?;
    for direction in ["out", "inout", "return"] {
        author(&mut project, "second", "direction", json!(direction))?;
        assert!(issues(&project)?.is_empty(), "{direction}");
    }
    for direction in ["inout", "return"] {
        author(&mut project, "first", "direction", json!(direction))?;
        author(&mut project, "second", "direction", json!("in"))?;
        assert!(issues(&project)?.is_empty(), "{direction}");
    }
    Ok(())
}
#[test]
fn forward_membership_exempts_nonstreaming_parameter_without_materialized_inverse() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "set",
        "parameter",
        json!([{"$ref":"first"},{"$ref":"second"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    author(&mut project, "second", "parameterSet", json!([]))?;
    expect(&project, ERROR)?;
    author(
        &mut project,
        "second",
        "parameterSet",
        json!([{"$ref":"set"}]),
    )?;
    assert!(issues(&project)?.is_empty());
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
fn inverse_to_uncertain_set_is_not_a_proven_contradiction_or_membership() -> Result {
    let mut project = document(true)?;
    additional_set(
        &mut project,
        "external-set",
        json!([{"href":"external.uml#p"}]),
    )?;
    author(
        &mut project,
        "second",
        "parameterSet",
        json!([{"$ref":"external-set"}]),
    )?;
    expect(&project, UNKNOWN)?;
    author(&mut project, "second", "isStream", json!(true))?;
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "first",
        "parameterSet",
        json!([{"$ref":"set"},{"$ref":"external-set"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn streaming_consequent_can_settle_an_unknown_antecedent() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "set",
        "parameter",
        json!([{"href":"external.uml#p"}]),
    )?;
    expect(&project, UNKNOWN)?;
    for id in ["first", "second"] {
        author(&mut project, id, "isStream", json!(true))?;
    }
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn missing_members_remain_unknown_unless_streaming_proves_the_consequent() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "set")
        .ok_or("set")?
        .properties
        .remove("parameter");
    expect(&project, UNKNOWN)?;
    for id in ["first", "second"] {
        author(&mut project, id, "isStream", json!(true))?;
    }
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn invalid_members_and_scalars_are_not_hidden_by_membership_or_streaming() -> Result {
    for value in [
        Value::Null,
        json!(42),
        json!([]),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"class"}]),
        json!([{"$ref":"first"},{"$ref":"first"}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "set", "parameter", value)?;
        author(&mut project, "second", "isStream", json!(true))?;
        expect(&project, ERROR)?;
    }
    for (id, property) in [
        ("first", "direction"),
        ("second", "direction"),
        ("first", "isStream"),
        ("second", "isStream"),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, property, json!(42))?;
        expect(&project, ERROR)?;
    }
    Ok(())
}
#[test]
fn canonical_owner_limits_parameters_and_stored_owner_cannot_override_it() -> Result {
    let mut project = document(true)?;
    reparent(&mut project, "second", "other")?;
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "set",
        "behavioralFeature",
        json!({"$ref":"other"}),
    )?;
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn behavioral_feature_navigation_is_not_invented_for_behavior_owned_parameter_sets() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "operation")
        .ok_or("operation")?
        .kind = "uml.opaque_behavior".into();
    expect(&project, UNKNOWN)?;
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
fn unknown_member_metaclass_does_not_invent_a_direction_or_membership() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "first")
        .ok_or("first")?
        .kind = "uml.unknown_parameter".into();
    author(&mut project, "second", "direction", json!("out"))?;
    expect(&project, UNKNOWN)?;
    author(&mut project, "second", "isStream", json!(true))?;
    expect(&project, UNKNOWN)?;
    Ok(())
}

#[test]
fn invalid_inverse_references_dominate_a_true_membership() -> Result {
    for inverse in [
        Value::Null,
        json!(42),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"class"}]),
        json!([{"$ref":"set"},{"$ref":"set"}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "second", "isStream", json!(true))?;
        author(&mut project, "first", "parameterSet", inverse)?;
        expect(&project, ERROR)?;
    }
    Ok(())
}

#[test]
fn known_output_violation_dominates_unknown_input_implication() -> Result {
    for reverse in [false, true] {
        let mut project = document(true)?;
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "first")
            .ok_or("first")?
            .kind = "uml.unknown_parameter".into();
        author(&mut project, "second", "direction", json!("out"))?;
        project.elements.push(serde_json::from_value(json!({"id":"output-member","kind":"uml.parameter","name":"Output","properties":{"direction":"out","type":{"$ref":"class"}}}))?);
        project.relations.push(serde_json::from_value(json!({"id":"owns-output-member","kind":"owns","from":"operation","to":"output-member"}))?);
        author(
            &mut project,
            "set",
            "parameter",
            json!([{"$ref":"first"},{"$ref":"output-member"}]),
        )?;
        if reverse {
            project.elements.reverse();
        }
        expect(&project, ERROR)?;
        author(&mut project, "second", "isStream", json!(true))?;
        expect(&project, UNKNOWN)?;
    }
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
