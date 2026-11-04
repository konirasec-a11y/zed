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
