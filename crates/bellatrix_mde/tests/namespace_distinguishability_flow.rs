//! Namespace overloads through canonical commands, validation and persistence.
//! UML 2.5.1 NamedElement/BehavioralFeature::isDistinguishableFrom and Namespace.

use bellatrix_mde::{
    Format, Project, StructuralSeverity,
    commands::{Command, apply},
    decode, encode,
    uml::{classifiers, namespace_imports, references::ReferenceIndex, root::Model},
};
use serde_json::{Value, json};

fn document() -> Value {
    json!({
        "id":"overloads", "name":"Namespace overloads", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"a", "kind":"uml.class", "name":"A"},
            {"id":"b", "kind":"uml.class", "name":"B"},
            {"id":"left", "kind":"uml.operation", "name":"process"},
            {"id":"right", "kind":"uml.operation", "name":"alternate"},
            {"id":"left-parameter", "kind":"uml.parameter", "name":"argument",
             "properties":{"type":{"$ref":"a"}}},
            {"id":"right-parameter", "kind":"uml.parameter", "name":"argument",
             "properties":{"type":{"$ref":"a"}}}
        ],
        "relations":[
            {"id":"owns-a", "kind":"owns", "from":"root", "to":"a"},
            {"id":"owns-b", "kind":"owns", "from":"root", "to":"b"},
            {"id":"owns-left", "kind":"owns", "from":"a", "to":"left"},
            {"id":"owns-right", "kind":"owns", "from":"a", "to":"right"},
            {"id":"owns-left-parameter", "kind":"owns", "from":"left", "to":"left-parameter"},
            {"id":"owns-right-parameter", "kind":"owns", "from":"right", "to":"right-parameter"}
        ]
    })
}

fn authored_element<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    document["elements"].as_array_mut().expect("elements")
        .iter_mut().find(|element| element["id"] == id).expect("authored identity")
}

fn reload(document: &Value) -> Project {
    decode(&serde_json::to_vec(document).expect("document bytes"), Format::Json)
        .expect("load authored document")
}

fn distinguishes(project: &Project, namespace: &str) -> bool {
    namespace_imports::members_are_distinguishable(
        &ReferenceIndex::new(Model::from_project(project)), namespace,
    ).expect("resolved namespace")
}

fn set_parameter(key: &str, value: Value) -> Command {
    if key == "name" {
        Command::Rename { element_id: "right-parameter".into(),
            name: value.as_str().expect("name").into() }
    } else {
        Command::SetProperty { element_id: "right-parameter".into(),
            path: format!("properties.{key}"), value }
    }
}

