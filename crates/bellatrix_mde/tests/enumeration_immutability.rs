use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "UML_ENUMERATION_IMMUTABLE";

fn document(read_only: Option<Value>, rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project", "name":"Enumeration immutability", "root_model":"root",
    "elements":[
        {"id":"root", "kind":"uml.model", "name":"Root"},
        {"id":"enumeration", "kind":"uml.enumeration", "name":"Color"},
        {"id":"attribute", "kind":"uml.attribute", "name":"rgb"}
    ],
    "relations":[
        {"id":"root-enumeration", "kind":"owns", "from":"root", "to":"enumeration"},
        {"id":"enumeration-attribute", "kind":"owns", "from":"enumeration", "to":"attribute"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    let mut project: Project = serde_json::from_value(value)?;
    if let Some(value) = read_only {
        author(&mut project, "attribute", "isReadOnly", value)?;
    }
    Ok(project)
}

fn author(project: &mut Project, id: &str, property: &str, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(property.into(), value);
    Ok(())
}

fn diagnostics(project: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1, "model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}

fn relevant(issue: &Value) -> bool {
    issue["message"]
        .as_str()
        .is_some_and(|message| message.starts_with("Enumeration-immutable:"))
}

#[test]
fn given_owned_attributes_then_only_true_is_immutable_in_rooted_and_rootless_models() -> Result {
    for rooted in [true, false] {
        for value in [None, Some(json!(false)), Some(json!(true))] {
            let project = document(value.clone(), rooted)?;
            let issues = diagnostics(&project)?;
            let immutable = value == Some(json!(true));
            assert_eq!(
                issues.iter().filter(|issue| issue["code"] == RULE).count(),
                usize::from(!immutable),
                "{issues:#?}"
            );
            if immutable {
                assert!(!issues.iter().any(relevant), "{issues:#?}");
            }
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&project, format)?, format)?;
                assert_eq!(diagnostics(&reopened)?, issues);
            }
        }
    }
    Ok(())
}

#[test]
fn given_nonboolean_read_only_then_representation_error_is_not_normative_violation() -> Result {
    for value in [
        Value::Null,
        json!("true"),
        json!(1),
        json!({"href":"external.uml#flag"}),
    ] {
        let project = document(Some(value), true)?;
        let issues = diagnostics(&project)?;
        assert!(
            !issues.iter().any(|issue| issue["code"] == RULE),
            "{issues:#?}"
        );
        assert!(
            issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "error"),
            "{issues:#?}"
        );
    }
    Ok(())
}

#[test]
fn given_external_owned_attribute_then_only_local_mutability_decides_the_universal() -> Result {
    for mutable in [true, false] {
        let mut project = document(Some(json!(!mutable)), true)?;
        author(
            &mut project,
            "enumeration",
            "ownedAttribute",
            json!([{"href":"external.uml#attribute"}, {"$ref":"attribute"}]),
        )?;
        let issues = diagnostics(&project)?;
        assert_eq!(
            issues.iter().any(|issue| issue["code"] == RULE),
            mutable,
            "{issues:#?}"
        );
        assert_eq!(
            issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "warning"),
            !mutable,
            "{issues:#?}"
        );
    }
    Ok(())
}

#[test]
fn given_mutable_inherited_attribute_or_other_owned_member_then_direct_owned_attribute_rule_does_not_apply()
-> Result {
    let mut project = document(Some(json!(false)), true)?;
    project.elements.push(serde_json::from_value(
        json!({"id":"parent", "kind":"uml.enumeration", "name":"Parent"}),
    )?);
    project.relations.push(serde_json::from_value(
        json!({"id":"root-parent", "kind":"owns", "from":"root", "to":"parent"}),
    )?);
    project.relations.push(serde_json::from_value(json!({"id":"generalization", "kind":"uml.generalization", "from":"enumeration", "to":"parent"}))?);
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "enumeration-attribute")
        .ok_or("owns")?
        .from = "parent".into();
    let issues = diagnostics(&project)?;
    assert!(
        !issues
            .iter()
            .any(|issue| relevant(issue) && issue["subject_id"] == "enumeration"),
        "{issues:#?}"
    );
    assert!(
        issues
            .iter()
            .any(|issue| issue["code"] == RULE && issue["subject_id"] == "parent"),
        "{issues:#?}"
    );
    let mut comment = document(None, true)?;
    comment
        .elements
        .iter_mut()
        .find(|element| element.id == "attribute")
        .ok_or("child")?
        .kind = "uml.comment".into();
    assert!(!diagnostics(&comment)?.iter().any(relevant));
    let mut data_type = document(None, true)?;
    data_type
        .elements
        .iter_mut()
        .find(|element| element.id == "enumeration")
        .ok_or("owner")?
        .kind = "uml.data_type".into();
    assert!(!diagnostics(&data_type)?.iter().any(relevant));
    Ok(())
}

