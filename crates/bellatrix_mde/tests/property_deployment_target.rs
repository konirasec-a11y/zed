use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "Property-deployment_target:";
const ERROR: &str = "UML_PROPERTY_DEPLOYMENT_TARGET";
const INVALID: &str = "MDE_PROPERTY_DEPLOYMENT_TARGET_INVALID";
const UNKNOWN: &str = "MDE_PROPERTY_DEPLOYMENT_TARGET_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Property deployment","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"node","kind":"uml.node","name":"Node"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"artifact","kind":"uml.artifact","name":"Artifact"},
        {"id":"property","kind":"uml.attribute","name":"Part","properties":{"type":{"$ref":"class"},"aggregation":"composite"}}
    ],"relations":[
        {"id":"deployment","kind":"uml.deployment","from":"property","to":"artifact","properties":{"visibility":null}}
    ]});
    for (owner, child) in [
        ("root", "node"),
        ("root", "class"),
        ("root", "artifact"),
        ("node", "property"),
        ("property", "deployment"),
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
    if let Some(element) = project.elements.iter_mut().find(|element| element.id == id) {
        element.properties.insert(key.into(), value);
    } else {
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == id)
            .ok_or("relation")?
            .properties
            .insert(key.into(), value);
    }
    Ok(())
}
fn reparent(project: &mut Project, child: &str, owner: &str) -> Result {
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == format!("owns-{child}"))
        .ok_or("ownership")?
        .from = owner.into();
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
            issue["subject_id"] == "property"
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
fn given_deployed_composite_owned_attribute_then_owning_node_is_a_part_witness() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        // This OCL does not impose Node typing; Node-internal_structure is a
        // separate obligation and is deliberately not implemented by this rule.
        author(&mut project, "property", "type", json!({"$ref":"class"}))?;
        author(&mut project, "property", "isComposite", json!(false))?;
        author(&mut project, "node", "part", json!([]))?;
        assert!(issues(&project)?.is_empty());
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(
                issues(&decode(&encode(&project, format)?, format)?)?,
                issues(&project)?
            );
        }
    }
    Ok(())
}

#[test]
fn given_deployed_property_then_wrong_owner_or_noncomposite_aggregation_violates_rule() -> Result {
    for aggregation in ["none", "shared"] {
        let mut project = document(true)?;
        author(&mut project, "property", "aggregation", json!(aggregation))?;
        author(&mut project, "property", "isComposite", json!(true))?;
        author(&mut project, "node", "part", json!([{"$ref":"property"}]))?;
        expect(&project, ERROR)?;
    }
    let mut project = document(true)?;
    reparent(&mut project, "property", "class")?;
    author(&mut project, "node", "part", json!([{"$ref":"property"}]))?;
    expect(&project, ERROR)?;
    let mut project = document(true)?;
    project
        .relations
        .retain(|relation| relation.id != "owns-property");
    expect(&project, ERROR)?;
    Ok(())
}

#[test]
fn given_node_deployment_only_then_property_antecedent_is_empty_and_vacuous() -> Result {
    let mut project = document(true)?;
    author(&mut project, "property", "aggregation", json!("none"))?;
    reparent(&mut project, "property", "class")?;
    reparent(&mut project, "deployment", "node")?;
    // The notation's from endpoint still names the Property; only canonical
    // ownership determines DeploymentTarget::deployment, not that endpoint.
    assert!(issues(&project)?.is_empty());
    author(&mut project, "property", "deployment", json!([]))?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}

#[test]
fn given_external_deployment_then_implication_uses_known_consequence_or_stays_unknown() -> Result {
    let mut project = document(true)?;
    project
        .relations
        .retain(|relation| relation.id != "owns-deployment" && relation.id != "deployment");
    author(
        &mut project,
        "property",
        "deployment",
        json!([{"href":"external.uml#deployment"}]),
    )?;
    assert!(
        issues(&project)?.is_empty(),
        "a true consequent proves the implication"
    );
    author(&mut project, "property", "aggregation", json!("none"))?;
    expect(&project, UNKNOWN)?;
    for reverse in [true, false] {
        let mut project = document(true)?;
        let mut entries = vec![
            json!({"$ref":"deployment"}),
            json!({"href":"external.uml#deployment"}),
        ];
        if reverse {
            entries.reverse();
        }
        author(&mut project, "property", "deployment", json!(entries))?;
        author(&mut project, "property", "aggregation", json!("none"))?;
        expect(&project, ERROR)?;
    }
    Ok(())
}

