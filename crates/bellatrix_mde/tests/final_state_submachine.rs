use bellatrix_mde::{Format, Project, StructuralSeverity, decode, encode, uml::root::Model, validate_uml_model};
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
        project.elements.iter_mut().find(|element| element.id == "final").ok_or("FinalState")?
            .properties.insert("submachine".into(), value);
    }
    Ok(project)
}
fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project).validate().into_iter()
        .filter(|issue| issue.subject_id == "final" && matches!(issue.code, ERROR | UNKNOWN))
        .map(|issue| (issue.severity, issue.code)).collect()
}

#[test]
fn optional_empty_submachine_is_valid_but_local_and_external_presence_is_invalid_after_roundtrip() -> Result {
    for (value, invalid) in [(None, false), (Some(json!(null)), false),
        (Some(json!({"$ref":"machine"})), true), (Some(json!({"$ref":"protocol"})), true),
        (Some(json!({"$ref":"unknown"})), true), (Some(json!({"href":"other.uml#Machine"})), true)] {
        let project = fixture(value)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format)?, format)?;
            assert_eq!(outcome(&reopened), if invalid { vec![(StructuralSeverity::Error, ERROR)] } else { vec![] });
        }
    }
    Ok(())
}

#[test]
fn malformed_or_unresolved_local_reference_is_invalid_not_a_successful_absence() -> Result {
    for value in [json!(42), json!("machine"), json!([]), json!([{"$ref":"machine"}]),
        json!({}), json!({"$ref":""}), json!({"$ref":"missing"}), json!({"$ref":"wrong"}),
        json!({"href":""}), json!({"href":null}), json!({"href":"external.uml#Machine","$ref":"machine"})] {
        assert_eq!(outcome(&fixture(Some(value))?), vec![(StructuralSeverity::Error, ERROR)]);
    }
    Ok(())
}

#[test]
fn stored_derived_flags_and_owned_machines_do_not_replace_the_normative_reference() -> Result {
    let mut project = fixture(None)?;
    project.elements.iter_mut().find(|element| element.id == "final").ok_or("FinalState")?
        .properties.insert("isSubmachineState".into(), json!(true));
    project.relations.push(serde_json::from_value(json!({"id":"owns-machine","kind":"owns","from":"final","to":"machine"}))?);
    assert!(outcome(&project).is_empty());
    let final_state = project.elements.iter_mut().find(|element| element.id == "final").ok_or("FinalState")?;
    final_state.properties.insert("isSubmachineState".into(), json!(false));
    final_state.properties.insert("submachine".into(), json!({"$ref":"machine"}));
    assert_eq!(outcome(&project), vec![(StructuralSeverity::Error, ERROR)]);
    final_state_kind(&mut project, "uml.state")?;
    assert!(outcome(&project).is_empty(), "ordinary State is outside this constraint");
    Ok(())
}
fn final_state_kind(project: &mut Project, kind: &str) -> Result {
    project.elements.iter_mut().find(|element| element.id == "final").ok_or("FinalState")?.kind = kind.into();
    Ok(())
}

#[test]
fn public_validation_api_reports_the_same_known_violation() -> Result {
    let project = fixture(Some(json!({"href":"other.uml#Machine"})))?;
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":project}).to_string())?)?;
    let issues = response["diagnostics"].as_array().ok_or("diagnostics")?;
    assert!(issues.iter().any(|issue| issue["subject_id"] == "final" && issue["code"] == ERROR));
    assert!(!issues.iter().any(|issue| issue["subject_id"] == "final" && issue["code"] == UNKNOWN));
    Ok(())
}

fn inverse(project: &mut Project, id: &str, value: Value) -> Result {
    project.elements.iter_mut().find(|element| element.id == id).ok_or("machine")?
        .properties.insert("submachineState".into(), value);
    Ok(())
}

#[test]
fn authored_opposite_proves_violation_without_direct_reference_and_survives_roundtrip() -> Result {
    for machine in ["machine", "protocol"] {
        let mut project = fixture(None)?;
        inverse(&mut project, machine, json!([{"$ref":"final"}]))?;
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(outcome(&decode(&encode(&project, format)?, format)?), vec![(StructuralSeverity::Error, ERROR)]);
        }
    }
    Ok(())
}

