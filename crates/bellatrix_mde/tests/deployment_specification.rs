use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "DeploymentSpecification-deployment_target:";

fn document(location_kind: &str, rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project", "name":"Deployment specification", "root_model":"root",
    "elements":[
        {"id":"root", "kind":"uml.model", "name":"Root"},
        {"id":"location", "kind":location_kind, "name":"Location"},
        {"id":"artifact", "kind":"uml.artifact", "name":"Artifact"},
        {"id":"specification", "kind":"uml.deployment_specification", "name":"Configuration", "properties":{"visibility":null}}
    ],
    "relations":[
        {"id":"deployment", "kind":"uml.deployment", "from":"location", "to":"artifact", "properties":{"visibility":null}},
        {"id":"root-location", "kind":"owns", "from":"root", "to":"location"},
        {"id":"root-artifact", "kind":"owns", "from":"root", "to":"artifact"},
        {"id":"location-deployment", "kind":"owns", "from":"location", "to":"deployment"},
        {"id":"deployment-specification", "kind":"owns", "from":"deployment", "to":"specification"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
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
        .is_some_and(|message| message.starts_with(RULE))
}

#[test]
fn given_configured_deployment_then_only_execution_environment_location_is_valid() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.execution_environment", "uml.node", "uml.device"] {
            let project = document(kind, rooted)?;
            let issues = diagnostics(&project)?;
            assert_eq!(
                issues
                    .iter()
                    .filter(|issue| relevant(issue)
                        && issue["code"] == "UML_DEPLOYMENT_SPECIFICATION_TARGET")
                    .count(),
                usize::from(kind != "uml.execution_environment"),
                "{kind}: {issues:#?}"
            );
            if kind == "uml.execution_environment" {
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
fn given_no_deployment_then_configuration_is_vacuously_valid() -> Result {
    let mut project = document("uml.node", true)?;
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "deployment-specification")
        .ok_or("owns")?
        .from = "root".into();
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "specification")
        .ok_or("specification")?
        .properties
        .insert("visibility".into(), json!("public"));
    assert!(!diagnostics(&project)?.iter().any(relevant));
    Ok(())
}

#[test]
fn given_unresolved_owner_or_inverse_then_constraint_is_unverified() -> Result {
    let project = document("uml.unknown_target", true)?;
    let issues = diagnostics(&project)?;
    assert!(
        issues
            .iter()
            .any(|issue| relevant(issue) && issue["severity"] == "warning"),
        "{issues:#?}"
    );
    for inverse in ["deployment", "location"] {
        let mut project = document("uml.execution_environment", true)?;
        if inverse == "deployment" {
            project
                .elements
                .iter_mut()
                .find(|element| element.id == "specification")
                .ok_or("spec")?
                .properties
                .insert(inverse.into(), json!({"href":"external.uml#deployment"}));
        } else {
            project
                .relations
                .iter_mut()
                .find(|relation| relation.id == "deployment")
                .ok_or("deployment")?
                .properties
                .insert(inverse.into(), json!({"href":"external.uml#location"}));
        }
        let issues = diagnostics(&project)?;
        assert!(
            issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "warning"),
            "{issues:#?}"
        );
    }
    Ok(())
}

#[test]
fn given_forged_inverse_or_multiple_owners_then_identity_error_cannot_override_canonical_location()
-> Result {
    for value in [
        Value::Null,
        json!({"$ref":"missing"}),
        json!({"$ref":"root"}),
    ] {
        let mut project = document("uml.execution_environment", true)?;
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "deployment")
            .ok_or("deployment")?
            .properties
            .insert("location".into(), value);
        let issues = diagnostics(&project)?;
        assert!(
            issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "error"),
            "{issues:#?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue["code"] == "UML_DEPLOYMENT_SPECIFICATION_TARGET"),
            "{issues:#?}"
        );
    }
    let mut project = document("uml.execution_environment", true)?;
    project.relations.push(serde_json::from_value(
        json!({"id":"other-owner", "kind":"owns", "from":"root", "to":"deployment"}),
    )?);
    let issues = diagnostics(&project)?;
    assert!(
        issues.iter().any(|issue| issue["severity"] == "error"),
        "{issues:#?}"
    );
    Ok(())
}

#[test]
fn when_configured_deployment_moves_to_wrong_target_then_reparent_rolls_back_and_draft_can_be_repaired()
-> Result {
    let mut valid = document("uml.execution_environment", true)?;
    valid.elements.push(serde_json::from_value(
        json!({"id":"other", "kind":"uml.node", "name":"Node"}),
    )?);
    valid.relations.push(serde_json::from_value(
        json!({"id":"root-other", "kind":"owns", "from":"root", "to":"other"}),
    )?);
    let before = encode(&valid, Format::Json)?;
    let error = apply(
        &valid,
        &Command::SetOwner {
            element_id: "deployment".into(),
            owner_id: Some("other".into()),
        },
        1,
    )
    .expect_err("configured target must be ExecutionEnvironment");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&valid, Format::Json)?, before);
    let mut invalid = document("uml.node", true)?;
    invalid.elements.push(serde_json::from_value(
        json!({"id":"other", "kind":"uml.execution_environment", "name":"Environment"}),
    )?);
    invalid.relations.push(serde_json::from_value(
        json!({"id":"root-other", "kind":"owns", "from":"root", "to":"other"}),
    )?);
    let repaired = apply(
        &invalid,
        &Command::SetOwner {
            element_id: "deployment".into(),
            owner_id: Some("other".into()),
        },
        2,
    )?;
    assert!(!diagnostics(&repaired)?.iter().any(relevant));
    Ok(())
}

