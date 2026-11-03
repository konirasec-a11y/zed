use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn document(properties: Value) -> Result<Project> {
    Ok(serde_json::from_value(
        json!({"id":"project","name":"Scalars","elements":[{"id":"parameter","kind":"uml.parameter","name":"P","properties":properties}],"relations":[]}),
    )?)
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
                && i["message"].as_str().is_some_and(|m| {
                    ["in_and_out", "not_exception", "stream_and_exception"]
                        .iter()
                        .any(|r| m.starts_with(&format!("Parameter::{r}:")))
                })
        })
        .cloned()
        .collect())
}
fn has(found: &[Value], rule: &str, code: &str) -> bool {
    found.iter().any(|i| {
        i["code"] == code
            && i["message"]
                .as_str()
                .is_some_and(|m| m.starts_with(&format!("Parameter::{rule}:")))
    })
}
#[test]
fn given_all_valid_scalar_combinations_then_each_rule_matches_its_ocl_independently() -> Result {
    for direction in ["in", "out", "inout", "return"] {
        for effect in [
            Value::Null,
            json!("read"),
            json!("update"),
            json!("create"),
            json!("delete"),
        ] {
            for exception in [false, true] {
                for stream in [false, true] {
                    let project = document(
                        json!({"direction":direction,"effect":effect,"isException":exception,"isStream":stream}),
                    )?;
                    let found = issues(&project)?;
                    let bad_effect = (effect == "delete" && !["in", "inout"].contains(&direction))
                        || (effect == "create" && !["out", "inout", "return"].contains(&direction));
                    for (rule, bad) in [
                        ("in_and_out", bad_effect),
                        (
                            "not_exception",
                            exception && ["in", "inout"].contains(&direction),
                        ),
                        ("stream_and_exception", exception && stream),
                    ] {
                        assert_eq!(
                            has(&found, rule, "UML_PARAMETER_CONSTRAINT"),
                            bad,
                            "{direction} {effect} {exception} {stream}: {found:?}"
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
#[test]
fn given_three_simultaneous_violations_then_all_three_are_reported_and_roundtrip() -> Result {
    let project =
        document(json!({"direction":"in","effect":"create","isException":true,"isStream":true}))?;
    for format in [Format::Json, Format::Protobuf] {
        let found = issues(&decode(&encode(&project, format)?, format)?)?;
        assert_eq!(found.len(), 3, "{found:?}");
        for rule in ["in_and_out", "not_exception", "stream_and_exception"] {
            assert!(has(&found, rule, "UML_PARAMETER_CONSTRAINT"));
        }
    }
    Ok(())
}
#[test]
fn given_absent_scalars_then_normative_defaults_are_used_but_null_required_scalars_are_invalid()
-> Result {
    assert!(issues(&document(json!({}))?)?.is_empty());
    let found = issues(&document(json!({"effect":"create","isException":true}))?)?;
    assert!(has(&found, "in_and_out", "UML_PARAMETER_CONSTRAINT"));
    assert!(has(&found, "not_exception", "UML_PARAMETER_CONSTRAINT"));
    for (key, rules) in [
        ("direction", vec!["in_and_out", "not_exception"]),
        ("isException", vec!["not_exception", "stream_and_exception"]),
        ("isStream", vec!["stream_and_exception"]),
    ] {
        for bad in [
            Value::Null,
            json!(42),
            json!("invalid"),
            json!({"href":"https://example.org/scalar"}),
        ] {
            let found = issues(&document(json!({key:bad}))?)?;
            for rule in &rules {
                assert!(
                    has(&found, rule, "MDE_PARAMETER_SCALAR_INVALID"),
                    "{key}: {found:?}"
                );
            }
        }
    }
    Ok(())
}
#[test]
fn given_malformed_effect_then_it_is_invalid_not_absent_and_other_rules_still_report() -> Result {
    for effect in [
        json!(true),
        json!(42),
        json!([]),
        json!("invalid"),
        json!({"href":"https://example.org/effect"}),
    ] {
        let found = issues(&document(
            json!({"effect":effect,"isException":true,"isStream":true}),
        )?)?;
        assert!(has(&found, "in_and_out", "MDE_PARAMETER_SCALAR_INVALID"));
        assert!(has(&found, "not_exception", "UML_PARAMETER_CONSTRAINT"));
        assert!(has(
            &found,
            "stream_and_exception",
            "UML_PARAMETER_CONSTRAINT"
        ));
    }
    Ok(())
}
#[test]
fn given_invalid_direction_then_independent_stream_exception_violation_is_not_hidden() -> Result {
    let found = issues(&document(
        json!({"direction":42,"isException":true,"isStream":true}),
    )?)?;
    assert!(has(&found, "in_and_out", "MDE_PARAMETER_SCALAR_INVALID"));
    assert!(has(&found, "not_exception", "MDE_PARAMETER_SCALAR_INVALID"));
    assert!(has(
        &found,
        "stream_and_exception",
        "UML_PARAMETER_CONSTRAINT"
    ));
    Ok(())
}
#[test]
fn given_unknown_type_then_scalar_rules_remain_decidable_and_invalid_is_not_downgraded() -> Result {
    let project = document(
        json!({"type":{"href":"https://example.org/type"},"effect":"create","isException":true,"isStream":true}),
    )?;
    assert_eq!(issues(&project)?.len(), 3);
    let kernel = bellatrix_mde::uml::root::Model::from_project(&project).validate();
    assert!(
        kernel
            .iter()
            .any(|i| i.code == "MDE_PARAMETER_OBJECT_EFFECT_UNVERIFIED")
    );
    let project = document(
        json!({"type":{"href":"https://example.org/type"},"direction":42,"isException":true,"isStream":true}),
    )?;
    assert!(has(
        &issues(&project)?,
        "not_exception",
        "MDE_PARAMETER_SCALAR_INVALID"
    ));
    Ok(())
}
#[test]
fn when_scalar_edit_introduces_violation_then_command_rolls_back_and_repair_succeeds() -> Result {
    for (initial, path, value, rule) in [
        (
            json!({"effect":null}),
            "effect",
            json!("create"),
            "in_and_out",
        ),
        (
            json!({"isException":false}),
            "isException",
            json!(true),
            "not_exception",
        ),
        (
            json!({"direction":"out","isException":true,"isStream":false}),
            "isStream",
            json!(true),
            "stream_and_exception",
        ),
    ] {
        let project = document(initial)?;
        let before = encode(&project, Format::Json)?;
        let error = apply(
            &project,
            &Command::SetProperty {
                element_id: "parameter".into(),
                path: format!("properties.{path}"),
                value: value.clone(),
            },
            1,
        )
        .expect_err("violation");
        assert!(error.contains(rule), "{error}");
        assert_eq!(before, encode(&project, Format::Json)?);
        let mut invalid = project.clone();
        invalid.elements[0].properties.insert(path.into(), value);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&invalid, format)?, format)?;
            let repaired = apply(
                &reopened,
                &Command::SetProperty {
                    element_id: "parameter".into(),
                    path: format!("properties.{path}"),
                    value: project.elements[0].properties[path].clone(),
                },
                2,
            )?;
            assert!(issues(&repaired)?.is_empty());
        }
    }
    Ok(())
}
