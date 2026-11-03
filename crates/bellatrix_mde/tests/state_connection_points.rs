use bellatrix_mde::{Format, Project, StructuralSeverity, commands::{Command, apply}, decode, encode, uml::root::Model};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn fixture(owner_kind: &str, kind: Option<Value>, role: Option<Value>) -> Result<Project> {
    let mut project: Project = serde_json::from_value(json!({
        "id":"model", "name":"Connection points", "elements":[
            {"id":"owner", "kind":owner_kind, "name":"Owner"},
            {"id":"point", "kind":"uml.pseudostate", "name":"Point"}
        ], "relations":[{"id":"owns-point", "kind":"owns", "from":"owner", "to":"point"}]
    }))?;
    if let Some(kind) = kind { edit(&mut project, "point", "kind", kind)?; }
    if let Some(role) = role { edit(&mut project, "owner", "connectionPoint", role)?; }
    Ok(project)
}
fn edit(project: &mut Project, id: &str, key: &str, value: Value) -> Result {
    project.elements.iter_mut().find(|element| element.id == id).ok_or("element")?
        .properties.insert(key.into(), value);
    Ok(())
}
fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project).validate().into_iter()
        .filter(|issue| issue.subject_id == "owner" && matches!(issue.code,
            "UML_STATE_MACHINE_CONNECTION_POINTS" | "MDE_STATE_MACHINE_CONNECTION_POINTS_UNVERIFIED"
            | "UML_STATE_ENTRY_OR_EXIT" | "MDE_STATE_ENTRY_OR_EXIT_UNVERIFIED"))
        .map(|issue| (issue.severity, issue.code)).collect()
}
fn error(owner_kind: &str) -> (StructuralSeverity, &'static str) {
    (StructuralSeverity::Error, if owner_kind == "uml.state" { "UML_STATE_ENTRY_OR_EXIT" } else { "UML_STATE_MACHINE_CONNECTION_POINTS" })
}
fn unknown(owner_kind: &str) -> (StructuralSeverity, &'static str) {
    (StructuralSeverity::Warning, if owner_kind == "uml.state" { "MDE_STATE_ENTRY_OR_EXIT_UNVERIFIED" } else { "MDE_STATE_MACHINE_CONNECTION_POINTS_UNVERIFIED" })
}

#[test]
fn imported_connection_points_enforce_normative_kinds_and_default_after_roundtrip() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        for kind in [None, Some("initial"), Some("entryPoint"), Some("exitPoint"), Some("choice"), Some("junction"), Some("fork"), Some("join"), Some("shallowHistory"), Some("deepHistory"), Some("terminate")] {
            for role in [None, Some(json!([{"$ref":"point"}]))] {
                let project = fixture(owner, kind.map(|kind| json!(kind)), role)?;
                let expected = if matches!(kind, Some("entryPoint" | "exitPoint")) { vec![] } else { vec![error(owner)] };
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
fn known_bad_kind_dominates_external_members_in_either_order() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        for reverse in [false, true] {
            let mut role = vec![json!({"href":"other.uml#point"}), json!({"$ref":"point"})];
            if reverse { role.reverse(); }
            for (kind, expected) in [("initial", error(owner)), ("entryPoint", unknown(owner))] {
                let project = fixture(owner, Some(json!(kind)), Some(json!(role)))?;
                assert_eq!(outcome(&project), vec![expected]);
            }
        }
    }
    Ok(())
}