#[test]
fn given_stored_inverse_then_it_must_match_ownership_even_when_a_neighbor_type_is_unknown() -> Result
{
    for kind in ["uml.execution_environment", "uml.unknown_target"] {
        let mut project = document(kind, true)?;
        project.elements.push(serde_json::from_value(
            json!({"id":"other", "kind":"uml.execution_environment", "name":"Other"}),
        )?);
        project.relations.push(serde_json::from_value(
            json!({"id":"root-other", "kind":"owns", "from":"root", "to":"other"}),
        )?);
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "deployment")
            .ok_or("deployment")?
            .properties
            .insert("location".into(), json!({"$ref":"other"}));
        let issues = diagnostics(&project)?;
        assert!(
            issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "error"),
            "{issues:#?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "warning"),
            "{issues:#?}"
        );
    }
    let mut project = document("uml.execution_environment", true)?;
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "deployment")
        .ok_or("deployment")?
        .properties
        .insert("location".into(), json!({"$ref":"location"}));
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "specification")
        .ok_or("specification")?
        .properties
        .insert("deployment".into(), json!({"$ref":"deployment"}));
    assert!(!diagnostics(&project)?.iter().any(relevant));
    Ok(())
}

#[test]
fn given_external_inverses_then_known_wrong_targets_remain_errors() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.node", "uml.device"] {
            for inverse in ["deployment", "location", "both"] {
                let mut project = document(kind, rooted)?;
                if inverse != "location" {
                    project
                        .elements
                        .iter_mut()
                        .find(|element| element.id == "specification")
                        .ok_or("specification")?
                        .properties
                        .insert(
                            "deployment".into(),
                            json!({"href":"external.uml#deployment"}),
                        );
                }
                if inverse != "deployment" {
                    project
                        .relations
                        .iter_mut()
                        .find(|relation| relation.id == "deployment")
                        .ok_or("deployment")?
                        .properties
                        .insert("location".into(), json!({"href":"external.uml#location"}));
                }
                let issues = diagnostics(&project)?;
                assert!(
                    issues.iter().any(|issue| relevant(issue)
                        && issue["code"] == "UML_DEPLOYMENT_SPECIFICATION_TARGET"
                        && issue["severity"] == "error"),
                    "{kind}/{inverse}: {issues:#?}"
                );
                assert!(
                    !issues
                        .iter()
                        .any(|issue| relevant(issue) && issue["severity"] == "warning"),
                    "{issues:#?}"
                );
                for format in [Format::Json, Format::Protobuf] {
                    assert_eq!(
                        diagnostics(&decode(&encode(&project, format)?, format)?)?,
                        issues
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn given_external_deployment_inverse_then_invalid_location_still_dominates() -> Result {
    for value in [
        Value::Null,
        json!(42),
        json!({"$ref":"missing"}),
        json!({"$ref":"root"}),
    ] {
        let mut project = document("uml.execution_environment", true)?;
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "specification")
            .ok_or("specification")?
            .properties
            .insert(
                "deployment".into(),
                json!({"href":"external.uml#deployment"}),
            );
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "deployment")
            .ok_or("deployment")?
            .properties
            .insert("location".into(), value);
        let issues = diagnostics(&project)?;
        assert!(
            issues.iter().any(|issue| relevant(issue)
                && issue["code"] == "MDE_DEPLOYMENT_SPECIFICATION_OWNERSHIP"
                && issue["severity"] == "error"),
            "{issues:#?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "warning"),
            "{issues:#?}"
        );
    }
    Ok(())
}

#[test]
fn given_external_inverse_when_reparented_to_node_then_command_rejects_atomically() -> Result {
    for inverse in ["deployment", "location", "both"] {
        let mut project = document("uml.execution_environment", true)?;
        if inverse != "location" {
            project
                .elements
                .iter_mut()
                .find(|element| element.id == "specification")
                .ok_or("specification")?
                .properties
                .insert(
                    "deployment".into(),
                    json!({"href":"external.uml#deployment"}),
                );
        }
        if inverse != "deployment" {
            project
                .relations
                .iter_mut()
                .find(|relation| relation.id == "deployment")
                .ok_or("deployment")?
                .properties
                .insert("location".into(), json!({"href":"external.uml#location"}));
        }
        let issues = diagnostics(&project)?;
        assert!(
            issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "warning"),
            "{issues:#?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == "error"),
            "{issues:#?}"
        );
        project.elements.push(serde_json::from_value(
            json!({"id":"other", "kind":"uml.node", "name":"Node"}),
        )?);
        project.relations.push(serde_json::from_value(
            json!({"id":"root-other", "kind":"owns", "from":"root", "to":"other"}),
        )?);
        let before = encode(&project, Format::Json)?;
        let error = apply(
            &project,
            &Command::SetOwner {
                element_id: "deployment".into(),
                owner_id: Some("other".into()),
            },
            1,
        )
        .expect_err("external inverse must not hide invalid canonical location");
        assert!(error.contains(RULE), "{error}");
        assert_eq!(encode(&project, Format::Json)?, before);
    }
    Ok(())
}

#[test]
fn given_external_inverses_then_missing_and_unknown_canonical_locations_stay_distinct() -> Result {
    for kind in ["uml.model", "uml.unknown_target"] {
        let mut project = document(kind, true)?;
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "specification")
            .ok_or("specification")?
            .properties
            .insert(
                "deployment".into(),
                json!({"href":"external.uml#deployment"}),
            );
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "deployment")
            .ok_or("deployment")?
            .properties
            .insert("location".into(), json!({"href":"external.uml#location"}));
        let issues = diagnostics(&project)?;
        let expected = if kind == "uml.model" {
            "error"
        } else {
            "warning"
        };
        assert!(
            issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] == expected),
            "{kind}: {issues:#?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| relevant(issue) && issue["severity"] != expected),
            "{kind}: {issues:#?}"
        );
    }
    Ok(())
}
