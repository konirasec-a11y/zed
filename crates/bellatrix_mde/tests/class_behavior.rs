use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "BehavioredClassifier-class_behavior:";

fn document(rooted: bool, selected: bool, specification: Option<Value>) -> Result<Project> {
    let mut value = json!({"id":"project", "name":"Classifier behavior", "root_model":"root",
    "elements":[
        {"id":"root", "kind":"uml.model", "name":"Root"},
        {"id":"class", "kind":"uml.class", "name":"Active", "properties":{"isActive":true, "ownedBehavior":[{"$ref":"behavior"}]}},
        {"id":"behavior", "kind":"uml.opaque_behavior", "name":"Implementation"},
        {"id":"operation", "kind":"uml.operation", "name":"Execute"}
    ],
    "relations":[
        {"id":"root-class", "kind":"owns", "from":"root", "to":"class"},
        {"id":"class-behavior", "kind":"owns", "from":"class", "to":"behavior"},
        {"id":"class-operation", "kind":"owns", "from":"class", "to":"operation"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    let mut project: Project = serde_json::from_value(value)?;
    if selected {
        author(
            &mut project,
            "class",
            "classifierBehavior",
            json!({"$ref":"behavior"}),
        )?;
    }
    if let Some(specification) = specification {
        author(&mut project, "behavior", "specification", specification)?;
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
        .is_some_and(|message| message.starts_with(RULE))
}

#[test]
fn given_classifier_behavior_then_only_nonempty_specification_is_forbidden_in_rooted_and_rootless_models()
-> Result {
    for rooted in [true, false] {
        for specification in [None, Some(Value::Null), Some(json!({"$ref":"operation"}))] {
            let project = document(rooted, true, specification.clone())?;
            let issues = diagnostics(&project)?;
            let invalid = specification == Some(json!({"$ref":"operation"}));
            assert_eq!(
                issues
                    .iter()
                    .filter(|issue| relevant(issue) && issue["severity"] == "error")
                    .count(),
                usize::from(invalid),
                "{issues:#?}"
            );
            if !invalid {
                assert!(
                    !issues.iter().any(|issue| issue["severity"] == "error"),
                    "{issues:#?}"
                );
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
fn when_generic_editor_cannot_write_classifier_behavior_then_rejection_preserves_the_model()
-> Result {
    let mut valid = document(true, false, Some(json!({"$ref":"operation"})))?;
    author(&mut valid, "class", "classifierBehavior", Value::Null)?;
    let before = encode(&valid, Format::Json)?;
    let error = apply(
        &valid,
        &Command::SetProperty {
            element_id: "class".into(),
            path: "properties.classifierBehavior".into(),
            value: json!({"$ref":"behavior"}),
        },
        1,
    )
    .expect_err("this reference is not writable through the generic editor");
    assert_eq!(error, "property is read-only");
    assert_eq!(encode(&valid, Format::Json)?, before);
    Ok(())
}
