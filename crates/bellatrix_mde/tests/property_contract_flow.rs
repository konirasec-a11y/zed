//! Feature flows through public commands, inspector, canonical validation and I/O.
//! Normative contracts: UML 2.5.1 Property metadata, Constraint and OpaqueExpression.
use bellatrix_mde::{
    Format, ModelingSection, Project, StructuralSeverity,
    commands::{Command, apply}, decode, encode, inspect_element,
    uml::root::Model,
};
use serde_json::{Value, json};

fn document() -> Project {
    decode(&serde_json::to_vec(&json!({
        "id":"project", "name":"Contracts", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"a","kind":"uml.class","name":"Customer"},
            {"id":"b","kind":"uml.class","name":"Failure"},
            {"id":"operation","kind":"uml.operation","name":"submit",
             "properties":{"raisedException":[{"$ref":"b"}]}},
            {"id":"parameter","kind":"uml.parameter","name":"input",
             "properties":{"type":{"$ref":"a"}}},
            {"id":"condition","kind":"uml.constraint","name":"Allowed",
             "properties":{"specification":{"$ref":"literal"},"constrainedElement":[{"$ref":"a"}]}},
            {"id":"literal","kind":"uml.literal_boolean","name":"",
             "properties":{"value":false,"visibility":null}},
            {"id":"expression","kind":"uml.opaque_expression","name":"",
             "properties":{"body":["value","value"],"language":["Rust","SQL"]}}
        ],
        "relations":[
            {"id":"owns-a","kind":"owns","from":"root","to":"a"},
            {"id":"owns-b","kind":"owns","from":"root","to":"b"},
            {"id":"owns-operation","kind":"owns","from":"a","to":"operation"},
            {"id":"owns-parameter","kind":"owns","from":"operation","to":"parameter"},
            {"id":"owns-condition","kind":"owns","from":"a","to":"condition"},
            {"id":"owns-literal","kind":"owns","from":"condition","to":"literal"},
            {"id":"owns-expression","kind":"owns","from":"root","to":"expression"}
        ]
    })).expect("document bytes"), Format::Json).expect("import document")
}

fn external_edit(project: &Project, subject: &str, property: &str, value: Value) -> Project {
    let mut document: Value = serde_json::from_slice(&encode(project, Format::Json)
        .expect("save before external edit")).expect("JSON document");
    let element = document["elements"].as_array_mut().expect("elements")
        .iter_mut().find(|element| element["id"].as_str() == Some(subject)).expect("subject");
    element["properties"][property] = value;
    decode(&serde_json::to_vec(&document).expect("edited bytes"), Format::Json)
        .expect("reload authored draft")
}

fn has_error(project: &Project, subject: &str, rule: &str) -> bool {
    Model::from_project(project).validate().iter().any(|diagnostic|
        diagnostic.severity == StructuralSeverity::Error
            && diagnostic.subject_id == subject && diagnostic.message.contains(rule))
}

