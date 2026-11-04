//! Association specialization and navigation through the public document APIs.
use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, project_model_section, validate_uml_model,
};
use bellatrix_mde_protocol::{
    command::*, inspector::ElementInspector, projection::ModelingProjection,
};
use serde_json::{Value, json};

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> Result<Value, String> {
    let decoded = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&decoded).map_err(|error| error.to_string())?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> Result<Vec<u8>, String> {
    encode_model_document(&envelope(model), format)
}

fn apply(model: &Value, command: &Command, revision: u64) -> Result<Value, String> {
    let document = prepare_model_document(&envelope(model))?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json")
}

fn projection(model: &Value) -> ModelingProjection {
    let document = prepare_model_document(&envelope(model)).expect("prepare canonical document");
    project_model_section(&document.id, &document.name, &document.section_json)
        .expect("project the public document")
}

fn inspect_element(model: &Value, id: &str) -> Option<ElementInspector> {
    projection(model).inspect_element(id)
}

fn inspected_property(model: &Value, id: &str, path: &str) -> Value {
    inspect_element(model, id)
        .expect("inspect canonical identity")
        .properties
        .into_iter()
        .find(|property| property.path == path)
        .expect("public inspector property")
        .value
}

fn model_diagnostics(model: &Value) -> Vec<Value> {
    let response =
        validate_uml_model(&envelope(model)).expect("validate through the kernel facade");
    let mut response: Value = serde_json::from_str(&response).expect("validation response");
    response["diagnostics"]
        .as_array_mut()
        .expect("public diagnostics")
        .to_owned()
}

fn document() -> Value {
    let mut content = json!({
        "id":"project", "name":"Association contracts", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"left","kind":"uml.class","name":"Left"},
            {"id":"right","kind":"uml.class","name":"Right"},
            {"id":"specific-left","kind":"uml.class","name":"SpecificLeft"},
            {"id":"specific-right","kind":"uml.class","name":"SpecificRight"},
            {"id":"leaf-left","kind":"uml.class","name":"LeafLeft"},
            {"id":"general","kind":"uml.nary_association","name":"General",
             "properties":{"memberEnd":[{"$ref":"general-left"},{"$ref":"general-right"}]}},
            {"id":"specific","kind":"uml.nary_association","name":"Specific",
             "properties":{"memberEnd":[{"$ref":"end-left"},{"$ref":"end-right"}]}},
            {"id":"general-left","kind":"uml.attribute","name":"generalLeft",
             "properties":{"type":{"$ref":"left"}}},
            {"id":"general-right","kind":"uml.attribute","name":"generalRight",
             "properties":{"type":{"$ref":"right"}}},
            {"id":"end-left","kind":"uml.attribute","name":"specificLeft",
             "properties":{"type":{"$ref":"leaf-left"}}},
            {"id":"end-right","kind":"uml.attribute","name":"specificRight",
             "properties":{"type":{"$ref":"specific-right"}}}
        ],
        "relations":[
            {"id":"left-parent","kind":"uml.generalization","from":"specific-left","to":"left"},
            {"id":"right-parent","kind":"uml.generalization","from":"specific-right","to":"right"},
            {"id":"leaf-parent","kind":"uml.generalization","from":"leaf-left","to":"specific-left"},
            {"id":"association-parent","kind":"uml.generalization","from":"specific","to":"general"}
        ]
    });
    for (owner, child) in [
        ("root", "left"),
        ("root", "right"),
        ("root", "specific-left"),
        ("root", "specific-right"),
        ("root", "leaf-left"),
        ("root", "general"),
        ("root", "specific"),
        ("general", "general-left"),
        ("general", "general-right"),
        ("specific", "end-left"),
        ("specific", "end-right"),
    ] {
        content["relations"]
            .as_array_mut()
            .expect("relations")
            .push(json!({
                "id":format!("owns-{child}"), "kind":"owns", "from":owner, "to":child,
            }));
    }
    decode(
        &serde_json::to_vec(&content).expect("document bytes"),
        "json",
    )
    .expect("import Association document")
}

fn external_edit(project: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let mut content = serde_json::from_slice(&encode(project, "json").expect("save document"))
        .expect("JSON document");
    edit(&mut content);
    decode(&serde_json::to_vec(&content).expect("edited bytes"), "json")
        .expect("reload authored draft")
}

