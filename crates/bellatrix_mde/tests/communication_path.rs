use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode,
    uml::{associations, references::ReferenceIndex, root::Model},
    validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "CommunicationPath-association_ends:";
const ERROR: &str = "UML_COMMUNICATION_PATH_ASSOCIATION_ENDS";
const INVALID: &str = "MDE_COMMUNICATION_PATH_ASSOCIATION_ENDS_INVALID";
const UNKNOWN: &str = "MDE_COMMUNICATION_PATH_ASSOCIATION_ENDS_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Communication path","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"left","kind":"uml.node","name":"Left"},
            {"id":"right","kind":"uml.device","name":"Right"},
            {"id":"class","kind":"uml.class","name":"Class"},
            {"id":"path","kind":"uml.communication_path","name":"Path","properties":{"memberEnd":[{"$ref":"end-left"},{"$ref":"end-right"}]}},
            {"id":"end-left","kind":"uml.attribute","name":"left","properties":{"type":{"$ref":"left"}}},
            {"id":"end-right","kind":"uml.attribute","name":"right","properties":{"type":{"$ref":"right"}}}
        ],"relations":[]});
    for (owner, child) in [
        ("root", "left"),
        ("root", "right"),
        ("root", "class"),
        ("root", "path"),
        ("path", "end-left"),
        ("path", "end-right"),
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
fn diagnostics(project: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}
fn relevant(issue: &&Value) -> bool {
    issue["subject_id"] == "path"
        && issue["message"]
            .as_str()
            .is_some_and(|message| message.starts_with(RULE))
}
fn expect(project: &Project, code: &str) -> Result {
    let issues = diagnostics(project)?;
    let specific = issues.iter().filter(relevant).collect::<Vec<_>>();
    assert_eq!(specific.len(), 1, "{issues:#?}");
    assert_eq!(specific[0]["code"], code, "{issues:#?}");
    assert_eq!(
        specific[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}

#[test]
fn given_canonical_communication_path_then_deployment_target_end_types_are_valid() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.node", "uml.device", "uml.execution_environment"] {
            let mut project = document(rooted)?;
            project
                .elements
                .iter_mut()
                .find(|element| element.id == "left")
                .ok_or("left")?
                .kind = kind.into();
            let issues = diagnostics(&project)?;
            assert_eq!(issues.iter().filter(relevant).count(), 0, "{issues:#?}");
            assert!(
                !issues.iter().any(|issue| issue["subject_id"] == "path"
                    && (issue["severity"] == "error"
                        || issue["code"] == "MDE_ASSOCIATION_UNSUPPORTED")),
                "{issues:#?}"
            );
            // The legacy authoring adapter still lacks this metaclass; this
            // kernel-only implementation does not claim to add a visual tool.
            assert!(
                issues.iter().any(|issue| issue["subject_id"] == "path"
                    && issue["code"] == "MDE_UML_METACLASS_UNSUPPORTED"),
                "{issues:#?}"
            );
            assert_eq!(
                associations::end_types(
                    &ReferenceIndex::new(Model::from_project(&project)),
                    "path"
                )?
                .len(),
                2
            );
            for format in [Format::Json, Format::Protobuf] {
                assert_eq!(
                    diagnostics(&decode(&encode(&project, format)?, format)?)?,
                    issues
                );
            }
        }
    }
    Ok(())
}

#[test]
fn when_end_type_becomes_class_then_edit_rejects_atomically_and_repair_succeeds() -> Result {
    let project = document(true)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "end-left".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"class"}),
        },
        1,
    )
    .expect_err("Class is not a DeploymentTarget");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&project, Format::Json)?, before);
    let mut invalid = project;
    author(&mut invalid, "end-left", "type", json!({"$ref":"class"}))?;
    expect(&invalid, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "end-left".into(),
                path: "properties.type".into(),
                value: json!({"$ref":"left"}),
            },
            2,
        )?;
        assert_eq!(diagnostics(&repaired)?.iter().filter(relevant).count(), 0);
    }
    Ok(())
}