#[test]
fn given_unknown_owner_then_known_noncomposite_part_can_still_disprove_consequence() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "node")
        .ok_or("node")?
        .kind = "uml.unknown_node".into();
    expect(&project, UNKNOWN)?;
    author(&mut project, "property", "aggregation", json!("none"))?;
    expect(&project, ERROR)?;
    author(&mut project, "property", "aggregation", json!(42))?;
    expect(&project, INVALID)?;
    Ok(())
}

#[test]
fn given_malformed_deployment_or_conflicting_inverse_then_invalid_dominates() -> Result {
    for value in [
        Value::Null,
        json!(42),
        json!([]),
        json!([{"$ref":"deployment"},{"$ref":"deployment"}]),
        json!([{"href":"external.uml#d"},{"$ref":"missing"}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "property", "deployment", value)?;
        expect(&project, INVALID)?;
    }
    for value in [
        Value::Null,
        json!(42),
        json!({"$ref":"node"}),
        json!({"href":""}),
    ] {
        let mut project = document(true)?;
        author(&mut project, "deployment", "location", value)?;
        expect(&project, INVALID)?;
    }
    let mut project = document(true)?;
    author(
        &mut project,
        "deployment",
        "location",
        json!({"href":"external.uml#location"}),
    )?;
    author(&mut project, "property", "aggregation", json!("none"))?;
    expect(&project, ERROR)?;
    Ok(())
}

#[test]
fn given_multiple_owners_then_canonical_ambiguity_is_invalid() -> Result {
    for child in ["property", "deployment"] {
        let mut project = document(true)?;
        project.relations.push(serde_json::from_value(
            json!({"id":"second-owner","kind":"owns","from":"root","to":child}),
        )?);
        let kernel = bellatrix_mde::uml::root::Model::from_project(&project).validate();
        assert!(
            kernel
                .iter()
                .any(|issue| issue.subject_id == "property" && issue.code == INVALID),
            "{kernel:#?}"
        );
        let response: Value = serde_json::from_str(&validate_uml_model(
            &json!({"api_version":1,"model":project}).to_string(),
        )?)?;
        assert_eq!(response["has_errors"], true);
        assert!(
            response["diagnostics"]
                .as_array()
                .ok_or("diagnostics")?
                .iter()
                .any(|issue| issue["code"] == "GRAPH_INTEGRITY" && issue["severity"] == "error"),
            "{response:#?}"
        );
    }
    Ok(())
}

#[test]
fn when_deployed_part_moves_out_of_node_then_command_rejects_atomically_and_repair_succeeds()
-> Result {
    let project = document(true)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetOwner {
            element_id: "property".into(),
            owner_id: Some("class".into()),
        },
        1,
    )
    .expect_err("a deployed Property must be a Node part");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&project, Format::Json)?, before);
    let mut invalid = project;
    reparent(&mut invalid, "property", "class")?;
    expect(&invalid, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetOwner {
                element_id: "property".into(),
                owner_id: Some("node".into()),
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}

#[test]
fn given_only_non_deployment_owned_elements_then_the_antecedent_stays_empty() -> Result {
    let mut project = document(true)?;
    project
        .relations
        .retain(|relation| relation.id != "owns-deployment" && relation.id != "deployment");
    project.elements.push(serde_json::from_value(json!({"id":"literal","kind":"uml.literal_integer","name":"Value","properties":{"value":1}}))?);
    project.relations.push(serde_json::from_value(
        json!({"id":"owns-literal","kind":"owns","from":"property","to":"literal"}),
    )?);
    author(&mut project, "property", "defaultValue", json!(42))?;
    author(&mut project, "property", "aggregation", json!("none"))?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}

#[test]
fn given_device_or_execution_environment_owner_then_node_subtype_can_witness_part() -> Result {
    for kind in ["uml.device", "uml.execution_environment"] {
        let mut project = document(true)?;
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "node")
            .ok_or("node")?
            .kind = kind.into();
        assert!(
            issues(&project)?.is_empty(),
            "{kind}: {:?}",
            issues(&project)?
        );
    }
    Ok(())
}

#[test]
fn given_property_inherited_by_node_then_inheritance_does_not_make_it_a_node_part() -> Result {
    let mut project = document(true)?;
    reparent(&mut project, "property", "class")?;
    project.relations.push(serde_json::from_value(
        json!({"id":"inherits","kind":"uml.generalization","from":"node","to":"class"}),
    )?);
    expect(&project, ERROR)?;
    Ok(())
}

#[test]
fn given_invalid_explicit_owned_attribute_then_canonical_node_part_cannot_be_verified() -> Result {
    for value in [
        Value::Null,
        json!([]),
        json!(42),
        json!([{"$ref":"property"},{"$ref":"property"}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "node", "ownedAttribute", value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