fn element_mut<'a>(content: &'a mut Value, id: &str) -> &'a mut Value {
    content["elements"]
        .as_array_mut()
        .expect("elements")
        .iter_mut()
        .find(|element| element["id"] == id)
        .expect("element")
}

fn has_rule_error(project: &Value, subject: &str, rule: &str) -> bool {
    model_diagnostics(project).iter().any(|diagnostic| {
        diagnostic["severity"] == "error"
            && diagnostic["subject_id"] == subject
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains(rule))
    })
}

#[test]
fn specialization_flow_preserves_ordered_type_conformance_and_rejects_parent_arity_changes() {
    let mut project = document();
    assert!(model_diagnostics(&project).is_empty());
    project = apply(
        &project,
        &Command::SetProperty {
            element_id: "end-left".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"specific-left"}),
        },
        1,
    )
    .expect("replace a transitive subtype with a direct subtype");
    let snapshot = encode(&project, "json").expect("snapshot");
    for command in [
        Command::SetProperty {
            element_id: "end-left".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"right"}),
        },
        Command::SetAssociationMemberEnds {
            association_id: "specific".into(),
            member_end_ids: vec!["end-right".into(), "end-left".into()],
        },
        Command::SetProperty {
            element_id: "general-left".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"right"}),
        },
        Command::EditAssociationEnds {
            association_id: "general".into(),
            edits: vec![AssociationEndEdit::Insert {
                position: 2,
                end: AssociationEndDraft {
                    name: "extra".into(),
                    type_id: "left".into(),
                    owner: AssociationEndOwner::Association,
                    aggregation: Aggregation::None,
                    multiplicity: None,
                },
            }],
        },
    ] {
        let error = apply(&project, &command, 2).expect_err("reject inconsistent specialization");
        assert!(error.contains("specialized_end_"), "{error}");
        assert_eq!(encode(&project, "json").expect("after rejection"), snapshot);
    }
    let mismatched = external_edit(&project, |content| {
        element_mut(content, "general")["properties"]["memberEnd"] =
            json!([{"$ref":"general-left"},{"$ref":"general-right"},{"$ref":"third"}]);
        content["elements"]
            .as_array_mut()
            .expect("elements")
            .push(json!({
                "id":"third","kind":"uml.attribute","name":"third",
                "properties":{"type":{"$ref":"left"}},
            }));
        content["relations"]
            .as_array_mut()
            .expect("relations")
            .push(json!({
                "id":"owns-third","kind":"owns","from":"general","to":"third",
            }));
    });
    assert!(has_rule_error(
        &mismatched,
        "specific",
        "specialized_end_number"
    ));
    let swapped = external_edit(&project, |content| {
        element_mut(content, "specific")["properties"]["memberEnd"] =
            json!([{"$ref":"end-right"},{"$ref":"end-left"}]);
    });
    assert!(has_rule_error(
        &swapped,
        "specific",
        "specialized_end_types"
    ));
    let multiple_parents = external_edit(&project, |content| {
        content["elements"]
            .as_array_mut()
            .expect("elements")
            .extend([
                json!({"id":"other-general","kind":"uml.nary_association","name":"OtherGeneral",
                "properties":{"memberEnd":[{"$ref":"other-left"},{"$ref":"other-right"}]}}),
                json!({"id":"other-left","kind":"uml.attribute","name":"otherLeft",
                "properties":{"type":{"$ref":"right"}}}),
                json!({"id":"other-right","kind":"uml.attribute","name":"otherRight",
                "properties":{"type":{"$ref":"left"}}}),
            ]);
        content["relations"].as_array_mut().expect("relations").extend([
            json!({"id":"owns-other-general","kind":"owns","from":"root","to":"other-general"}),
            json!({"id":"owns-other-left","kind":"owns","from":"other-general","to":"other-left"}),
            json!({"id":"owns-other-right","kind":"owns","from":"other-general","to":"other-right"}),
            json!({"id":"other-parent","kind":"uml.generalization","from":"specific","to":"other-general"}),
        ]);
    });
    assert!(has_rule_error(
        &multiple_parents,
        "specific",
        "specialized_end_types"
    ));
    let unresolved_parent = external_edit(&project, |content| {
        assert!(
            element_mut(content, "general")["properties"]
                .as_object_mut()
                .expect("Association properties")
                .remove("memberEnd")
                .is_some()
        );
    });
    assert!(
        model_diagnostics(&unresolved_parent)
            .iter()
            .any(|diagnostic| diagnostic["subject_id"] == "specific"
                && diagnostic["severity"] == "warning"
                && diagnostic["code"] == "MDE_ASSOCIATION_UNSUPPORTED"
                && diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("no materialized memberEnd")))
    );
    project = apply(
        &project,
        &Command::Rename {
            element_id: "specific".into(),
            name: "Refined association".into(),
        },
        3,
    )
    .expect("continue authoring after rejected edits");
    for format in ["json", "protobuf"] {
        for original in [
            &project,
            &mismatched,
            &swapped,
            &multiple_parents,
            &unresolved_parent,
        ] {
            let reopened =
                decode(&encode(original, format).expect("save"), format).expect("reopen");
            assert_eq!(&reopened, original);
            assert_eq!(model_diagnostics(&reopened), model_diagnostics(original));
        }
    }
}

