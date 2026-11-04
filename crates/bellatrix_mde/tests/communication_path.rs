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