#[test]
fn structural_document_flow_edits_inspects_rejects_bad_references_and_reopens_drafts() {
    // Given one canonical document, including an ordered non-unique body and a
    // Boolean false constraint expression (false is not a metamodel type error).
    let mut project = document();
    assert!(!has_error(&project, "expression", "OpaqueExpression-body"));
    assert!(!has_error(&project, "condition", "Constraint-boolean_value"));

    // When authoring through the real command and inspector contracts.
    project = apply(&project, &Command::Rename {
        element_id: "a".into(), name: "Account".into(),
    }, 1).expect("rename through the domain");
    project = apply(&project, &Command::SetProperty {
        element_id: "parameter".into(), path: "properties.type".into(), value: json!({"$ref":"b"}),
    }, 2).expect("choose a canonical Type");
    let inspector = inspect_element(&ModelingSection::from_model(&project), "operation")
        .expect("inspect Operation");
    assert!(inspector.parameters.iter().any(|parameter| parameter.id == "parameter"));
    let before = encode(&project, Format::Json).expect("before rejection");
    assert!(apply(&project, &Command::SetProperty {
        element_id: "parameter".into(), path: "properties.type".into(),
        value: json!({"$ref":"condition"}),
    }, 3).is_err());
    assert_eq!(encode(&project, Format::Json).expect("after rejection"), before);

    // Then externally authored collections are validated as part of the whole
    // document, not silently accepted because the scalar editor cannot edit them.
    for invalid in [json!([{"$ref":"b"},{"$ref":"b"}]),
                    json!([{"$ref":"missing"}]), json!([{"$ref":"condition"}]), json!("b")] {
        let draft = external_edit(&project, "operation", "raisedException", invalid);
        assert!(has_error(&draft, "operation", "Operation-raisedException"));
        let reopened = decode(&encode(&draft, Format::Json).expect("save invalid draft"),
            Format::Json).expect("reopen invalid draft");
        assert_eq!(draft, reopened);
        assert!(has_error(&reopened, "operation", "Operation-raisedException"));
    }
    let bad_body = external_edit(&project, "expression", "body", json!(["value", 12]));
    assert!(has_error(&bad_body, "expression", "OpaqueExpression-body"));
    let duplicate_languages = external_edit(&project, "expression", "language", json!(["Rust","Rust"]));
    assert!(has_error(&duplicate_languages, "expression", "OpaqueExpression-language"));
    let mismatched_languages = external_edit(&project, "expression", "language", json!(["Rust"]));
    assert!(has_error(&mismatched_languages, "expression", "OpaqueExpression-language_body_size"));

    // Unsupported declarations remain visible and round-trip, but are not
    // presented as a checked UML contract. Repair uses the same command path.
    let unresolved = external_edit(&project, "parameter", "type", json!("LegacyName"));
    assert!(Model::from_project(&unresolved).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "parameter"
            && diagnostic.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"));
    project = apply(&unresolved, &Command::SetProperty {
        element_id: "parameter".into(), path: "properties.type".into(), value: json!({"$ref":"b"}),
    }, 4).expect("resolve preserved declaration");
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format).expect("save"), format).expect("reopen");
        assert_eq!(project, reopened);
        assert!(!has_error(&reopened, "operation", "Operation-raisedException"));
        assert_eq!(reopened.element("a").expect("same identity").name, "Account");
    }
}

#[test]
fn constraint_authoring_flow_keeps_identity_ownership_boolean_contract_and_recovery() {
    let project = document();
    let self_constrained = external_edit(&project, "condition", "constrainedElement",
        json!([{"$ref":"condition"}]));
    assert!(has_error(&self_constrained, "condition", "Constraint-not_apply_to_self"));
    let wrong_owner = external_edit(&project, "condition", "specification", json!({"$ref":"expression"}));
    assert!(has_error(&wrong_owner, "condition", "Constraint-specification"));

    // Reload a draft whose expression has the wrong UML value type, repair it,
    // then edit through the public command without changing its semantic ID.
    let mut content = serde_json::to_value(&project).expect("document");
    let literal = content["elements"].as_array_mut().expect("elements").iter_mut()
        .find(|element| element["id"] == "literal").expect("literal");
    literal["kind"] = json!("uml.literal_integer");
    literal["properties"]["value"] = json!(7);
    let draft = decode(&serde_json::to_vec(&content).expect("draft"), Format::Json).expect("reload");
    assert!(has_error(&draft, "condition", "Constraint-boolean_value"));
    let draft = decode(&encode(&draft, Format::Protobuf).expect("save draft"),
        Format::Protobuf).expect("reopen draft");
    assert!(has_error(&draft, "condition", "Constraint-boolean_value"));
    let repaired = decode(&encode(&project, Format::Json).expect("repaired document"),
        Format::Json).expect("reload repair");
    let repaired = apply(&repaired, &Command::Rename {
        element_id: "condition".into(), name: "MaySubmit".into(),
    }, 5).expect("edit after repair");
    let restored = decode(&encode(&repaired, Format::Json).expect("save repair"),
        Format::Json).expect("reopen repair");
    assert_eq!(restored.element("condition").expect("same Constraint").name, "MaySubmit");
    assert!(!has_error(&restored, "condition", "Constraint-boolean_value"));
    assert!(!has_error(&restored, "condition", "Constraint-not_apply_to_self"));
}
