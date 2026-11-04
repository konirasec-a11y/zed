use bellatrix_mde::{Format, Project, StructuralSeverity, commands::{Command, apply}, decode, encode, uml::root::Model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_FINAL_STATE_NO_REGIONS";
const UNKNOWN: &str = "MDE_FINAL_STATE_REGIONS_UNVERIFIED";
fn fixture(owner: Option<&str>) -> Result<Project> {
    let mut project: Project = serde_json::from_value(json!({"id":"model","name":"FinalState regions","elements":[
        {"id":"final","kind":"uml.final_state","name":"Done"},
        {"id":"state","kind":"uml.state","name":"State"},
        {"id":"region","kind":"uml.region","name":"Region"},
        {"id":"unknown","kind":"extension.unknown","name":"Unknown"}
    ]}))?;
    if let Some(owner) = owner { own(&mut project, "region", owner)?; }
    Ok(project)
}
fn own(project: &mut Project, child: &str, owner: &str) -> Result {
    project.relations.push(serde_json::from_value(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}))?);
    Ok(())
}
fn edit(project: &mut Project, id: &str, key: &str, value: Value) -> Result {
    project.elements.iter_mut().find(|element| element.id == id).ok_or("element")?.properties.insert(key.into(), value);
    Ok(())
}
fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project).validate().into_iter()
        .filter(|issue| issue.subject_id == "final" && matches!(issue.code, ERROR | UNKNOWN))
        .map(|issue| (issue.severity, issue.code)).collect()
}
fn invalid() -> Vec<(StructuralSeverity, &'static str)> { vec![(StructuralSeverity::Error, ERROR)] }

#[test]
fn empty_role_is_valid_but_owned_or_explicit_regions_are_invalid_after_roundtrip() -> Result {
    for owner in [None, Some("final"), Some("state")] {
        for role in [None, Some(json!([])), Some(json!([{"$ref":"region"}]))] {
            let mut project = fixture(owner)?;
            let explicit_nonempty = role.as_ref().and_then(Value::as_array).is_some_and(|values| !values.is_empty());
            if let Some(role) = role { edit(&mut project, "final", "region", role)?; }
            let expected = if owner == Some("final") || explicit_nonempty { invalid() } else { vec![] };
            for format in [Format::Json, Format::Protobuf] {
                assert_eq!(outcome(&decode(&encode(&project, format)?, format)?), expected);
            }
        }
    }
    Ok(())
}

#[test]
fn reparent_gate_rejects_region_into_final_state_atomically_and_repairs_invalid_import() -> Result {
    let project = fixture(Some("state"))?;
    let before = project.clone();
    let failure = apply(&project, &Command::SetOwner { element_id:"region".into(), owner_id:Some("final".into()) }, 0)
        .expect_err("FinalState cannot own a Region");
    assert!(failure.starts_with(ERROR), "{failure}");
    assert_eq!(project, before);
    let invalid_import = fixture(Some("final"))?;
    let repaired = apply(&invalid_import, &Command::SetOwner { element_id:"region".into(), owner_id:Some("state".into()) }, 0)?;
    assert!(outcome(&repaired).is_empty());
    for format in [Format::Json, Format::Protobuf] {
        assert!(outcome(&decode(&encode(&repaired, format)?, format)?).is_empty());
    }
    Ok(())
}
