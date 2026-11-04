use bellatrix_mde::{
    Format, Project, StructuralSeverity,
    commands::{Command, apply},
    decode, encode,
    uml::root::Model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PSEUDOSTATE_HISTORY_VERTICES";
const UNKNOWN: &str = "MDE_PSEUDOSTATE_HISTORY_VERTICES_UNVERIFIED";
fn fixture(kind: Option<Value>) -> Result<Project> {
    let mut project: Project =
        serde_json::from_value(json!({"id":"model","name":"History vertices","elements":[
            {"id":"point","kind":"uml.pseudostate","name":"History"},
            {"id":"other","kind":"uml.state","name":"Other"},
            {"id":"unknown","kind":"extension.unknown","name":"Unknown"}
        ]}))?;
    if let Some(kind) = kind {
        edit(&mut project, "point", "kind", kind)?;
    }
    Ok(project)
}
fn edit(project: &mut Project, id: &str, key: &str, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn transition(
    project: &mut Project,
    id: &str,
    kind: &str,
    source: &str,
    is_relation: bool,
) -> Result {
    if is_relation {
        project.relations.push(serde_json::from_value(
            json!({"id":id,"kind":kind,"from":source,"to":"other"}),
        )?);
    } else {
        project.elements.push(serde_json::from_value(
            json!({"id":id,"kind":kind,"name":id,"properties":{"source":{"$ref":source}}}),
        )?);
    }
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
fn both_history_kinds_enforce_zero_or_one_outgoing_in_both_representations() -> Result {
    for kind in ["deepHistory", "shallowHistory"] {
        for is_relation in [true, false] {
            for count in 0..=3 {
                let mut project = fixture(Some(json!(kind)))?;
                for index in 0..count {
                    transition(
                        &mut project,
                        &format!("edge-{index}"),
                        if index % 2 == 0 {
                            "uml.transition"
                        } else {
                            "uml.protocol_transition"
                        },
                        "point",
                        is_relation,
                    )?;
                }
                for format in [Format::Json, Format::Protobuf] {
                    assert_eq!(
                        outcome(&decode(&encode(&project, format)?, format)?),
                        if count > 1 { invalid() } else { vec![] },
                        "{kind} count={count}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn kind_change_rejects_new_violation_atomically_and_deletion_repairs_import() -> Result {
    for kind in ["deepHistory", "shallowHistory"] {
        let mut project = fixture(Some(json!("choice")))?;
        transition(&mut project, "one", "uml.transition", "point", true)?;
        transition(
            &mut project,
            "two",
            "uml.protocol_transition",
            "point",
            true,
        )?;
        let before = project.clone();
        let failure = apply(
            &project,
            &Command::SetProperty {
                element_id: "point".into(),
                path: "properties.kind".into(),
                value: json!(kind),
            },
            0,
        )
        .expect_err("history kind must enforce its outgoing limit");
        assert!(failure.starts_with(ERROR), "{failure}");
        assert_eq!(project, before);
        edit(&mut project, "point", "kind", json!(kind))?;
        let repaired = apply(
            &project,
            &Command::DeleteElement {
                element_id: "two".into(),
            },
            0,
        )?;
        assert!(outcome(&repaired).is_empty());
        assert_eq!(outcome(&project), invalid());
    }
    Ok(())
}