#[test]
fn navigation_flow_uses_canonical_roles_and_checks_inherited_association_contracts() {
    let project = document();
    assert!(inspected_property(&project, "end-left", "uml.isNavigable") == json!(false));
    let directed = external_edit(&project, |content| {
        content["elements"]
            .as_array_mut()
            .expect("elements")
            .extend([
                json!({"id":"directed-left","kind":"uml.attribute","name":"directedLeft",
                "properties":{"type":{"$ref":"left"}}}),
                json!({"id":"directed-right","kind":"uml.attribute","name":"directedRight",
                "properties":{"type":{"$ref":"right"}}}),
            ]);
        content["relations"].as_array_mut().expect("relations").extend([
            json!({"id":"directed","kind":"uml.directed_association","from":"left","to":"right",
                "properties":{"memberEnd":[{"$ref":"directed-left"},{"$ref":"directed-right"}],
                    "sourceRole":"directedLeft","targetRole":"directedRight"}}),
            json!({"id":"owns-directed","kind":"owns","from":"root","to":"directed"}),
            json!({"id":"owns-directed-left","kind":"owns","from":"directed","to":"directed-left"}),
            json!({"id":"owns-directed-right","kind":"owns","from":"directed","to":"directed-right"}),
        ]);
    });
    assert!(inspected_property(&directed, "directed-left", "uml.isNavigable") == json!(false));
    assert!(inspected_property(&directed, "directed-right", "uml.isNavigable") == json!(false));
    let diagnostics = model_diagnostics(&directed);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["severity"] == "warning"
                && diagnostic["subject_id"] == "directed"
                && diagnostic["code"] == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
                && (diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with("sourceRole:"))
                    || diagnostic["message"]
                        .as_str()
                        .is_some_and(|message| message.starts_with("targetRole:")))),
        "{diagnostics:?}"
    );
    let navigable = external_edit(&project, |content| {
        element_mut(content, "specific")["properties"]["navigableOwnedEnd"] =
            json!([{"$ref":"end-left"}]);
    });
    assert!(model_diagnostics(&navigable).is_empty());
    assert!(inspected_property(&navigable, "end-left", "uml.isNavigable") == json!(true));
    assert!(inspected_property(&navigable, "end-right", "uml.isNavigable") == json!(false));
    let inspector = inspect_element(&navigable, "end-left").expect("inspect canonical end");
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "uml.isNavigable"
                && property.value == json!(true)
                && !property.editable)
    );
    let inspector = inspect_element(&navigable, "specific").expect("inspect Association");
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "uml.navigableOwnedEnd"
                && property.value == json!([{"$ref":"end-left"}]))
    );
    let reparent = Command::SetOwner {
        element_id: "end-left".into(),
        owner_id: Some("specific-right".into()),
    };
    let snapshot = encode(&navigable, "json").expect("snapshot");
    assert!(
        apply(&navigable, &reparent, 4)
            .expect_err("navigation must remain a subset of ownedEnd")
            .contains("navigableOwnedEnd")
    );
    assert_eq!(encode(&navigable, "json").expect("rollback"), snapshot);
    let cleared = external_edit(&navigable, |content| {
        element_mut(content, "specific")["properties"]["navigableOwnedEnd"] = json!([]);
    });
    let reparented = apply(&cleared, &reparent, 5).expect("make the end classifier-owned");
    assert!(inspected_property(&reparented, "end-left", "uml.isNavigable") == json!(true));
    assert!(inspected_property(&reparented, "end-left", "uml.owningAssociation").is_null());

    let subclass = external_edit(&project, |content| {
        let association = element_mut(content, "specific");
        association["kind"] = json!("uml.association_class");
        association["properties"]["ownedEnd"] = json!([{"$ref":"end-right"},{"$ref":"end-left"}]);
        association["properties"]["navigableOwnedEnd"] = json!([{"$ref":"end-right"}]);
        content["elements"]
            .as_array_mut()
            .expect("elements")
            .push(json!({
                "id":"metadata","kind":"uml.attribute","name":"metadata",
                "properties":{"type":{"$ref":"left"}},
            }));
        content["relations"]
            .as_array_mut()
            .expect("relations")
            .push(json!({
                "id":"owns-metadata","kind":"owns","from":"specific","to":"metadata",
            }));
    });
    assert_eq!(
        inspected_property(&subclass, "specific", "uml.ownedEnd"),
        json!([{"$ref":"end-right"},{"$ref":"end-left"}])
    );
    assert!(inspected_property(&subclass, "metadata", "uml.isNavigable") == json!(true));
    assert!(inspected_property(&subclass, "end-right", "uml.isNavigable") == json!(true));
    assert!(inspected_property(&subclass, "end-left", "uml.isNavigable") == json!(false));
    assert!(inspected_property(&subclass, "metadata", "uml.association").is_null());
    assert!(inspected_property(&subclass, "metadata", "uml.owningAssociation").is_null());
    let diagnostics = model_diagnostics(&subclass);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["severity"] != "error"),
        "{diagnostics:?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["subject_id"] == "specific"
            && diagnostic["code"] == "MDE_ASSOCIATION_UNSUPPORTED"
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("inherited Association rules were checked"))
    }));
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "MDE_ASSOCIATION_NAVIGATION_UNSUPPORTED")
    );
    let snapshot = encode(&subclass, "json").expect("subclass snapshot");
    let unchecked = apply(
        &subclass,
        &Command::SetProperty {
            element_id: "end-left".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"specific-left"}),
        },
        6,
    )
    .expect_err("unimplemented specialized contracts must not be approved");
    assert!(
        unchecked.contains("MDE_ASSOCIATION_UNSUPPORTED"),
        "{unchecked}"
    );
    let invalid = apply(
        &subclass,
        &Command::SetProperty {
            element_id: "end-left".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"right"}),
        },
        6,
    )
    .expect_err("check inherited rules before specialized contracts");
    assert!(invalid.contains("specialized_end_types"), "{invalid}");
    assert_eq!(
        encode(&subclass, "json").expect("subclass rollback"),
        snapshot
    );
    let invalid_subclass = external_edit(&subclass, |content| {
        element_mut(content, "end-left")["properties"]["type"] = json!({"$ref":"right"});
    });
    assert!(has_rule_error(
        &invalid_subclass,
        "specific",
        "specialized_end_types"
    ));
    let conflicting_role = external_edit(&subclass, |content| {
        element_mut(content, "specific")["properties"]["ownedAttribute"] =
            json!([{"$ref":"end-left"}]);
    });
    assert!(has_rule_error(
        &conflicting_role,
        "specific",
        "disjoint_attributes_ends"
    ));
    let incomplete_owned_ends = external_edit(&subclass, |content| {
        element_mut(content, "specific")["properties"]["ownedEnd"] = json!([{"$ref":"end-left"}]);
    });
    assert!(has_rule_error(
        &incomplete_owned_ends,
        "specific",
        "canonically owned member ends"
    ));
    for format in ["json", "protobuf"] {
        for original in [&directed, &reparented, &subclass, &invalid_subclass] {
            let reopened =
                decode(&encode(original, format).expect("save"), format).expect("reopen");
            assert_eq!(&reopened, original);
            assert_eq!(model_diagnostics(&reopened), model_diagnostics(original));
        }
    }
}