#[test]
fn when_attribute_becomes_mutable_then_edit_rolls_back_and_invalid_draft_can_be_repaired() -> Result
{
    let valid = document(Some(json!(true)), true)?;
    let before = encode(&valid, Format::Json)?;
    let error = apply(
        &valid,
        &Command::SetProperty {
            element_id: "attribute".into(),
            path: "properties.isReadOnly".into(),
            value: json!(false),
        },
        1,
    )
    .expect_err("Enumeration attribute must remain immutable");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&valid, Format::Json)?, before);
    let invalid = document(None, true)?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "attribute".into(),
            path: "properties.isReadOnly".into(),
            value: json!(true),
        },
        2,
    )?;
    assert!(!diagnostics(&repaired)?.iter().any(relevant));
    Ok(())
}

#[test]
fn given_invalid_collection_then_local_mutable_attribute_cannot_hide_identity_errors() -> Result {
    for invalid in [
        Value::Null,
        json!({"$ref":"attribute"}),
        json!({"$ref":"missing"}),
    ] {
        let mut project = document(None, true)?;
        author(
            &mut project,
            "enumeration",
            "ownedAttribute",
            json!([{"href":"external.uml#attribute"}, {"$ref":"attribute"}, invalid]),
        )?;
        let issues = diagnostics(&project)?;
        assert!(
            !issues.iter().any(|issue| issue["code"] == RULE),
            "{issues:#?}"
        );
        assert!(
            issues.iter().any(|issue| issue["severity"] == "error"),
            "{issues:#?}"
        );
    }
    Ok(())
}

#[test]
fn given_empty_enumeration_then_immutable_is_satisfied_unless_an_external_attribute_is_unresolved()
-> Result {
    let mut project = document(None, true)?;
    project.elements.retain(|element| element.id != "attribute");
    project
        .relations
        .retain(|relation| relation.id != "enumeration-attribute");
    assert!(!diagnostics(&project)?.iter().any(relevant));
    author(
        &mut project,
        "enumeration",
        "ownedAttribute",
        json!([{"href":"external.uml#attribute"}]),
    )?;
    let issues = diagnostics(&project)?;
    assert!(
        issues
            .iter()
            .any(|issue| relevant(issue) && issue["severity"] == "warning"),
        "{issues:#?}"
    );
    assert!(
        !issues.iter().any(|issue| issue["code"] == RULE),
        "{issues:#?}"
    );
    Ok(())
}

#[test]
fn given_unrelated_owned_roles_then_external_or_invalid_values_do_not_contaminate_immutable()
-> Result {
    for role in ["ownedLiteral", "ownedOperation"] {
        for value in [
            json!([{"href":"external.uml#member"}]),
            json!([{"$ref":"missing"}]),
            json!([null]),
            json!("legacy"),
        ] {
            for with_attribute in [true, false] {
                let mut project = document(Some(json!(true)), true)?;
                if !with_attribute {
                    project.elements.retain(|element| element.id != "attribute");
                    project
                        .relations
                        .retain(|relation| relation.id != "enumeration-attribute");
                    author(&mut project, "enumeration", "ownedAttribute", json!([]))?;
                }
                if role == "ownedOperation" {
                    project.elements.push(serde_json::from_value(
                        json!({"id":"operation", "kind":"uml.operation", "name":"Operation"}),
                    )?);
                    project.relations.push(serde_json::from_value(json!({"id":"owns-operation", "kind":"owns", "from":"enumeration", "to":"operation"}))?);
                }
                author(&mut project, "enumeration", role, value.clone())?;
                let issues = diagnostics(&project)?;
                assert!(!issues.iter().any(relevant), "{role}: {value}: {issues:#?}");
                let severity = if value[0].get("href").is_some() {
                    "warning"
                } else {
                    "error"
                };
                assert!(
                    issues
                        .iter()
                        .any(|issue| issue["severity"] == severity && !relevant(issue)),
                    "{role}: {value}: {issues:#?}"
                );
            }
        }
    }
    Ok(())
}
