use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "DeploymentSpecification-deployed_elements:";
const ERROR: &str = "UML_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS";
const UNKNOWN: &str = "MDE_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Deployed elements","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"location","kind":"uml.execution_environment","name":"Environment"},
        {"id":"artifact","kind":"uml.artifact","name":"Artifact"},
        {"id":"specification","kind":"uml.deployment_specification","name":"Configuration","properties":{"visibility":null}},
        {"id":"component","kind":"uml.component","name":"Component"},
        {"id":"class","kind":"uml.class","name":"Class"}
    ],
    "relations":[
        {"id":"deployment","kind":"uml.deployment","from":"location","to":"artifact","properties":{"visibility":null,"deployedArtifact":[{"$ref":"artifact"}]}},
        {"id":"manifestation","kind":"uml.manifestation","from":"artifact","to":"component","properties":{"utilizedElement":{"$ref":"component"},"supplier":[{"$ref":"component"},{"$ref":"class"}]}}
    ]});
    for (owner, child) in [
        ("root", "location"),
        ("root", "artifact"),
        ("root", "component"),
        ("root", "class"),
        ("location", "deployment"),
        ("deployment", "specification"),
        ("artifact", "manifestation"),
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
fn author(project: &mut Project, id: &str, property: &str, value: Value) -> Result {
    if let Some(element) = project.elements.iter_mut().find(|element| element.id == id) {
        element.properties.insert(property.into(), value);
    } else {
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == id)
            .ok_or("relation")?
            .properties
            .insert(property.into(), value);
    }
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
            issue["message"]
                .as_str()
                .is_some_and(|message| message.starts_with(RULE))
        })
        .cloned()
        .collect())
}
fn expect(project: &Project, code: &str) -> Result {
    let actual = issues(project)?;
    assert_eq!(actual.len(), 1, "{actual:#?}");
    assert_eq!(actual[0]["code"], code, "{actual:#?}");
    assert_eq!(
        actual[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    assert_eq!(actual[0]["subject_id"], "specification");
    Ok(())
}

#[test]
fn given_artifact_manifestations_then_only_components_are_deployed_elements() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        author(
            &mut project,
            "manifestation",
            "utilizedElement",
            json!({"$ref":"class"}),
        )?;
        expect(&project, ERROR)?;
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
fn when_manifestation_moves_to_deployed_artifact_then_reparent_rejects_atomically_and_repair_succeeds()
-> Result {
    let mut project = document(true)?;
    project.elements.push(serde_json::from_value(
        json!({"id":"unused", "kind":"uml.artifact", "name":"Unused"}),
    )?);
    project.relations.push(serde_json::from_value(
        json!({"id":"owns-unused", "kind":"owns", "from":"root", "to":"unused"}),
    )?);
    project.relations.push(serde_json::from_value(
        json!({"id":"bad", "kind":"uml.manifestation", "from":"unused", "to":"class",
        "properties":{"utilizedElement":{"$ref":"class"}}}),
    )?);
    project.relations.push(serde_json::from_value(
        json!({"id":"owns-bad", "kind":"owns", "from":"unused", "to":"bad"}),
    )?);
    assert!(issues(&project)?.is_empty());
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetOwner {
            element_id: "bad".into(),
            owner_id: Some("artifact".into()),
        },
        1,
    )
    .expect_err("non-Component must be rejected");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(encode(&project, Format::Json)?, before);
    let mut invalid = project;
    invalid
        .relations
        .iter_mut()
        .find(|relation| relation.id == "owns-bad")
        .ok_or("owner")?
        .from = "artifact".into();
    expect(&invalid, ERROR)?;
    let repaired = apply(
        &invalid,
        &Command::SetOwner {
            element_id: "bad".into(),
            owner_id: Some("unused".into()),
        },
        2,
    )?;
    assert!(issues(&repaired)?.is_empty());
    Ok(())
}