#[test]
fn incomplete_opposite_is_unknown_unless_an_independent_witness_proves_violation() -> Result {
    for reverse in [false, true] {
        let mut role = vec![json!({"href":"external.uml#State"})];
        let mut project = fixture(None)?;
        inverse(&mut project, "machine", json!(role))?;
        assert_eq!(outcome(&project), vec![(StructuralSeverity::Warning, UNKNOWN)]);
        role.push(json!({"$ref":"final"}));
        if reverse { role.reverse(); }
        inverse(&mut project, "machine", json!(role))?;
        assert_eq!(outcome(&project), vec![(StructuralSeverity::Error, ERROR)]);
    }
    let mut project = fixture(Some(json!({"href":"other.uml#Machine"})))?;
    inverse(&mut project, "machine", json!([{"href":"other.uml#State"}]))?;
    assert_eq!(outcome(&project), vec![(StructuralSeverity::Error, ERROR)]);
    Ok(())
}

#[test]
fn malformed_and_contradictory_opposites_are_invalid() -> Result {
    for role in [json!(null), json!(42), json!([{"$ref":"missing"}]),
        json!([{"$ref":"final"},{"$ref":"final"}]), json!([{"$ref":"wrong"}]),
        json!([{"href":""}]), json!([{"href":"other.uml#State"},null])] {
        let mut project = fixture(None)?;
        inverse(&mut project, "machine", role)?;
        assert_eq!(outcome(&project), vec![(StructuralSeverity::Error, ERROR)]);
    }
    for direct in [Some(Value::Null), Some(json!({"$ref":"protocol"}))] {
        let mut project = fixture(direct)?;
        inverse(&mut project, "machine", json!([{"$ref":"final"}]))?;
        assert!(Model::from_project(&project).validate().iter().any(|issue| issue.code == ERROR && issue.message.contains("contradicts")));
    }
    let mut project = fixture(Some(json!({"$ref":"machine"})))?;
    inverse(&mut project, "machine", json!([]))?;
    assert!(Model::from_project(&project).validate().iter().any(|issue| issue.code == ERROR && issue.message.contains("excluded")));
    let mut multiple = fixture(None)?;
    inverse(&mut multiple, "machine", json!([{"$ref":"final"}]))?;
    inverse(&mut multiple, "protocol", json!([{"$ref":"final"}]))?;
    assert!(Model::from_project(&multiple).validate().iter().any(|issue| issue.code == ERROR && issue.message.contains("more than one")));
    Ok(())
}

#[test]
fn complete_empty_opposites_do_not_invent_a_submachine() -> Result {
    let mut project = fixture(None)?;
    inverse(&mut project, "machine", json!([]))?;
    assert!(outcome(&project).is_empty());
    project.elements.push(serde_json::from_value(json!({"id":"other","kind":"uml.state","name":"Other"}))?);
    inverse(&mut project, "machine", json!([{"$ref":"other"}]))?;
    assert!(outcome(&project).is_empty());
    Ok(())
}

#[test]
fn complete_local_identities_decide_exclusion_despite_an_unresolved_member_type() -> Result {
    let mut project = fixture(None)?;
    inverse(&mut project, "machine", json!([{"$ref":"unknown"}]))?;
    assert!(outcome(&project).is_empty());
    assert!(Model::from_project(&project).validate().iter().any(|issue|
        issue.subject_id == "machine" && issue.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
            && issue.message.starts_with("StateMachine-submachineState:")));
    project.elements.iter_mut().find(|element| element.id == "final").ok_or("FinalState")?
        .properties.insert("submachine".into(), json!({"$ref":"machine"}));
    assert!(Model::from_project(&project).validate().iter().any(|issue|
        issue.subject_id == "final" && issue.code == ERROR && issue.message.contains("excluded")));
    inverse(&mut project, "machine", json!([{"$ref":"unknown"},{"$ref":"missing"}]))?;
    assert_eq!(outcome(&project), vec![(StructuralSeverity::Error, ERROR)], "invalid references cannot be suppressed by complete identities");
    Ok(())
}
