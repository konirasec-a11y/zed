use bellatrix_mde::{Format, Project, StructuralSeverity, commands::{Command, apply}, decode, encode, uml::root::Model, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_REGION_OWNED";
const UNKNOWN: &str = "MDE_REGION_OWNED_UNVERIFIED";

fn fixture(owner: Option<&str>) -> Result<Project> {
    let mut project: Project = serde_json::from_value(json!({
        "id":"model", "name":"Region ownership", "elements":[
            {"id":"region","kind":"uml.region","name":"Region"},
            {"id":"state","kind":"uml.state","name":"State"},
            {"id":"machine","kind":"uml.state_machine","name":"Machine"},
            {"id":"protocol","kind":"uml.protocol_state_machine","name":"Protocol"},
            {"id":"unknown","kind":"extension.unknown","name":"Unknown"},
            {"id":"wrong","kind":"uml.class","name":"Wrong"}
        ]
    }))?;
    if let Some(owner) = owner {
        project.relations.push(serde_json::from_value(json!({"id":"owns-region","kind":"owns","from":owner,"to":"region"}))?);
    }
    Ok(project)
}
fn edit(project: &mut Project, id: &str, key: &str, value: Value) -> Result {
    project.elements.iter_mut().find(|element| element.id == id).ok_or("element")?
        .properties.insert(key.into(), value);
    Ok(())
}
fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project).validate().into_iter()
        .filter(|issue| issue.subject_id == "region" && matches!(issue.code, ERROR | UNKNOWN))
        .map(|issue| (issue.severity, issue.code)).collect()
}
fn invalid() -> Vec<(StructuralSeverity, &'static str)> { vec![(StructuralSeverity::Error, ERROR)] }
fn unknown() -> Vec<(StructuralSeverity, &'static str)> { vec![(StructuralSeverity::Warning, UNKNOWN)] }

#[test]
fn canonical_state_or_machine_owner_is_exclusive_and_unowned_drafts_remain_allowed() -> Result {
    for owner in [None, Some("state"), Some("machine"), Some("protocol")] {
        let project = fixture(owner)?;
        assert!(outcome(&project).is_empty());
        if let Some(owner) = owner {
            let role = if owner == "state" { "state" } else { "stateMachine" };
            let mut explicit = project;
            edit(&mut explicit, "region", role, json!({"$ref":owner}))?;
            edit(&mut explicit, owner, "region", json!([{"$ref":"region"}]))?;
            for format in [Format::Json, Format::Protobuf] {
                assert!(outcome(&decode(&encode(&explicit, format)?, format)?).is_empty());
            }
        }
    }
    let mut draft = fixture(None)?;
    edit(&mut draft, "region", "state", Value::Null)?;
    edit(&mut draft, "region", "stateMachine", Value::Null)?;
    assert!(outcome(&draft).is_empty());
    Ok(())
}

#[test]
fn two_present_roles_are_invalid_even_when_both_references_are_external() -> Result {
    for (state, machine) in [(json!({"$ref":"state"}),json!({"$ref":"machine"})),
        (json!({"href":"external.uml#State"}),json!({"href":"external.uml#Machine"}))] {
        let mut project = fixture(None)?;
        edit(&mut project, "region", "state", state)?;
        edit(&mut project, "region", "stateMachine", machine)?;
        assert_eq!(outcome(&project), invalid());
    }
    for (owner, opposite) in [("state","stateMachine"),("machine","state")] {
        let mut project = fixture(Some(owner))?;
        edit(&mut project, "region", opposite, json!({"href":"external.uml#Owner"}))?;
        assert_eq!(outcome(&project), invalid(), "canonical presence must dominate uncertainty");
    }
    Ok(())
}

#[test]
fn local_inverses_must_agree_with_canonical_ownership() -> Result {
    for (owner, role, value) in [(None,"state",json!({"$ref":"state"})),
        (Some("state"),"state",Value::Null), (Some("state"),"stateMachine",json!({"$ref":"machine"})),
        (Some("machine"),"stateMachine",json!({"$ref":"protocol"})),
        (Some("unknown"),"state",json!({"$ref":"state"}))] {
        let mut project = fixture(owner)?;
        edit(&mut project, "region", role, value)?;
        assert_eq!(outcome(&project), invalid());
    }
    let mut project = fixture(Some("state"))?;
    edit(&mut project, "state", "region", json!([]))?;
    assert_eq!(outcome(&project), invalid(), "explicit empty composite contradicts ownership");
    Ok(())
}

#[test]
fn incomplete_owner_evidence_is_unknown_without_hiding_invalid_values() -> Result {
    assert_eq!(outcome(&fixture(Some("unknown"))?), unknown());
    let mut external = fixture(None)?;
    edit(&mut external, "region", "state", json!({"href":"external.uml#State"}))?;
    assert_eq!(outcome(&external), unknown());
    for value in [json!(42), json!([]), json!({"$ref":"missing"}), json!({"$ref":"wrong"}), json!({"href":""})] {
        let mut project = external.clone();
        edit(&mut project, "region", "stateMachine", value)?;
        assert_eq!(outcome(&project), invalid());
    }
    let mut same_unknown = fixture(Some("unknown"))?;
    edit(&mut same_unknown, "region", "state", json!({"$ref":"unknown"}))?;
    assert_eq!(outcome(&same_unknown), unknown());
    Ok(())
}

#[test]
fn duplicate_or_ambiguous_ownership_is_invalid() -> Result {
    for owner in ["state","machine"] {
        let mut project = fixture(Some("state"))?;
        project.relations.push(serde_json::from_value(json!({"id":"other-owns","kind":"owns","from":owner,"to":"region"}))?);
        assert_eq!(outcome(&project), invalid());
    }
    Ok(())
}

#[test]
fn public_api_preserves_the_same_region_violation() -> Result {
    let mut project = fixture(Some("state"))?;
    edit(&mut project, "region", "stateMachine", json!({"href":"external.uml#Machine"}))?;
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":project}).to_string())?)?;
    assert!(response["diagnostics"].as_array().ok_or("diagnostics")?.iter()
        .any(|issue| issue["subject_id"] == "region" && issue["code"] == ERROR));
    Ok(())
}

#[test]
fn reparenting_rejects_stale_authored_inverse_atomically_but_allows_consistent_move() -> Result {
    let mut project = fixture(Some("state"))?;
    edit(&mut project, "region", "state", json!({"$ref":"state"}))?;
    let before = project.clone();
    let command = Command::SetOwner { element_id: "region".into(), owner_id: Some("machine".into()) };
    let failure = apply(&project, &command, 0).expect_err("the stale inverse must prevent the reparent");
    assert!(failure.starts_with(ERROR), "{failure}");
    assert_eq!(project, before);
    project.elements.iter_mut().find(|element| element.id == "region").ok_or("Region")?.properties.remove("state");
    let moved = apply(&project, &command, 0)?;
    assert!(outcome(&moved).is_empty());
    assert!(moved.relations.iter().any(|edge| edge.kind == "owns" && edge.to == "region" && edge.from == "machine"));
    Ok(())
}

#[test]
fn unresolved_forward_collection_cannot_erase_a_known_owner_witness() -> Result {
    for (owner, opposite) in [("state","stateMachine"),("machine","state"),("protocol","state")] {
        let mut project = fixture(Some(owner))?;
        edit(&mut project, owner, "region", json!([{"href":"external.uml#Region"}]))?;
        edit(&mut project, "region", opposite, json!({"href":"external.uml#Owner"}))?;
        assert_eq!(outcome(&project), invalid(), "{owner}: known ownership plus authored opposite proves both roles present");
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(outcome(&decode(&encode(&project, format)?, format)?), invalid());
        }
        project.elements.iter_mut().find(|element| element.id == "region").ok_or("Region")?.properties.clear();
        let canonical_role = if owner == "state" { "state" } else { "stateMachine" };
        edit(&mut project, "region", canonical_role, Value::Null)?;
        assert_eq!(outcome(&project), invalid(), "null must not erase canonical ownership despite unresolved forward membership");
        assert!(Model::from_project(&project).validate().iter().any(|issue| issue.subject_id == "region" && issue.code == ERROR && issue.message.contains("null")));
    }
    Ok(())
}
