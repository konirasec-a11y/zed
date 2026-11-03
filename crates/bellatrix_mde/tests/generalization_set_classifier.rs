use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "GeneralizationSet-generalization_same_classifier:";
const ERROR: &str = "UML_GENERALIZATION_SET_SAME_CLASSIFIER";
const INVALID: &str = "MDE_GENERALIZATION_SET_SAME_CLASSIFIER_INVALID";
const UNKNOWN: &str = "MDE_GENERALIZATION_SET_SAME_CLASSIFIER_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Generalization sets","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"general","kind":"uml.class","name":"General"},
        {"id":"other","kind":"uml.class","name":"Other"},
        {"id":"first","kind":"uml.class","name":"First"},
        {"id":"second","kind":"uml.class","name":"Second"},
        {"id":"set","kind":"uml.generalization_set","name":"Variants","properties":{"generalization":[{"$ref":"first-general"},{"$ref":"second-general"}]}}
    ],"relations":[
        {"id":"first-general","kind":"uml.generalization","from":"first","to":"general"},
        {"id":"second-general","kind":"uml.generalization","from":"second","to":"general"}
    ]});
    for child in ["general", "other", "first", "second", "set"] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":"root","to":child}));
    }
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
}
fn author(project: &mut Project, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "set")
        .ok_or("set")?
        .properties
        .insert("generalization".into(), value);
    Ok(())
}
fn target(project: &mut Project, id: &str, target: &str) -> Result {
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == id)
        .ok_or("relation")?
        .to = target.into();
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
fn same_general_empty_and_singleton_sets_satisfy_rule_in_rooted_and_draft_models() -> Result {
    for rooted in [true, false] {
        for values in [
            json!([]),
            json!([{"$ref":"first-general"}]),
            json!([{"$ref":"first-general"},{"$ref":"second-general"}]),
        ] {
            let mut project = document(rooted)?;
            author(&mut project, values)?;
            assert!(issues(&project)?.is_empty());
            for format in [Format::Json, Format::Protobuf] {
                assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
            }
        }
    }
    Ok(())
}
#[test]
fn distinct_generals_violate_even_if_related_by_generalization() -> Result {
    let mut project = document(true)?;
    target(&mut project, "second-general", "other")?;
    project.relations.push(serde_json::from_value(
        json!({"id":"general-parent","kind":"uml.generalization","from":"other","to":"general"}),
    )?);
    expect(&project, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
    }
    Ok(())
}
#[test]
fn external_members_do_not_hide_known_conflicts_in_either_order() -> Result {
    for reverse in [true, false] {
        let mut project = document(true)?;
        let mut values = vec![
            json!({"href":"external.uml#Generalization"}),
            json!({"$ref":"first-general"}),
            json!({"$ref":"second-general"}),
        ];
        if reverse {
            values.reverse();
        }
        author(&mut project, json!(values))?;
        expect(&project, UNKNOWN)?;
        target(&mut project, "second-general", "other")?;
        expect(&project, ERROR)?;
    }
    Ok(())
}
#[test]
fn malformed_members_dominate_unknown_and_false() -> Result {
    for value in [
        Value::Null,
        json!(42),
        json!([{"$ref":"first-general"},{"$ref":"first-general"}]),
        json!([{"href":"external.uml#Generalization"},{"$ref":"missing"}]),
        json!([{"href":"external.uml#Generalization"},{"$ref":"general"}]),
        json!([{"href":42}]),
    ] {
        let mut project = document(true)?;
        target(&mut project, "second-general", "other")?;
        author(&mut project, value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn missing_membership_is_unknown_and_inverse_storage_does_not_invent_completeness() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "set")
        .ok_or("set")?
        .properties
        .remove("generalization");
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "first-general")
        .ok_or("relation")?
        .properties
        .insert("generalizationSet".into(), json!([{"$ref":"set"}]));
    expect(&project, UNKNOWN)?;
    for format in [Format::Json, Format::Protobuf] {
        expect(&decode(&encode(&project, format)?, format)?, UNKNOWN)?;
    }
    Ok(())
}
#[test]
fn generalization_objects_require_adapter_instead_of_silently_becoming_empty() -> Result {
    for kind in ["uml.generalization", "uml.unresolved_generalization"] {
        let mut project = document(true)?;
        project.elements.push(serde_json::from_value(json!({"id":"object","kind":kind,"name":"Object","properties":{"general":{"$ref":"general"}}}))?);
        author(&mut project, json!([{"$ref":"object"}]))?;
        expect(&project, UNKNOWN)?;
    }
    Ok(())
}
#[test]
fn unknown_general_metaclass_retains_identity_evidence_without_claiming_type_resolution() -> Result
{
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "general")
        .ok_or("general")?
        .kind = "uml.unresolved_classifier".into();
    expect(&project, UNKNOWN)?;
    target(&mut project, "second-general", "other")?;
    expect(&project, ERROR)?;
    target(&mut project, "second-general", "root")?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn membership_authoring_is_read_only_and_reopened_snapshots_can_be_repaired() -> Result {
    let mut project = document(true)?;
    target(&mut project, "second-general", "other")?;
    author(&mut project, json!([{"$ref":"first-general"}]))?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "set".into(),
            path: "properties.generalization".into(),
            value: json!([{"$ref":"first-general"},{"$ref":"second-general"}]),
        },
        1,
    )
    .expect_err("different generals");
    assert_eq!(error, "property is read-only");
    assert_eq!(before, encode(&project, Format::Json)?);
    author(
        &mut project,
        json!([{"$ref":"first-general"},{"$ref":"second-general"}]),
    )?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format)?, format)?;
        expect(&reopened, ERROR)?;
        let mut repaired = reopened;
        author(&mut repaired, json!([{"$ref":"first-general"}]))?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}

#[test]
fn authored_general_cannot_override_canonical_relationship_target() -> Result {
    for value in [
        json!({"$ref":"other"}),
        json!({"$ref":"missing"}),
        json!(42),
    ] {
        let mut project = document(true)?;
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "first-general")
            .ok_or("relation")?
            .properties
            .insert("general".into(), value);
        expect(&project, INVALID)?;
    }
    let mut project = document(true)?;
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "first-general")
        .ok_or("relation")?
        .properties
        .insert("general".into(), json!({"$ref":"general"}));
    assert!(issues(&project)?.is_empty());
    Ok(())
}

#[test]
fn unresolved_authored_general_retains_local_identity_contradictions() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "other")
        .ok_or("other")?
        .kind = "uml.unresolved_classifier".into();
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "first-general")
        .ok_or("relation")?
        .properties
        .insert("general".into(), json!({"$ref":"other"}));
    expect(&project, INVALID)?;
    for format in [Format::Json, Format::Protobuf] {
        expect(&decode(&encode(&project, format)?, format)?, INVALID)?;
    }
    target(&mut project, "first-general", "other")?;
    author(&mut project, json!([{"$ref":"first-general"}]))?;
    expect(&project, UNKNOWN)?;
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "first-general")
        .ok_or("relation")?
        .properties
        .insert("general".into(), json!({"href":"external.uml#General"}));
    expect(&project, UNKNOWN)?;
    Ok(())
}
