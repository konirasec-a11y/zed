use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_protocol::Command;
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> Result<Value> {
    let response = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&response)?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> Result<Vec<u8>> {
    Ok(encode_model_document(&envelope(model), format)?)
}

fn import_document(model: &Value) -> Result<Value> {
    decode(&serde_json::to_vec(model)?, "json")
}

fn apply(model: &Value, command: &Command, revision: u64) -> std::result::Result<Value, String> {
    let document = prepare_model_document(&envelope(model))?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
        false,
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json").map_err(|error| error.to_string())
}

const RULE: &str = "DeploymentSpecification-deployment_target:";

fn document(location_kind: &str, rooted: bool) -> Result<Value> {
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
    import_document(&value)
}

fn diagnostics(project: &Value) -> Result<Vec<Value>> {
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
            for format in ["json", "protobuf"] {
                let reopened = decode(&encode(&project, format)?, format)?;
                assert_eq!(diagnostics(&reopened)?, issues);
            }
        }
    }
    Ok(())
}

#[test]
fn when_configured_deployment_moves_to_wrong_target_then_reparent_rolls_back_and_draft_can_be_repaired()
-> Result {
    let mut valid = document("uml.execution_environment", true)?;
    valid["elements"].as_array_mut().ok_or("elements")?.push(
        json!({"id":"other", "kind":"uml.node", "name":"Node"}),
    );
    valid["relations"].as_array_mut().ok_or("relations")?.push(
        json!({"id":"root-other", "kind":"owns", "from":"root", "to":"other"}),
    );
    valid = import_document(&valid)?;
    let before = encode(&valid, "json")?;
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
    assert_eq!(encode(&valid, "json")?, before);
    let mut invalid = document("uml.node", true)?;
    invalid["elements"].as_array_mut().ok_or("elements")?.push(
        json!({"id":"other", "kind":"uml.execution_environment", "name":"Environment"}),
    );
    invalid["relations"].as_array_mut().ok_or("relations")?.push(
        json!({"id":"root-other", "kind":"owns", "from":"root", "to":"other"}),
    );
    invalid = import_document(&invalid)?;
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
fn given_external_inverses_then_known_wrong_targets_remain_errors() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.node", "uml.device"] {
            for inverse in ["deployment", "location", "both"] {
                let mut project = document(kind, rooted)?;
                if inverse != "location" {
                    project["elements"]
                        .as_array_mut()
                        .ok_or("elements")?
                        .iter_mut()
                        .find(|element| element["id"] == "specification")
                        .ok_or("specification")?["properties"]["deployment"] =
                        json!({"href":"external.uml#deployment"});
                }
                if inverse != "deployment" {
                    project["relations"]
                        .as_array_mut()
                        .ok_or("relations")?
                        .iter_mut()
                        .find(|relation| relation["id"] == "deployment")
                        .ok_or("deployment")?["properties"]["location"] =
                        json!({"href":"external.uml#location"});
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
                for format in ["json", "protobuf"] {
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
fn given_external_inverse_when_reparented_to_node_then_command_rejects_atomically() -> Result {
    for inverse in ["deployment", "location", "both"] {
        let mut project = document("uml.execution_environment", true)?;
        if inverse != "location" {
            project["elements"]
                .as_array_mut()
                .ok_or("elements")?
                .iter_mut()
                .find(|element| element["id"] == "specification")
                .ok_or("specification")?["properties"]["deployment"] =
                json!({"href":"external.uml#deployment"});
        }
        if inverse != "deployment" {
            project["relations"]
                .as_array_mut()
                .ok_or("relations")?
                .iter_mut()
                .find(|relation| relation["id"] == "deployment")
                .ok_or("deployment")?["properties"]["location"] =
                json!({"href":"external.uml#location"});
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
        project["elements"].as_array_mut().ok_or("elements")?.push(
            json!({"id":"other", "kind":"uml.node", "name":"Node"}),
        );
        project["relations"].as_array_mut().ok_or("relations")?.push(
            json!({"id":"root-other", "kind":"owns", "from":"root", "to":"other"}),
        );
        project = import_document(&project)?;
        let before = encode(&project, "json")?;
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
        assert_eq!(encode(&project, "json")?, before);
    }
    Ok(())
}
