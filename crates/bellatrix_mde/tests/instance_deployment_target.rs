use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "InstanceSpecification-deployment_target:";
const ERROR: &str = "UML_INSTANCE_SPECIFICATION_DEPLOYMENT_TARGET";
const UNKNOWN: &str = "MDE_INSTANCE_SPECIFICATION_DEPLOYMENT_TARGET_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Property deployment","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"node","kind":"uml.node","name":"Node"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"artifact","kind":"uml.artifact","name":"Artifact"},
        {"id":"instance","kind":"uml.instance_specification","name":"Instance","properties":{"classifier":[{"$ref":"node"}]}},
        {"id":"container","kind":"uml.node","name":"Container"},
        {"id":"property","kind":"uml.attribute","name":"Part","properties":{"type":{"$ref":"node"},"aggregation":"composite"}}
    ],"relations":[
        {"id":"deployment","kind":"uml.deployment","from":"instance","to":"artifact","properties":{"visibility":null}}
    ]});
    for (owner, child) in [
        ("root", "node"),
        ("root", "class"),
        ("root", "artifact"),
        ("root", "instance"),
        ("root", "container"),
        ("container", "property"),
        ("instance", "deployment"),
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
            issue["subject_id"] == "instance"
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
fn node_part_type_witness_survives_roundtrip_and_draft() -> Result {
    for rooted in [true, false] {
        for kind in ["uml.node", "uml.device", "uml.execution_environment"] {
            let mut project = document(rooted)?;
            project
                .elements
                .iter_mut()
                .find(|e| e.id == "node")
                .ok_or("node")?
                .kind = kind.into();
            assert!(issues(&project)?.is_empty(), "{:?}", issues(&project)?);
            for format in [Format::Json, Format::Protobuf] {
                assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
            }
        }
    }
    Ok(())
}

#[test]
fn moving_only_part_out_of_node_rolls_back_and_invalid_snapshot_can_be_repaired() -> Result {
    let project = document(true)?;
    let snapshot = serde_json::to_value(&project)?;
    let error = apply(
        &project,
        &Command::SetOwner {
            element_id: "property".into(),
            owner_id: Some("class".into()),
        },
        1,
    )
    .expect_err("must reject");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(snapshot, serde_json::to_value(&project)?);
    let mut invalid = project;
    reparent(&mut invalid, "property", "class")?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetOwner {
                element_id: "property".into(),
                owner_id: Some("container".into()),
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}