#[test]
fn given_external_member_or_type_then_complete_query_stays_unknown_but_known_failure_dominates()
-> Result {
    for external_member in [true, false] {
        for reverse in [true, false] {
            let mut project = document(true)?;
            if external_member {
                let mut values = vec![
                    json!({"$ref":"end-left"}),
                    json!({"href":"external.uml#end"}),
                ];
                if reverse {
                    values.reverse();
                }
                author(&mut project, "path", "memberEnd", json!(values))?;
            } else {
                author(
                    &mut project,
                    "end-right",
                    "type",
                    json!({"href":"external.uml#type"}),
                )?;
                if reverse {
                    author(
                        &mut project,
                        "path",
                        "memberEnd",
                        json!([{"$ref":"end-right"},{"$ref":"end-left"}]),
                    )?;
                }
            }
            expect(&project, UNKNOWN)?;
            assert!(
                associations::end_types(
                    &ReferenceIndex::new(Model::from_project(&project)),
                    "path"
                )
                .is_err()
            );
            author(&mut project, "end-left", "type", json!({"$ref":"class"}))?;
            expect(&project, ERROR)?;
            assert!(
                associations::end_types(
                    &ReferenceIndex::new(Model::from_project(&project)),
                    "path"
                )
                .is_err()
            );
            author(&mut project, "end-left", "type", json!({"$ref":"missing"}))?;
            expect(&project, INVALID)?;
        }
    }
    Ok(())
}

#[test]
fn given_malformed_members_or_types_then_invalid_dominates_unknown_and_false() -> Result {
    for malformed in [
        json!({"href":""}),
        json!({"href":"external.uml#type","$ref":"class"}),
        json!(42),
        json!({"$ref":"missing"}),
        json!({"$ref":"end-left"}),
    ] {
        for neighbor in [json!({"href":"external.uml#type"}), json!({"$ref":"class"})] {
            let mut project = document(true)?;
            author(&mut project, "end-left", "type", malformed.clone())?;
            author(&mut project, "end-right", "type", neighbor)?;
            expect(&project, INVALID)?;
        }
    }
    for members in [
        Value::Null,
        json!({"$ref":"end-left"}),
        json!([]),
        json!([{"$ref":"end-left"},{"$ref":"end-left"}]),
        json!([{"href":"external.uml#end"},{"$ref":"missing"}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "path", "memberEnd", members)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}

#[test]
fn given_unmaterialized_roles_or_unknown_type_then_no_endpoint_or_cache_is_used_as_proof() -> Result
{
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "path")
        .ok_or("path")?
        .properties
        .remove("memberEnd");
    author(
        &mut project,
        "path",
        "endType",
        json!([{"$ref":"left"},{"$ref":"right"}]),
    )?;
    expect(&project, UNKNOWN)?;
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "left")
        .ok_or("left")?
        .kind = "uml.unknown_node".into();
    expect(&project, UNKNOWN)?;
    let mut project = document(true)?;
    author(&mut project, "end-left", "type", json!("Node"))?;
    expect(&project, UNKNOWN)?;
    let mut project = document(true)?;
    author(
        &mut project,
        "path",
        "endType",
        json!([{"$ref":"left"},{"$ref":"right"}]),
    )?;
    author(&mut project, "end-left", "type", json!({"$ref":"class"}))?;
    expect(&project, ERROR)?;
    Ok(())
}

#[test]
fn given_absent_optional_type_then_inherited_typed_end_rule_still_rejects_it() -> Result {
    for absent in [true, false] {
        let mut project = document(true)?;
        if absent {
            project
                .elements
                .iter_mut()
                .find(|element| element.id == "end-left")
                .ok_or("end")?
                .properties
                .remove("type");
        } else {
            author(&mut project, "end-left", "type", Value::Null)?;
        }
        let issues = diagnostics(&project)?;
        assert_eq!(issues.iter().filter(relevant).count(), 0, "{issues:#?}");
        assert!(
            issues.iter().any(|issue| issue["subject_id"] == "path"
                && issue["severity"] == "error"
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("ends_must_be_typed"))),
            "{issues:#?}"
        );
    }
    Ok(())
}

#[test]
fn given_nary_path_then_all_types_are_checked_and_duplicate_types_remain_a_set() -> Result {
    let mut project = document(true)?;
    project.elements.push(serde_json::from_value(json!({"id":"third","kind":"uml.attribute","name":"third","properties":{"type":{"$ref":"left"}}}))?);
    project.relations.push(serde_json::from_value(
        json!({"id":"owns-third","kind":"owns","from":"path","to":"third"}),
    )?);
    author(
        &mut project,
        "path",
        "memberEnd",
        json!([{"$ref":"end-left"},{"$ref":"end-right"},{"$ref":"third"}]),
    )?;
    assert_eq!(diagnostics(&project)?.iter().filter(relevant).count(), 0);
    assert_eq!(
        associations::end_types(&ReferenceIndex::new(Model::from_project(&project)), "path")?.len(),
        2
    );
    author(&mut project, "third", "type", json!({"$ref":"class"}))?;
    expect(&project, ERROR)?;
    Ok(())
}
