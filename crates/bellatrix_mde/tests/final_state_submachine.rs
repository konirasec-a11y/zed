use bellatrix_mde::{Format, Project, StructuralSeverity, decode, encode, uml::root::Model};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_FINAL_STATE_CANNOT_REFERENCE_SUBMACHINE";
const UNKNOWN: &str = "MDE_FINAL_STATE_SUBMACHINE_UNVERIFIED";

fn fixture(value: Option<Value>) -> Result<Project> {
    let mut project: Project = serde_json::from_value(json!({
        "id":"model", "name":"FinalState submachine", "elements":[
            {"id":"final", "kind":"uml.final_state", "name":"Done"},
            {"id":"machine", "kind":"uml.state_machine", "name":"Machine"},
            {"id":"protocol", "kind":"uml.protocol_state_machine", "name":"Protocol"},
            {"id":"wrong", "kind":"uml.class", "name":"Wrong"},
            {"id":"unknown", "kind":"extension.unknown", "name":"Unknown"}
        ]
    }))?;
    if let Some(value) = value {
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "final")
            .ok_or("FinalState")?
            .properties
            .insert("submachine".into(), value);
    }
    Ok(project)
}
fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project)
        .validate()
        .into_iter()
        .filter(|issue| issue.subject_id == "final" && matches!(issue.code, ERROR | UNKNOWN))
        .map(|issue| (issue.severity, issue.code))
        .collect()
}

#[test]
fn optional_empty_submachine_is_valid_but_local_and_external_presence_is_invalid_after_roundtrip()
-> Result {
    for (value, invalid) in [
        (None, false),
        (Some(json!(null)), false),
        (Some(json!({"$ref":"machine"})), true),
        (Some(json!({"$ref":"protocol"})), true),
        (Some(json!({"$ref":"unknown"})), true),
        (Some(json!({"href":"other.uml#Machine"})), true),
    ] {
        let project = fixture(value)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format)?, format)?;
            assert_eq!(
                outcome(&reopened),
                if invalid {
                    vec![(StructuralSeverity::Error, ERROR)]
                } else {
                    vec![]
                }
            );
        }
    }
    Ok(())
}

fn inverse(project: &mut Project, id: &str, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("machine")?
        .properties
        .insert("submachineState".into(), value);
    Ok(())
}

#[test]
fn authored_opposite_proves_violation_without_direct_reference_and_survives_roundtrip() -> Result {
    for machine in ["machine", "protocol"] {
        let mut project = fixture(None)?;
        inverse(&mut project, machine, json!([{"$ref":"final"}]))?;
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(
                outcome(&decode(&encode(&project, format)?, format)?),
                vec![(StructuralSeverity::Error, ERROR)]
            );
        }
    }
    Ok(())
}
