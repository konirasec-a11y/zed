use bellatrix_mde::{
    Format, Project, StructuralSeverity,
    commands::{Command, apply},
    decode, encode,
    uml::root::Model,
};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn fixture(owner_kind: &str, kind: Option<Value>, role: Option<Value>) -> Result<Project> {
    let mut project: Project = serde_json::from_value(json!({
        "id":"model", "name":"Connection points", "elements":[
            {"id":"owner", "kind":owner_kind, "name":"Owner"},
            {"id":"point", "kind":"uml.pseudostate", "name":"Point"}
        ], "relations":[{"id":"owns-point", "kind":"owns", "from":"owner", "to":"point"}]
    }))?;
    if let Some(kind) = kind {
        edit(&mut project, "point", "kind", kind)?;
    }
    if let Some(role) = role {
        edit(&mut project, "owner", "connectionPoint", role)?;
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
fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project)
        .validate()
        .into_iter()
        .filter(|issue| {
            issue.subject_id == "owner"
                && matches!(
                    issue.code,
                    "UML_STATE_MACHINE_CONNECTION_POINTS"
                        | "MDE_STATE_MACHINE_CONNECTION_POINTS_UNVERIFIED"
                        | "UML_STATE_ENTRY_OR_EXIT"
                        | "MDE_STATE_ENTRY_OR_EXIT_UNVERIFIED"
                )
        })
        .map(|issue| (issue.severity, issue.code))
        .collect()
}
fn error(owner_kind: &str) -> (StructuralSeverity, &'static str) {
    (
        StructuralSeverity::Error,
        if owner_kind == "uml.state" {
            "UML_STATE_ENTRY_OR_EXIT"
        } else {
            "UML_STATE_MACHINE_CONNECTION_POINTS"
        },
    )
}

#[test]
fn imported_connection_points_enforce_normative_kinds_and_default_after_roundtrip() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        for kind in [
            None,
            Some("initial"),
            Some("entryPoint"),
            Some("exitPoint"),
            Some("choice"),
            Some("junction"),
            Some("fork"),
            Some("join"),
            Some("shallowHistory"),
            Some("deepHistory"),
            Some("terminate"),
        ] {
            for role in [None, Some(json!([{"$ref":"point"}]))] {
                let project = fixture(owner, kind.map(|kind| json!(kind)), role)?;
                let expected = if matches!(kind, Some("entryPoint" | "exitPoint")) {
                    vec![]
                } else {
                    vec![error(owner)]
                };
                for format in [Format::Json, Format::Protobuf] {
                    let reopened = decode(&encode(&project, format)?, format)?;
                    assert_eq!(outcome(&reopened), expected, "{owner} / {kind:?}");
                }
            }
        }
    }
    Ok(())
}

#[test]
fn kind_edits_reject_new_violation_atomically_and_repair_imported_invalid_snapshot() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        let project = fixture(owner, Some(json!("entryPoint")), None)?;
        let before = project.clone();
        let change = |kind| Command::SetProperty {
            element_id: "point".into(),
            path: "properties.kind".into(),
            value: json!(kind),
        };
        let failure = apply(&project, &change("initial"), 0)
            .expect_err("kind must be checked after ownership was established");
        assert!(failure.starts_with(error(owner).1), "{failure}");
        assert_eq!(project, before);
        let valid = apply(&project, &change("exitPoint"), 0)?;
        assert!(outcome(&valid).is_empty());
        let invalid = fixture(owner, Some(json!("initial")), None)?;
        let repaired = apply(&invalid, &change("entryPoint"), 0)?;
        assert!(outcome(&repaired).is_empty());
        for format in [Format::Json, Format::Protobuf] {
            assert!(outcome(&decode(&encode(&repaired, format)?, format)?).is_empty());
        }
    }
    Ok(())
}