#[test]
fn overload_authoring_flow_checks_every_tuple_field_and_effective_defaults() {
    for (key, different, matching) in [
        ("name", json!("other"), json!("argument")),
        ("type", json!({"$ref":"b"}), json!({"$ref":"a"})),
        ("effect", json!("read"), Value::Null),
        ("direction", json!("inout"), json!("in")),
        ("isException", json!(true), json!(false)),
        ("isStream", json!(true), json!(false)),
        ("isOrdered", json!(true), json!(false)),
        ("isUnique", json!(false), json!(true)),
        ("multiplicity", json!("0..1"), json!("1")),
        ("multiplicity", json!("1..*"), json!("1")),
    ] {
        let mut content = document();
        if matches!(key, "effect" | "isException" | "isStream") {
            authored_element(&mut content, "right-parameter")["properties"][key] = matching.clone();
        }
        if key == "isException" {
            for id in ["left-parameter", "right-parameter"] {
                authored_element(&mut content, id)["properties"]["direction"] = json!("out");
            }
        }
        if key == "type" {
            content["relations"].as_array_mut().expect("relations").push(json!({
                "id":"type-specialization", "kind":"uml.generalization", "from":"b", "to":"a",
            }));
        }
        let initial = reload(&content);
        let distinct = apply(&initial, &set_parameter(key, different), 1)
            .expect("author a distinct signature");
        let overloaded = apply(&distinct, &Command::Rename {
            element_id: "right".into(), name: "process".into(),
        }, 2).expect("different parameter tuples permit the same operation name");
        assert!(distinguishes(&overloaded, "a"), "{key}");
        assert!(!Model::from_project(&overloaded).validate().iter().any(|issue|
            issue.severity == StructuralSeverity::Error), "{key}: {:?}",
            Model::from_project(&overloaded).validate());

        let before = encode(&overloaded, Format::Json).expect("snapshot before rejected edit");
        let collision = apply(&overloaded, &set_parameter(key, matching.clone()), 3);
        assert!(collision.is_err(), "matching {key} must reject the overload");
        assert_eq!(before, encode(&overloaded, Format::Json).expect("unchanged snapshot"));

        let mut invalid_content: Value = serde_json::from_slice(&before).expect("external draft");
        if key == "name" {
            authored_element(&mut invalid_content, "right-parameter")["name"] = matching;
        } else {
            authored_element(&mut invalid_content, "right-parameter")["properties"][key] = matching;
        }
        let invalid = reload(&invalid_content);
        assert!(!distinguishes(&invalid, "a"));
        assert!(Model::from_project(&invalid).validate().iter().any(|issue|
            issue.code == "UML_NAMESPACE_MEMBERS_DISTINGUISHABLE"));
        let repaired = apply(&invalid, &Command::Rename {
            element_id: "right".into(), name: "alternate".into(),
        }, 4).expect("repair an imported invalid overload");
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&overloaded, format).expect("save valid overload"), format)
                .expect("reopen valid overload");
            assert_eq!(overloaded, reopened);
            assert!(distinguishes(&reopened, "a"));
            let reopened = decode(&encode(&invalid, format).expect("save invalid draft"), format)
                .expect("reopen invalid draft");
            assert!(!distinguishes(&reopened, "a"));
            assert!(distinguishes(&repaired, "a"));
        }
    }

    let mut unresolved = document();
    authored_element(&mut unresolved, "right")["name"] = json!("process");
    for id in ["left-parameter", "right-parameter"] {
        authored_element(&mut unresolved, id)["properties"]["type"] = json!("LegacyType");
    }
    let unresolved = reload(&unresolved);
    assert!(matches!(namespace_imports::members_are_distinguishable(
        &ReferenceIndex::new(Model::from_project(&unresolved)), "a",
    ), Err(classifiers::QueryError::Unsupported { .. })));
    assert!(Model::from_project(&unresolved).validate().iter().any(|issue|
        issue.code == "UML_NAMESPACE_DISTINGUISHABILITY_UNSUPPORTED"));
    let repaired = apply(&unresolved, &Command::Rename {
        element_id: "right".into(), name: "alternate".into(),
    }, 5).expect("different names decide distinction without inventing Type identity");
    assert!(distinguishes(&repaired, "a"));
}

