//! Association specialization and navigation through commands, validation, inspector and I/O.
use bellatrix_mde::{
    Format, ModelingSection, Project, StructuralSeverity,
    commands::{AssociationEndDraft, AssociationEndEdit, AssociationEndOwner, Command, apply},
    decode, encode, inspect_element,
    uml::{associations, references::ReferenceIndex, root::Model, schema::Aggregation},
};
use serde_json::{Value, json};

fn document() -> Project {
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
        ("root", "left"), ("root", "right"), ("root", "specific-left"),
        ("root", "specific-right"), ("root", "leaf-left"), ("root", "general"),
        ("root", "specific"), ("general", "general-left"), ("general", "general-right"),
        ("specific", "end-left"), ("specific", "end-right"),
    ] {
        content["relations"].as_array_mut().expect("relations").push(json!({
            "id":format!("owns-{child}"), "kind":"owns", "from":owner, "to":child,
        }));
    }
    decode(&serde_json::to_vec(&content).expect("document bytes"), Format::Json)
        .expect("import Association document")
}

fn external_edit(project: &Project, edit: impl FnOnce(&mut Value)) -> Project {
    let mut content = serde_json::from_slice(&encode(project, Format::Json)
        .expect("save document")).expect("JSON document");
    edit(&mut content);
    decode(&serde_json::to_vec(&content).expect("edited bytes"), Format::Json)
        .expect("reload authored draft")
}

fn element_mut<'a>(content: &'a mut Value, id: &str) -> &'a mut Value {
    content["elements"].as_array_mut().expect("elements").iter_mut()
        .find(|element| element["id"] == id).expect("element")
}

fn has_rule_error(project: &Project, subject: &str, rule: &str) -> bool {
    Model::from_project(project).validate().iter().any(|diagnostic|
        diagnostic.severity == StructuralSeverity::Error && diagnostic.subject_id == subject
            && diagnostic.message.contains(rule))
}

