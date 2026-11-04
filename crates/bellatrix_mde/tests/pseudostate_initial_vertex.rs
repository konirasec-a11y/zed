use bellatrix_mde::{
    Format, Project, StructuralSeverity,
    commands::{Command, apply},
    decode, encode,
    uml::root::Model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PSEUDOSTATE_INITIAL_VERTEX";
const UNKNOWN: &str = "MDE_PSEUDOSTATE_INITIAL_VERTEX_UNVERIFIED";
fn fixture(kind: Option<Value>) -> Result<Project> {
    let mut project: Project =
        serde_json::from_value(json!({"id":"model","name":"Initial vertex","elements":[
            {"id":"point","kind":"uml.pseudostate","name":"Initial"},
            {"id":"other","kind":"uml.state","name":"Other"},
            {"id":"unknown","kind":"extension.unknown","name":"Unknown"},
            {"id":"wrong","kind":"uml.class","name":"Wrong"}
        ]}))?;
    if let Some(kind) = kind {
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "point")
            .ok_or("Pseudostate")?
            .properties
            .insert("kind".into(), kind);
    }
    Ok(project)
}
fn edge(project: &mut Project, id: &str, kind: &str, source: &str) -> Result {
    project.relations.push(serde_json::from_value(
        json!({"id":id,"kind":kind,"from":source,"to":"other"}),
    )?);
    Ok(())
}

fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project)
        .validate()
        .into_iter()
        .filter(|issue| issue.subject_id == "point" && matches!(issue.code, ERROR | UNKNOWN))
        .map(|issue| (issue.severity, issue.code))
        .collect()
}
fn invalid() -> Vec<(StructuralSeverity, &'static str)> {
    vec![(StructuralSeverity::Error, ERROR)]
}
fn unknown() -> Vec<(StructuralSeverity, &'static str)> {
    vec![(StructuralSeverity::Warning, UNKNOWN)]
}

#[test]
fn initial_default_and_explicit_kind_allow_zero_or_one_but_not_two_outgoing_transitions() -> Result
{
    for kind in [None, Some(json!("initial"))] {
        for count in 0..=3 {
            let mut project = fixture(kind.clone())?;
            for index in 0..count {
                edge(
                    &mut project,
                    &format!("edge-{index}"),
                    if index % 2 == 0 {
                        "uml.transition"
                    } else {
                        "uml.protocol_transition"
                    },
                    "point",
                )?;
            }
            for format in [Format::Json, Format::Protobuf] {
                assert_eq!(
                    outcome(&decode(&encode(&project, format)?, format)?),
                    if count > 1 { invalid() } else { vec![] }
                );
            }
        }
    }
    Ok(())
}

#[test]
fn changing_kind_rejects_new_violation_atomically_and_delete_repairs_import() -> Result {
    let mut project = fixture(Some(json!("choice")))?;
    edge(&mut project, "one", "uml.transition", "point")?;
    edge(&mut project, "two", "uml.transition", "point")?;
    let before = project.clone();
    let failure = apply(
        &project,
        &Command::SetProperty {
            element_id: "point".into(),
            path: "properties.kind".into(),
            value: json!("initial"),
        },
        0,
    )
    .expect_err("switching to initial must enforce the cardinality");
    assert!(failure.starts_with(ERROR), "{failure}");
    assert_eq!(project, before);
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "point")
        .ok_or("Pseudostate")?
        .properties
        .insert("kind".into(), json!("initial"));
    let repaired = apply(
        &project,
        &Command::DeleteElement {
            element_id: "two".into(),
        },
        0,
    )?;
    assert!(outcome(&repaired).is_empty());
    assert_eq!(outcome(&project), invalid());
    Ok(())
}
