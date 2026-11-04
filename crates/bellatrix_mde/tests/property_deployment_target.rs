use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "Property-deployment_target:";
const ERROR: &str = "UML_PROPERTY_DEPLOYMENT_TARGET";
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
