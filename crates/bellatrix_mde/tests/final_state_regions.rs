use bellatrix_mde::{Format, Project, StructuralSeverity, commands::{Command, apply}, decode, encode, uml::root::Model, validate_uml_model};
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
fn unknown() -> Vec<(StructuralSeverity, &'static str)> { vec![(StructuralSeverity::Warning, UNKNOWN)] }

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
fn nonzero_cardinality_is_decisive_even_with_external_or_unknown_members() -> Result {
    for role in [json!([{"href":"external.uml#Region"}]), json!([{"$ref":"unknown"}]),
        json!([{"href":"external.uml#Region"},{"$ref":"region"}])] {
        let mut project = fixture(Some("final"))?;
        own(&mut project, "unknown", "final")?;
        edit(&mut project, "final", "region", role)?;
        assert_eq!(outcome(&project), invalid());
    }
    let mut project = fixture(None)?;
    own(&mut project, "unknown", "final")?;
    assert_eq!(outcome(&project), unknown());
    edit(&mut project, "final", "region", json!([{"href":"external.uml#Region"}]))?;
    assert_eq!(outcome(&project), invalid());
    Ok(())
}

#[test]
fn malformed_collection_and_duplicate_ownership_remain_invalid() -> Result {
    for role in [json!(null), json!(42), json!([null]), json!([{"$ref":"missing"}]),
        json!([{"$ref":"state"}]), json!([{"$ref":"region"},{"$ref":"region"}]), json!([{"href":""}])] {
        for owner in [None, Some("final")] {
            let mut project = fixture(owner)?;
            edit(&mut project, "final", "region", role.clone())?;
            assert_eq!(outcome(&project), invalid(), "owner={owner:?}");
        }
    }
    let mut duplicate = fixture(Some("final"))?;
    own(&mut duplicate, "region", "final")?;
    assert_eq!(outcome(&duplicate), invalid());
    Ok(())
}

#[test]
fn authored_inverse_cannot_claim_membership_without_canonical_ownership() -> Result {
    for owner in [None, Some("state"), Some("final")] {
        let mut project = fixture(owner)?;
        edit(&mut project, "region", "state", json!({"$ref":"final"}))?;
        assert_eq!(outcome(&project), invalid());
    }
    let mut contradictory_null = fixture(Some("final"))?;
    edit(&mut contradictory_null, "region", "state", Value::Null)?;
    assert_eq!(outcome(&contradictory_null), invalid());
    Ok(())
}

#[test]
fn incomplete_inverse_is_unknown_only_when_membership_is_not_already_excluded() -> Result {
    let mut project = fixture(None)?;
    edit(&mut project, "region", "state", json!({"href":"external.uml#State"}))?;
    assert_eq!(outcome(&project), unknown());
    own(&mut project, "region", "state")?;
    assert!(outcome(&project).is_empty());
    assert!(Model::from_project(&project).validate().iter().any(|issue|
        issue.subject_id == "region" && issue.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"));
    project.relations.clear();
    edit(&mut project, "region", "state", json!({"$ref":"unknown"}))?;
    assert!(outcome(&project).is_empty(), "a different complete local identity excludes this FinalState");
    edit(&mut project, "region", "state", json!(42))?;
    assert_eq!(outcome(&project), invalid());
    Ok(())
}

#[test]
fn descendants_and_cached_flags_do_not_replace_direct_region_membership() -> Result {
    let mut project = fixture(Some("state"))?;
    own(&mut project, "state", "final")?;
    edit(&mut project, "final", "isComposite", json!(true))?;
    assert!(outcome(&project).is_empty(), "only direct Regions count");
    project.relations.retain(|edge| edge.to != "region");
    own(&mut project, "region", "final")?;
    edit(&mut project, "final", "isComposite", json!(false))?;
    assert_eq!(outcome(&project), invalid());
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

#[test]
fn public_api_preserves_the_same_no_regions_error() -> Result {
    let project = fixture(Some("final"))?;
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":project}).to_string())?)?;
    assert!(response["diagnostics"].as_array().ok_or("diagnostics")?.iter()
        .any(|issue| issue["subject_id"] == "final" && issue["code"] == ERROR));
    Ok(())
}
