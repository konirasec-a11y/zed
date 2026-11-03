use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "Parameter-object_effect:";
const ERROR: &str = "UML_PARAMETER_OBJECT_EFFECT";
const INVALID: &str = "MDE_PARAMETER_OBJECT_EFFECT_INVALID";
const UNKNOWN: &str = "MDE_PARAMETER_OBJECT_EFFECT_UNVERIFIED";
fn document(kind: &str, rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Effects","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"type","kind":kind,"name":"Type"},
        {"id":"parameter","kind":"uml.parameter","name":"Argument","properties":{"type":{"$ref":"type"},"effect":"read"}}
    ],"relations":[{"id":"owns-type","kind":"owns","from":"root","to":"type"}]});
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
}
fn author(project: &mut Project, key: &str, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "parameter")
        .ok_or("parameter")?
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
        .filter(|i| {
            i["subject_id"] == "parameter"
                && i["message"].as_str().is_some_and(|m| m.starts_with(RULE))
        })
        .cloned()
        .collect())
}
fn expect(project: &Project, code: &str) -> Result {
    let found = issues(project)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    assert_eq!(
        found[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
#[test]
fn given_data_type_or_subclass_then_every_defined_effect_is_forbidden() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.data_type", "uml.primitive_type", "uml.enumeration"] {
            for effect in ["read", "update", "create", "delete"] {
                let mut project = document(kind, rooted)?;
                author(&mut project, "effect", json!(effect))?;
                expect(&project, ERROR)?;
                for format in [Format::Json, Format::Protobuf] {
                    expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
                }
            }
        }
    }
    Ok(())
}
#[test]
fn given_no_effect_then_implication_holds_even_for_external_or_unknown_type() -> Result {
    for target in [
        json!({"$ref":"type"}),
        json!({"href":"https://example.org/model#Type"}),
    ] {
        for kind in ["uml.data_type", "uml.unknown_type"] {
            let mut project = document(kind, true)?;
            author(&mut project, "type", target.clone())?;
            author(&mut project, "effect", Value::Null)?;
            assert!(issues(&project)?.is_empty());
            project
                .elements
                .iter_mut()
                .find(|e| e.id == "parameter")
                .ok_or("parameter")?
                .properties
                .remove("effect");
            assert!(issues(&project)?.is_empty());
        }
    }
    Ok(())
}
#[test]
fn given_non_data_type_or_absent_type_then_effect_is_allowed() -> Result {
    let mut project = document("uml.class", true)?;
    assert!(issues(&project)?.is_empty());
    author(&mut project, "type", Value::Null)?;
    assert!(issues(&project)?.is_empty());
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "parameter")
        .ok_or("parameter")?
        .properties
        .remove("type");
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn given_unresolved_type_and_effect_then_result_is_unknown() -> Result {
    let mut project = document("uml.unknown_type", true)?;
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "type",
        json!({"href":"https://example.org/model#Type"}),
    )?;
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn given_malformed_effect_then_invalid_survives_unknown_or_non_data_type() -> Result {
    for kind in ["uml.unknown_type", "uml.class", "uml.data_type"] {
        for effect in [
            json!(42),
            json!(true),
            json!("invalid"),
            json!([]),
            json!({"href":"https://example.org/effect"}),
        ] {
            let mut project = document(kind, true)?;
            author(&mut project, "effect", effect)?;
            expect(&project, INVALID)?;
            author(
                &mut project,
                "type",
                json!({"href":"https://example.org/model#Type"}),
            )?;
            expect(&project, INVALID)?;
        }
    }
    Ok(())
}
#[test]
fn given_malformed_type_then_absent_effect_does_not_hide_invalid() -> Result {
    for target in [
        json!({"$ref":"missing"}),
        json!({"$ref":"parameter"}),
        json!(17),
        json!([]),
        json!({"href":17}),
    ] {
        let mut project = document("uml.class", true)?;
        author(&mut project, "type", target)?;
        author(&mut project, "effect", Value::Null)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn when_effect_is_added_to_data_type_parameter_then_command_rolls_back_and_repair_succeeds()
-> Result {
    let mut project = document("uml.data_type", true)?;
    author(&mut project, "effect", Value::Null)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "parameter".into(),
            path: "properties.effect".into(),
            value: json!("read"),
        },
        1,
    )
    .expect_err("effect forbidden");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(before, encode(&project, Format::Json)?);
    author(&mut project, "effect", json!("read"))?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format)?, format)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "parameter".into(),
                path: "properties.effect".into(),
                value: Value::Null,
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}