#[test]
fn malformed_roles_and_kinds_remain_invalid_beside_unknown_members() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        for role in [json!(null), json!(42), json!([null]), json!([{"$ref":"missing"}]),
            json!([{"$ref":"point"},{"$ref":"point"}]), json!([])] {
            let project = fixture(owner, Some(json!("entryPoint")), Some(role))?;
            assert_eq!(outcome(&project), vec![error(owner)]);
        }
        for kind in [json!(null), json!(42), json!("unknown"), json!([])] {
            let project = fixture(owner, Some(kind), Some(json!([{"href":"other.uml#point"},{"$ref":"point"}])))?;
            assert_eq!(outcome(&project), vec![error(owner)]);
        }
        let mut wrong_type = fixture(owner, Some(json!("entryPoint")), Some(json!([{"href":"other.uml#point"},{"$ref":"point"}])))?;
        wrong_type.elements.iter_mut().find(|element| element.id == "point").ok_or("point")?.kind = "uml.class".into();
        assert_eq!(outcome(&wrong_type), vec![error(owner)]);
        wrong_type.elements.iter_mut().find(|element| element.id == "point").ok_or("point")?.kind = "extension.unknown".into();
        assert_eq!(outcome(&wrong_type), vec![unknown(owner)]);
    }
    Ok(())
}

#[test]
fn explicit_composite_references_cannot_override_canonical_ownership() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        let mut project = fixture(owner, Some(json!("entryPoint")), Some(json!([{"$ref":"point"}])))?;
        project.relations.clear();
        assert_eq!(outcome(&project), vec![error(owner)]);
        project.elements.retain(|element| element.id != "point");
        edit(&mut project, "owner", "connectionPoint", json!([]))?;
        assert!(outcome(&project).is_empty());
        edit(&mut project, "owner", "connectionPoint", json!([{"href":"other.uml#point"}]))?;
        assert_eq!(outcome(&project), vec![unknown(owner)]);
        project.elements.iter_mut().find(|element| element.id == "owner").ok_or("owner")?.properties.clear();
        assert!(outcome(&project).is_empty());
    }
    Ok(())
}

#[test]
fn kind_edits_reject_new_violation_atomically_and_repair_imported_invalid_snapshot() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        let project = fixture(owner, Some(json!("entryPoint")), None)?;
        let before = project.clone();
        let change = |kind| Command::SetProperty { element_id: "point".into(), path: "properties.kind".into(), value: json!(kind) };
        let failure = apply(&project, &change("initial"), 0).expect_err("kind must be checked after ownership was established");
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

#[test]
fn protocol_state_machine_inherits_constraint_but_region_vertices_are_not_connection_points() -> Result {
    for kind in ["entryPoint", "initial"] {
        let project = fixture("uml.protocol_state_machine", Some(json!(kind)), None)?;
        assert_eq!(outcome(&project), if kind == "initial" { vec![error("uml.state_machine")] } else { vec![] });
    }
    for owner in ["uml.state_machine", "uml.state", "uml.protocol_state_machine"] {
        let project: Project = serde_json::from_value(json!({
            "id":"model", "name":"Region vertices", "elements":[
                {"id":"owner", "kind":owner, "name":"Owner"},
                {"id":"region", "kind":"uml.region", "name":"Region"},
                {"id":"point", "kind":"uml.pseudostate", "name":"Initial"}
            ], "relations":[
                {"id":"owns-region", "kind":"owns", "from":"owner", "to":"region"},
                {"id":"owns-point", "kind":"owns", "from":"region", "to":"point"}
            ]
        }))?;
        assert!(outcome(&project).is_empty(), "{owner}");
    }
    Ok(())
}

#[test]
fn duplicate_and_ambiguous_ownership_are_invalid_not_unverified() -> Result {
    for owner in ["uml.state_machine", "uml.state"] {
        let project = fixture(owner, Some(json!("entryPoint")), Some(json!([{"href":"external.uml#point"},{"$ref":"point"}])))?;
        let mut duplicate = project.clone();
        let mut edge = duplicate.relations.first().ok_or("ownership")?.clone();
        edge.id = "duplicate-ownership".into();
        duplicate.relations.push(edge.clone());
        assert_eq!(outcome(&duplicate), vec![error(owner)]);
        let mut ambiguous = project;
        let mut second = ambiguous.elements.iter().find(|element| element.id == "owner").ok_or("owner")?.clone();
        second.id = "other-owner".into();
        second.properties.clear();
        ambiguous.elements.push(second);
        edge.from = "other-owner".into();
        ambiguous.relations.push(edge);
        assert_eq!(outcome(&ambiguous), vec![error(owner)]);
    }
    Ok(())
}
