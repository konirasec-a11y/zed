use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "InstanceSpecification-deployment_target:";
const ERROR: &str = "UML_INSTANCE_SPECIFICATION_DEPLOYMENT_TARGET";
const INVALID: &str = "MDE_INSTANCE_SPECIFICATION_DEPLOYMENT_TARGET_INVALID";
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
fn empty_deployment_is_vacuous_but_nonempty_needs_node_classifier() -> Result {
    let mut project = document(true)?;
    author(&mut project, "instance", "classifier", json!([]))?;
    expect(&project, ERROR)?;
    author(
        &mut project,
        "instance",
        "classifier",
        json!([{"$ref":"class"}]),
    )?;
    expect(&project, ERROR)?;
    project
        .relations
        .retain(|r| r.id != "deployment" && r.id != "owns-deployment");
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn parts_are_composite_owned_attributes_and_type_uses_identity() -> Result {
    for aggregation in ["none", "shared"] {
        let mut project = document(true)?;
        author(&mut project, "property", "aggregation", json!(aggregation))?;
        expect(&project, ERROR)?;
    }
    let mut project = document(true)?;
    reparent(&mut project, "property", "class")?;
    project.relations.push(serde_json::from_value(
        json!({"id":"general","kind":"uml.generalization","from":"container","to":"class"}),
    )?);
    expect(&project, ERROR)?;
    let mut project = document(true)?;
    author(
        &mut project,
        "property",
        "type",
        json!({"$ref":"container"}),
    )?;
    project.relations.push(serde_json::from_value(
        json!({"id":"general","kind":"uml.generalization","from":"container","to":"node"}),
    )?);
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn external_evidence_stays_unknown_without_erasing_local_witnesses() -> Result {
    for (id, key, value) in [
        ("instance", "classifier", json!([{"href":"external#Node"}])),
        ("property", "type", json!({"href":"external#Node"})),
        (
            "instance",
            "deployment",
            json!([{"$ref":"deployment"},{"href":"external#Deployment"}]),
        ),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, key, value)?;
        if key == "deployment" {
            assert!(issues(&project)?.is_empty());
        } else {
            expect(&project, UNKNOWN)?;
        }
    }
    let mut project = document(true)?;
    author(
        &mut project,
        "instance",
        "classifier",
        json!([{"href":"external#Node"},{"$ref":"node"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn invalid_contracts_dominate_unknown_and_known_witnesses() -> Result {
    for (id, key, value) in [
        (
            "instance",
            "classifier",
            json!([{"$ref":"node"},{"$ref":"node"}]),
        ),
        (
            "instance",
            "classifier",
            json!([{"href":"external#Node"},42]),
        ),
        (
            "instance",
            "deployment",
            json!([{"$ref":"deployment"},{"$ref":"deployment"}]),
        ),
        ("container", "ownedAttribute", json!([])),
        ("property", "type", json!(42)),
        ("property", "aggregation", json!("broken")),
        ("deployment", "location", json!({"$ref":"node"})),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, key, value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn local_unknown_classifier_and_untyped_parts_are_distinguished() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "node")
        .ok_or("node")?
        .kind = "uml.future_node".into();
    expect(&project, UNKNOWN)?;
    for value in [Value::Null] {
        let mut project = document(true)?;
        author(&mut project, "property", "type", value)?;
        expect(&project, ERROR)?;
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

#[test]
fn witnesses_in_multiple_containers_preserve_unknown_and_invalid_precedence_in_both_orders()
-> Result {
    for reverse in [false, true] {
        for bad_type in [json!({"href":"external#Node"}), json!(42)] {
            let mut project = document(true)?;
            project.elements.push(serde_json::from_value(
                json!({"id":"container2","kind":"uml.node","name":"Other container"}),
            )?);
            project.elements.push(serde_json::from_value(json!({"id":"property2","kind":"uml.property","name":"Other part","properties":{"aggregation":"composite","type":bad_type}}))?);
            for (owner, child) in [("root", "container2"), ("container2", "property2")] {
                project.relations.push(serde_json::from_value(
                    json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}),
                )?);
            }
            let mut candidates = vec![
                json!({"$ref":"container2"}),
                json!({"$ref":"node"}),
                json!({"href":"external#Classifier"}),
            ];
            if reverse {
                project.elements.reverse();
                candidates.reverse();
            }
            author(&mut project, "instance", "classifier", json!(candidates))?;
            if bad_type.is_number() {
                expect(&project, INVALID)?;
            } else {
                assert!(issues(&project)?.is_empty());
            }
        }
    }
    Ok(())
}