#[test]
fn namespace_flow_preserves_parameter_order_aliases_and_inherited_overloads() {
    let mut content = document();
    content["elements"].as_array_mut().expect("elements").extend([
        json!({"id":"left-second", "kind":"uml.parameter", "name":"second",
            "properties":{"type":{"$ref":"b"},"direction":"return"}}),
        json!({"id":"right-second", "kind":"uml.parameter", "name":"second",
            "properties":{"type":{"$ref":"b"},"direction":"return"}}),
        json!({"id":"package", "kind":"uml.package", "name":"Library"}),
        json!({"id":"imported", "kind":"uml.class", "name":"Original"}),
        json!({"id":"nested", "kind":"uml.class", "name":"Alias"}),
        json!({"id":"child", "kind":"uml.class", "name":"Child"}),
    ]);
    content["relations"].as_array_mut().expect("relations").extend([
        json!({"id":"owns-left-second", "kind":"owns", "from":"left", "to":"left-second"}),
        json!({"id":"owns-right-second", "kind":"owns", "from":"right", "to":"right-second"}),
        json!({"id":"owns-package", "kind":"owns", "from":"root", "to":"package"}),
        json!({"id":"owns-imported", "kind":"owns", "from":"package", "to":"imported"}),
        json!({"id":"owns-nested", "kind":"owns", "from":"a", "to":"nested"}),
        json!({"id":"owns-child", "kind":"owns", "from":"root", "to":"child"}),
        json!({"id":"child-parent", "kind":"uml.generalization", "from":"child", "to":"a"}),
    ]);
    let project = reload(&content);
    let project = apply(&project, &Command::MoveParameter {
        parameter_id: "right-second".into(), up: true,
    }, 1).expect("reorder through the canonical sequence");
    let project = apply(&project, &Command::Rename {
        element_id: "right".into(), name: "process".into(),
    }, 2).expect("sequence order distinguishes otherwise equal tuples");
    assert!(distinguishes(&project, "a"));
    assert!(distinguishes(&project, "child"));
    assert!(apply(&project, &Command::MoveParameter {
        parameter_id: "right-second".into(), up: false,
    }, 3).is_err(), "restoring equal order creates an invalid overload");

    let imported = apply(&project, &Command::SetNamespaceImport {
        import_id: "alias-import".into(), namespace_id: "a".into(),
        target_id: "imported".into(), is_package: false, alias: Some("Alias".into()),
        visibility: "public".into(),
    }, 4).expect("an imported name collision is excluded, not an owned-member error");
    let references = ReferenceIndex::new(Model::from_project(&imported));
    let namespace = classifiers::query(&references, "a").expect("effective members");
    assert!(namespace.excluded_imports.iter().any(|member| member.id() == "imported"));
    assert!(!namespace_imports::is_distinguishable_from(&references, "a", "imported", "nested")
        .expect("excluded candidates retain their aliases for comparison"));
    assert!(namespace_imports::members_are_distinguishable(&references, "a").expect("valid aliases"));
    let visible = apply(&imported, &Command::SetNamespaceImport {
        import_id: "alias-import".into(), namespace_id: "a".into(),
        target_id: "imported".into(), is_package: false, alias: Some("OtherAlias".into()),
        visibility: "public".into(),
    }, 5).expect("repair alias collision");
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&visible, format).expect("save names and order"), format)
            .expect("reopen names and order");
        assert_eq!(visible, reopened);
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        assert_eq!(namespace_imports::get_names_of_member(&references, "child", "imported")
            .expect("inherited alias"), std::collections::BTreeSet::from([Some("OtherAlias".into())]));
        assert!(distinguishes(&reopened, "child"));
    }
}

#[test]
fn imported_interaction_draft_preserves_gate_and_message_name_overrides() {
    let mut project = reload(&json!({
        "id":"interaction-draft", "name":"Namespace overrides", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"interaction", "kind":"uml.interaction", "name":"Interaction"},
            {"id":"gate", "kind":"uml.gate", "name":"boundary"},
            {"id":"other-gate", "kind":"uml.gate", "name":"anotherBoundary"},
            {"id":"message", "kind":"uml.message", "name":"delivery"},
            {"id":"other-message", "kind":"uml.message", "name":"anotherDelivery"}
        ],
        "relations":[
            {"id":"owns-interaction", "kind":"owns", "from":"root", "to":"interaction"},
            {"id":"owns-gate", "kind":"owns", "from":"interaction", "to":"gate"},
            {"id":"owns-other-gate", "kind":"owns", "from":"interaction", "to":"other-gate"},
            {"id":"owns-message", "kind":"owns", "from":"interaction", "to":"message"},
            {"id":"owns-other-message", "kind":"owns", "from":"interaction", "to":"other-message"}
        ]
    }));
    for (id, name) in [("other-gate", "boundary"), ("other-message", "delivery")] {
        project = apply(&project, &Command::Rename {
            element_id: id.into(), name: name.into(),
        }, 1).expect("the name override permits another member with this name");
        assert!(distinguishes(&project, "interaction"));
        assert!(!Model::from_project(&project).validate().iter().any(|issue|
            issue.code == "UML_NAMESPACE_MEMBERS_DISTINGUISHABLE"));
    }
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format).expect("save interaction draft"), format)
            .expect("reopen interaction draft");
        assert_eq!(project, reopened);
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        for (left, right) in [("gate", "other-gate"), ("message", "other-message")] {
            assert!(namespace_imports::is_distinguishable_from(
                &references, "interaction", left, right,
            ).expect("Gate/Message normative override"));
        }
        assert!(distinguishes(&reopened, "interaction"));
    }
}