#[test]
fn specialization_flow_preserves_ordered_type_conformance_and_rejects_parent_arity_changes() {
    let mut project = document();
    assert!(Model::from_project(&project).validate().is_empty());
    project = apply(&project, &Command::SetProperty {
        element_id:"end-left".into(), path:"properties.type".into(),
        value:json!({"$ref":"specific-left"}),
    }, 1).expect("replace a transitive subtype with a direct subtype");
    let snapshot = encode(&project, Format::Json).expect("snapshot");
    for command in [
        Command::SetProperty {
            element_id:"end-left".into(), path:"properties.type".into(),
            value:json!({"$ref":"right"}),
        },
        Command::SetAssociationMemberEnds {
            association_id:"specific".into(),
            member_end_ids:vec!["end-right".into(), "end-left".into()],
        },
        Command::SetProperty {
            element_id:"general-left".into(), path:"properties.type".into(),
            value:json!({"$ref":"right"}),
        },
        Command::EditAssociationEnds {
            association_id:"general".into(),
            edits:vec![AssociationEndEdit::Insert {
                position:2,
                end:AssociationEndDraft {
                    name:"extra".into(), type_id:"left".into(),
                    owner:AssociationEndOwner::Association,
                    aggregation:Aggregation::None, multiplicity:None,
                },
            }],
        },
    ] {
        let error = apply(&project, &command, 2).expect_err("reject inconsistent specialization");
        assert!(error.contains("specialized_end_"), "{error}");
        assert_eq!(encode(&project, Format::Json).expect("after rejection"), snapshot);
    }
    let mismatched = external_edit(&project, |content| {
        element_mut(content, "general")["properties"]["memberEnd"] =
            json!([{"$ref":"general-left"},{"$ref":"general-right"},{"$ref":"third"}]);
        content["elements"].as_array_mut().expect("elements").push(json!({
            "id":"third","kind":"uml.attribute","name":"third",
            "properties":{"type":{"$ref":"left"}},
        }));
        content["relations"].as_array_mut().expect("relations").push(json!({
            "id":"owns-third","kind":"owns","from":"general","to":"third",
        }));
    });
    assert!(has_rule_error(&mismatched, "specific", "specialized_end_number"));
    let swapped = external_edit(&project, |content| {
        element_mut(content, "specific")["properties"]["memberEnd"] =
            json!([{"$ref":"end-right"},{"$ref":"end-left"}]);
    });
    assert!(has_rule_error(&swapped, "specific", "specialized_end_types"));
    let multiple_parents = external_edit(&project, |content| {
        content["elements"].as_array_mut().expect("elements").extend([
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
    assert!(has_rule_error(&multiple_parents, "specific", "specialized_end_types"));
    let unresolved_parent = external_edit(&project, |content| {
        assert!(element_mut(content, "general")["properties"].as_object_mut()
            .expect("Association properties").remove("memberEnd").is_some());
    });
    assert!(Model::from_project(&unresolved_parent).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "specific" && diagnostic.severity == StructuralSeverity::Warning
            && diagnostic.code == "MDE_ASSOCIATION_UNSUPPORTED"
            && diagnostic.message.contains("no materialized memberEnd")));
    project = apply(&project, &Command::Rename {
        element_id:"specific".into(), name:"Refined association".into(),
    }, 3).expect("continue authoring after rejected edits");
    for format in [Format::Json, Format::Protobuf] {
        for original in [&project, &mismatched, &swapped, &multiple_parents, &unresolved_parent] {
            let reopened = decode(&encode(original, format).expect("save"), format)
                .expect("reopen");
            assert_eq!(&reopened, original);
            assert_eq!(Model::from_project(&reopened).validate(),
                Model::from_project(original).validate());
        }
    }
}

#[test]
fn navigation_flow_uses_canonical_roles_and_checks_inherited_association_contracts() {
    let project = document();
    let references = ReferenceIndex::new(Model::from_project(&project));
    assert!(!associations::is_navigable(&references, "end-left").expect("non-navigable end"));
    let directed = external_edit(&project, |content| {
        content["elements"].as_array_mut().expect("elements").extend([
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
    let references = ReferenceIndex::new(Model::from_project(&directed));
    assert!(!associations::is_navigable(&references, "directed-left").expect("notation is not semantics"));
    assert!(!associations::is_navigable(&references, "directed-right").expect("arrow is not navigation"));
    let diagnostics = Model::from_project(&directed).validate();
    assert!(diagnostics.iter().all(|diagnostic|
        diagnostic.severity == StructuralSeverity::Warning
            && diagnostic.subject_id == "directed"
            && diagnostic.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
            && (diagnostic.message.starts_with("sourceRole:")
                || diagnostic.message.starts_with("targetRole:"))), "{diagnostics:?}");
    let navigable = external_edit(&project, |content| {
        element_mut(content, "specific")["properties"]["navigableOwnedEnd"] =
            json!([{"$ref":"end-left"}]);
    });
    assert!(Model::from_project(&navigable).validate().is_empty());
    let references = ReferenceIndex::new(Model::from_project(&navigable));
    assert!(associations::is_navigable(&references, "end-left").expect("explicit navigation"));
    assert!(!associations::is_navigable(&references, "end-right").expect("other end"));
    let inspector = inspect_element(&ModelingSection::from_model(&navigable), "end-left")
        .expect("inspect canonical end");
    assert!(inspector.properties.iter().any(|property|
        property.path == "uml.isNavigable" && property.value == json!(true) && !property.editable));
    let inspector = inspect_element(&ModelingSection::from_model(&navigable), "specific")
        .expect("inspect Association");
    assert!(inspector.properties.iter().any(|property|
        property.path == "uml.navigableOwnedEnd" && property.value == json!([{"$ref":"end-left"}])));
    let reparent = Command::SetOwner {
        element_id:"end-left".into(), owner_id:Some("specific-right".into()),
    };
    let snapshot = encode(&navigable, Format::Json).expect("snapshot");
    assert!(apply(&navigable, &reparent, 4).expect_err("navigation must remain a subset of ownedEnd")
        .contains("navigableOwnedEnd"));
    assert_eq!(encode(&navigable, Format::Json).expect("rollback"), snapshot);
    let cleared = external_edit(&navigable, |content| {
        element_mut(content, "specific")["properties"]["navigableOwnedEnd"] = json!([]);
    });
    let reparented = apply(&cleared, &reparent, 5).expect("make the end classifier-owned");
    let references = ReferenceIndex::new(Model::from_project(&reparented));
    assert!(associations::is_navigable(&references, "end-left").expect("classifier-owned end"));
    assert!(associations::owning_association(&references, "end-left").expect("owner inverse").is_none());

    let subclass = external_edit(&project, |content| {
        let association = element_mut(content, "specific");
        association["kind"] = json!("uml.association_class");
        association["properties"]["ownedEnd"] = json!([{"$ref":"end-right"},{"$ref":"end-left"}]);
        association["properties"]["navigableOwnedEnd"] = json!([{"$ref":"end-right"}]);
        content["elements"].as_array_mut().expect("elements").push(json!({
            "id":"metadata","kind":"uml.attribute","name":"metadata",
            "properties":{"type":{"$ref":"left"}},
        }));
        content["relations"].as_array_mut().expect("relations").push(json!({
            "id":"owns-metadata","kind":"owns","from":"specific","to":"metadata",
        }));
    });
    let references = ReferenceIndex::new(Model::from_project(&subclass));
    assert_eq!(associations::owned_ends(&references, "specific").expect("separate ownership roles")
        .iter().map(|end| end.id.as_str()).collect::<Vec<_>>(), vec!["end-right", "end-left"]);
    assert!(associations::is_navigable(&references, "metadata").expect("AssociationClass attribute"));
    assert!(associations::is_navigable(&references, "end-right").expect("AssociationClass navigable end"));
    assert!(!associations::is_navigable(&references, "end-left").expect("AssociationClass other end"));
    assert!(associations::association_of(&references, "metadata").expect("attribute is not an end").is_none());
    assert!(associations::owning_association(&references, "metadata").expect("attribute owner role").is_none());
    let diagnostics = Model::from_project(&subclass).validate();
    assert!(diagnostics.iter().all(|diagnostic| diagnostic.severity != StructuralSeverity::Error),
        "{diagnostics:?}");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.subject_id == "specific"
        && diagnostic.code == "MDE_ASSOCIATION_UNSUPPORTED"
        && diagnostic.message.contains("inherited Association rules were checked")));
    assert!(!diagnostics.iter().any(|diagnostic| diagnostic.code == "MDE_ASSOCIATION_NAVIGATION_UNSUPPORTED"));
    let snapshot = encode(&subclass, Format::Json).expect("subclass snapshot");
    let unchecked = apply(&subclass, &Command::SetProperty {
        element_id:"end-left".into(), path:"properties.type".into(),
        value:json!({"$ref":"specific-left"}),
    }, 6).expect_err("unimplemented specialized contracts must not be approved");
    assert!(unchecked.contains("MDE_ASSOCIATION_UNSUPPORTED"), "{unchecked}");
    let invalid = apply(&subclass, &Command::SetProperty {
        element_id:"end-left".into(), path:"properties.type".into(),
        value:json!({"$ref":"right"}),
    }, 6).expect_err("check inherited rules before specialized contracts");
    assert!(invalid.contains("specialized_end_types"), "{invalid}");
    assert_eq!(encode(&subclass, Format::Json).expect("subclass rollback"), snapshot);
    let invalid_subclass = external_edit(&subclass, |content| {
        element_mut(content, "end-left")["properties"]["type"] = json!({"$ref":"right"});
    });
    assert!(has_rule_error(&invalid_subclass, "specific", "specialized_end_types"));
    let conflicting_role = external_edit(&subclass, |content| {
        element_mut(content, "specific")["properties"]["ownedAttribute"] = json!([{"$ref":"end-left"}]);
    });
    assert!(has_rule_error(&conflicting_role, "specific", "disjoint_attributes_ends"));
    let incomplete_owned_ends = external_edit(&subclass, |content| {
        element_mut(content, "specific")["properties"]["ownedEnd"] = json!([{"$ref":"end-left"}]);
    });
    assert!(has_rule_error(&incomplete_owned_ends, "specific", "canonically owned member ends"));
    for format in [Format::Json, Format::Protobuf] {
        for original in [&directed, &reparented, &subclass, &invalid_subclass] {
            let reopened = decode(&encode(original, format).expect("save"), format).expect("reopen");
            assert_eq!(&reopened, original);
            assert_eq!(Model::from_project(&reopened).validate(), Model::from_project(original).validate());
        }
    }
}
