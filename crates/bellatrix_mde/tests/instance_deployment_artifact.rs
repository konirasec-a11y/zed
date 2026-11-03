use bellatrix_mde::{Format, Project, decode, encode, uml::root::Model, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "InstanceSpecification-deployment_artifact:";
const ERROR: &str = "UML_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT";
const INVALID: &str = "MDE_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT_INVALID";
const UNKNOWN: &str = "MDE_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Instance deployment","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"node","kind":"uml.node","name":"Node"},
        {"id":"artifact","kind":"uml.artifact","name":"Artifact"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"instance","kind":"uml.instance_specification","name":"Instance","properties":{"classifier":[{"$ref":"artifact"}]}}
    ],"relations":[
        {"id":"deployment","kind":"uml.deployment","from":"node","to":"instance","properties":{"visibility":null,"deployedArtifact":[{"$ref":"instance"}]}}
    ]});
    for (owner, child) in [
        ("root", "node"),
        ("root", "artifact"),
        ("root", "class"),
        ("root", "instance"),
        ("node", "deployment"),
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
fn given_deployed_instance_then_an_artifact_classifier_is_required() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        for value in [json!([]), json!([{"$ref":"class"}])] {
            author(&mut project, "instance", "classifier", value)?;
            expect(&project, ERROR)?;
            for format in [Format::Json, Format::Protobuf] {
                expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
            }
        }
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "instance")
            .ok_or("instance")?
            .properties
            .remove("classifier");
        expect(&project, ERROR)?;
    }
    Ok(())
}

#[test]
fn given_empty_canonical_membership_then_endpoints_do_not_invent_a_deployment() -> Result {
    let mut project = document(true)?;
    author(&mut project, "instance", "classifier", json!([]))?;
    author(&mut project, "deployment", "deployedArtifact", json!([]))?;
    assert!(issues(&project)?.is_empty());
    author(&mut project, "instance", "deploymentForArtifact", json!([]))?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}

#[test]
fn given_inconclusive_antecedent_then_known_artifact_proves_the_implication() -> Result {
    for missing in [true, false] {
        let mut project = document(true)?;
        if missing {
            project
                .relations
                .iter_mut()
                .find(|relation| relation.id == "deployment")
                .ok_or("deployment")?
                .properties
                .remove("deployedArtifact");
        } else {
            author(
                &mut project,
                "deployment",
                "deployedArtifact",
                json!([{"href":"external.uml#instance"}]),
            )?;
        }
        assert!(issues(&project)?.is_empty());
        author(
            &mut project,
            "instance",
            "classifier",
            json!([{"$ref":"class"}]),
        )?;
        expect(&project, UNKNOWN)?;
        author(
            &mut project,
            "instance",
            "classifier",
            json!([{"$ref":"missing"}]),
        )?;
        expect(&project, INVALID)?;
    }
    Ok(())
}

#[test]
fn given_external_neighbors_then_known_membership_and_artifact_evidence_are_retained() -> Result {
    for reverse in [true, false] {
        let mut project = document(true)?;
        let mut deployed = vec![
            json!({"href":"external.uml#instance"}),
            json!({"$ref":"instance"}),
        ];
        let mut classifiers = vec![
            json!({"href":"external.uml#classifier"}),
            json!({"$ref":"artifact"}),
        ];
        if reverse {
            deployed.reverse();
            classifiers.reverse();
        }
        author(
            &mut project,
            "deployment",
            "deployedArtifact",
            json!(deployed),
        )?;
        author(&mut project, "instance", "classifier", json!(classifiers))?;
        assert!(issues(&project)?.is_empty());
        let model = Model::from_project(&project);
        assert!(
            model
                .element("instance")
                .ok_or("instance")?
                .instance_specification()
                .ok_or("instance view")?
                .classifiers(model)
                .is_err(),
            "the complete classifier query must not expose a partial collection"
        );
        author(
            &mut project,
            "instance",
            "classifier",
            json!([{"$ref":"class"}]),
        )?;
        expect(&project, ERROR)?;
        author(
            &mut project,
            "instance",
            "classifier",
            json!([{"href":"external.uml#classifier"}]),
        )?;
        expect(&project, UNKNOWN)?;
    }
    Ok(())
}

#[test]
fn given_independent_invalid_data_then_true_consequence_does_not_hide_it() -> Result {
    for value in [
        Value::Null,
        json!(42),
        json!([{"$ref":"instance"},{"$ref":"instance"}]),
        json!([{"href":"external.uml#i"},{"$ref":"missing"}]),
        json!([{"href":""}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "deployment", "deployedArtifact", value)?;
        expect(&project, INVALID)?;
    }
    for value in [
        Value::Null,
        json!(42),
        json!([{"$ref":"artifact"},{"$ref":"artifact"}]),
        json!([{"$ref":"artifact"},{"$ref":"missing"}]),
        json!([{"$ref":"artifact"},{"$ref":"instance"}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "instance", "classifier", value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}

#[test]
fn given_authored_inverse_then_it_must_agree_with_forward_memberships() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "instance",
        "deploymentForArtifact",
        json!([{"$ref":"deployment"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    author(&mut project, "instance", "deploymentForArtifact", json!([]))?;
    expect(&project, INVALID)?;
    author(
        &mut project,
        "instance",
        "deploymentForArtifact",
        json!([{"$ref":"deployment"}]),
    )?;
    author(&mut project, "deployment", "deployedArtifact", json!([]))?;
    expect(&project, INVALID)?;
    author(
        &mut project,
        "instance",
        "deploymentForArtifact",
        json!([{"href":"external.uml#deployment"}]),
    )?;
    author(
        &mut project,
        "instance",
        "classifier",
        json!([{"$ref":"class"}]),
    )?;
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "instance",
        "deploymentForArtifact",
        json!([{"href":""}]),
    )?;
    expect(&project, INVALID)?;
    Ok(())
}

#[test]
fn given_unknown_classifier_metaclass_then_the_rule_remains_unverified() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "artifact")
        .ok_or("artifact")?
        .kind = "uml.unknown_artifact".into();
    expect(&project, UNKNOWN)?;
    Ok(())
}

#[test]
fn given_artifact_subclass_then_metaclass_conformance_not_user_inheritance_decides() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "artifact")
        .ok_or("artifact")?
        .kind = "uml.deployment_specification".into();
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "instance",
        "classifier",
        json!([{"$ref":"class"}]),
    )?;
    project.relations.push(serde_json::from_value(
        json!({"id":"general","kind":"uml.generalization","from":"class","to":"artifact"}),
    )?);
    expect(&project, ERROR)?;
    Ok(())
}
