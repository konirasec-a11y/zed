// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2019-2026 Starion Group S.A.
// Modified fixtures from uml4net AssociationExtensionsTestFixture.cs and
// PropertyExtensionsTestFixture.cs, 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d.
// Bellatrix adds transactional commands, validation and persistence assertions.
// See docs/bellatrix/references/uml4net/{README.md,LICENSE,NOTICE}.

use bellatrix_mde::{
    Format, ModelingRelationKind, ModelingSection, Project,
    commands::{
        AssociationEndDraft, AssociationEndEdit, AssociationEndOwner, Command, ModelingElementKind,
        RemovedAssociationEnd, apply,
    },
    decode, encode, inspect_element, relationship_presentation,
    uml::{associations, references::ReferenceIndex, root::Model, schema::Aggregation},
};
use serde_json::json;

fn project() -> Project {
    serde_json::from_value(json!({
        "id": "project", "name": "Kernel", "root_model": "root",
        "elements": [
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"a","kind":"uml.class","name":"A"},
            {"id":"b","kind":"uml.class","name":"B"},
            {"id":"c","kind":"uml.class","name":"C"}
        ],
        "relations": [
            {"id":"owns-a","kind":"owns","from":"root","to":"a"},
            {"id":"owns-b","kind":"owns","from":"root","to":"b"},
            {"id":"owns-c","kind":"owns","from":"root","to":"c"}
        ],
        "diagrams": [{"id":"diagram","name":"Classes","uml_di_type":"UMLClassDiagram","views":[
            {"id":"view-a","subject":"a","geometry":{"kind":"rect","x":0.0,"y":0.0,"width":100.0,"height":100.0}},
            {"id":"view-b","subject":"b","geometry":{"kind":"rect","x":300.0,"y":0.0,"width":100.0,"height":100.0}},
            {"id":"view-c","subject":"c","geometry":{"kind":"rect","x":600.0,"y":0.0,"width":100.0,"height":100.0}}
        ]}]
    })).expect("fixture")
}

fn create(project: &Project, to: &str) -> Project {
    apply(
        project,
        &Command::CreateRelation {
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::Association,
            from: "a".into(),
            to: to.into(),
        },
        0,
    )
    .expect("create association")
}

fn association_id(project: &Project) -> String {
    project
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.association")
        .expect("association")
        .id
        .clone()
}

fn end_ids(project: &Project, association_id: &str) -> Vec<String> {
    associations::member_ends(
        &ReferenceIndex::new(Model::from_project(project)),
        association_id,
    )
    .expect("canonical ends")
    .iter()
    .map(|end| end.id.clone())
    .collect()
}

fn edit(project: &Project, id: &str, path: &str, value: serde_json::Value) -> Project {
    apply(
        project,
        &Command::SetProperty {
            element_id: id.into(),
            path: path.into(),
            value,
        },
        1,
    )
    .expect("edit")
}

#[test]
fn public_association_flow_creates_edits_reparents_navigates_rejects_and_reopens() {
    let mut model = create(&project(), "b");
    let association = association_id(&model);
    let ends = end_ids(&model, &association);
    assert_eq!(ends.len(), 2);
    let names = ends.iter().map(|id| model.element(id).expect("materialized Property").name.clone())
        .collect::<Vec<_>>();
    assert!(names.iter().all(|name| !name.is_empty()));
    assert_ne!(names[0], names[1]);
    let reflexive = create(&project(), "a");
    let reflexive_ends = end_ids(&reflexive, &association_id(&reflexive));
    assert_ne!(reflexive.element(&reflexive_ends[0]).expect("first reflexive end").name,
        reflexive.element(&reflexive_ends[1]).expect("second reflexive end").name);
    model = edit(
        &model,
        &association,
        "properties.targetRole",
        json!("parts"),
    );
    model = edit(&model, &ends[1], "properties.type", json!({"$ref":"c"}));
    model = edit(&model, &ends[1], "properties.multiplicity", json!("0..*"));
    model = edit(
        &model,
        &ends[1],
        "properties.aggregation",
        json!("composite"),
    );
    model = apply(
        &model,
        &Command::SetOwner {
            element_id: ends[1].clone(),
            owner_id: Some("a".into()),
        },
        2,
    )
    .expect("reparent to opposite Type");
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert_eq!(
        associations::owned_ends(&references, &association)
            .expect("owned ends")
            .iter()
            .map(|end| end.id.as_str())
            .collect::<Vec<_>>(),
        vec![ends[0].as_str()]
    );
    assert_eq!(
        associations::association_of(&references, &ends[1])
            .expect("association inverse")
            .expect("member")
            .id(),
        association
    );
    assert_eq!(
        associations::opposite(&references, &ends[1])
            .expect("opposite")
            .expect("binary")
            .id,
        ends[0]
    );
    assert_eq!(
        associations::end_types(&references, &association)
            .expect("end Types")
            .iter()
            .map(|element| element.id())
            .collect::<Vec<_>>(),
        vec!["a", "c"]
    );
    assert!(
        associations::is_composite(model.element(&ends[1]).expect("Property")).expect("composite")
    );
    assert!(
        !associations::is_composite(model.element(&ends[0]).expect("Property"))
            .expect("opposite is not composite")
    );
    let relation = model
        .relations
        .iter()
        .find(|relation| relation.id == association)
        .expect("binary adapter");
    assert_eq!(
        (&relation.from, &relation.to),
        (&"a".to_owned(), &"c".to_owned())
    );
    assert_eq!(
        relationship_presentation(relation).notation,
        "uml.composition"
    );
    assert_eq!(
        relationship_presentation(relation)
            .target_labels
            .role
            .as_deref(),
        Some("parts")
    );
    let inspector = inspect_element(&ModelingSection::from_model(&model), &ends[1])
        .expect("navigate to real Property");
    assert_eq!(inspector.name, "parts");
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "uml.opposite"
                && property.value == json!({"$ref": ends[0]})
                && !property.editable)
    );
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "uml.association"
                && property.value == json!({"$ref": association})
                && !property.editable)
    );
    assert!(inspector.properties.iter().any(
        |property| property.path == "properties.type" && property.value == json!({"$ref":"c"})
    ));
    for command in [
        Command::SetProperty {
            element_id: association.clone(),
            path: "properties.sourceMultiplicity".into(),
            value: json!("0..2"),
        },
        Command::SetProperty {
            element_id: ends[1].clone(),
            path: "properties.type".into(),
            value: serde_json::Value::Null,
        },
        Command::SetOwner {
            element_id: ends[1].clone(),
            owner_id: Some("b".into()),
        },
        Command::SetAssociationMemberEnds {
            association_id: association.clone(),
            member_end_ids: vec![ends[0].clone(), ends[0].clone()],
        },
    ] {
        let unchanged = model.clone();
        assert!(apply(&model, &command, 3).is_err(), "{command:?}");
        assert_eq!(model, unchanged);
    }
    model = apply(
        &model,
        &Command::Rename {
            element_id: "c".into(),
            name: "Renamed C".into(),
        },
        4,
    )
    .expect("rename Type");
    let expected_warnings = [
        (ends[1].as_str(), "multiplicity"),
        (association.as_str(), "end1Aggregation"),
        (association.as_str(), "sourceRole"),
        (association.as_str(), "targetMultiplicity"),
        (association.as_str(), "targetRole"),
    ];
    let assert_projection_warnings = |project: &Project| {
        let diagnostics = Model::from_project(project).validate();
        assert_eq!(diagnostics.len(), expected_warnings.len(), "{diagnostics:?}");
        for (subject, property) in expected_warnings {
            let expected_message = format!("{property}: no normative property or registered extension contract");
            assert_eq!(diagnostics.iter().filter(|diagnostic|
                diagnostic.subject_id == subject
                    && diagnostic.severity == bellatrix_mde::StructuralSeverity::Warning
                    && diagnostic.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
                    && diagnostic.message == expected_message).count(), 1, "{diagnostics:?}");
        }
    };
    assert_projection_warnings(&model);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format).expect("save"), format).expect("reopen");
        assert_eq!(reopened, model);
        assert_eq!(end_ids(&reopened, &association), ends);
        assert_projection_warnings(&reopened);
    }
    // Reopen identity-based literal bounds, edit their values through the public
    // command path, and keep the same graph usable by Association and Operation.
    let mut bounded = model.clone();
    for (id, kind, value, owner) in [
        (
            "bound-lower",
            "uml.literal_integer",
            Some(json!(0)),
            ends[1].as_str(),
        ),
        (
            "bound-upper",
            "uml.literal_unlimited_natural",
            Some(json!("*")),
            ends[1].as_str(),
        ),
        (
            "parameter-lower",
            "uml.literal_integer",
            None,
            "literal-result",
        ),
    ] {
        let mut element: bellatrix_mde::Element = serde_json::from_value(
            json!({"id": id, "kind": kind, "name": id, "properties": {"visibility":"public"}}),
        )
        .expect("literal record");
        if let Some(value) = value {
            element.properties.insert("value".into(), value);
        }
        if owner != "root" {
            element
                .properties
                .insert("visibility".into(), serde_json::Value::Null);
        }
        bounded.elements.push(element);
        bounded.relations.push(
            serde_json::from_value(
                json!({"id":format!("owns-{id}"), "kind":"owns", "from":owner, "to":id}),
            )
            .expect("literal owner"),
        );
    }
    bounded.elements.push(
        serde_json::from_value(
            json!({"id":"literal-operation", "kind":"uml.operation", "name":"Read literals"}),
        )
        .expect("Operation"),
    );
    bounded.elements.push(serde_json::from_value(json!({"id":"literal-result", "kind":"uml.parameter", "name":"result", "properties":{"direction":"return", "lowerValue":{"$ref":"parameter-lower"}}})).expect("return Parameter"));
    for (id, from, to) in [
        ("owns-literal-operation", "a", "literal-operation"),
        ("owns-literal-result", "literal-operation", "literal-result"),
    ] {
        bounded.relations.push(
            serde_json::from_value(json!({"id":id, "kind":"owns", "from":from, "to":to}))
                .expect("Operation ownership"),
        );
    }
    let property = bounded
        .elements
        .iter_mut()
        .find(|element| element.id == ends[1])
        .expect("Property");
    property
        .properties
        .insert("lowerValue".into(), json!({"$ref":"bound-lower"}));
    property
        .properties
        .insert("upperValue".into(), json!({"$ref":"bound-upper"}));
    bounded = decode(
        &encode(&bounded, Format::Json).expect("save identity bounds"),
        Format::Json,
    )
    .expect("load identity bounds");
    let references = ReferenceIndex::new(Model::from_project(&bounded));
    use bellatrix_mde::uml::value_specifications::{self, EvaluationError};
    let returned =
        bellatrix_mde::uml::operations::return_properties(&references, "literal-operation")
            .expect("literal return");
    assert_eq!(
        returned
            .lower()
            .expect("lower")
            .expect("return")
            .to_string(),
        "0"
    );
    assert_eq!(
        returned
            .upper()
            .expect("upper")
            .expect("return")
            .to_string(),
        "1"
    );
    let huge = "1000000000000000000000000000000000000000000000000000000000000";
    bounded = edit(&bounded, "bound-lower", "properties.value", json!(huge));
    assert_eq!(
        bounded
            .relations
            .iter()
            .find(|relation| relation.id == association)
            .expect("projected relation")
            .properties
            .get("targetMultiplicity"),
        Some(&json!(format!("{huge}..*")))
    );
    assert_eq!(
        bounded
            .element(&ends[1])
            .expect("authored Property")
            .properties
            .get("multiplicity"),
        Some(&json!("0..*")),
        "old notation survives as ignored authored data"
    );
    for command in [
        Command::SetProperty {
            element_id: "bound-upper".into(),
            path: "properties.value".into(),
            value: json!(1),
        },
        Command::SetProperty {
            element_id: ends[1].clone(),
            path: "properties.multiplicity".into(),
            value: json!("1"),
        },
        Command::SetProperty {
            element_id: association.clone(),
            path: "properties.targetMultiplicity".into(),
            value: json!("1"),
        },
    ] {
        let before = bounded.clone();
        assert!(apply(&bounded, &command, 10).is_err());
        assert_eq!(bounded, before);
    }
    let mut unknown = bounded.clone();
    unknown
        .elements
        .iter_mut()
        .find(|element| element.id == "bound-lower")
        .expect("lower")
        .kind = "uml.opaque_expression".into();
    let references = ReferenceIndex::new(Model::from_project(&unknown));
    assert!(matches!(
        value_specifications::multiplicity(
            &references,
            references.element(&ends[1]).expect("Property")
        ),
        Err(EvaluationError::Unsupported { .. })
    ));
    assert!(
        Model::from_project(&unknown)
            .validate()
            .iter()
            .all(|issue| issue.severity != bellatrix_mde::StructuralSeverity::Error),
        "an unevaluated bound is not proof of an invalid interval or projection: {:?}",
        Model::from_project(&unknown).validate()
    );
    let mut invalid = bounded.clone();
    invalid
        .elements
        .iter_mut()
        .find(|element| element.id == "bound-lower")
        .expect("lower")
        .kind = "uml.literal_unlimited_natural".into();
    let references = ReferenceIndex::new(Model::from_project(&invalid));
    assert!(matches!(
        value_specifications::multiplicity(
            &references,
            references.element(&ends[1]).expect("Property")
        ),
        Err(EvaluationError::Invalid { .. })
    ));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(
            &encode(&bounded, format).expect("save literal graph"),
            format,
        )
        .expect("reopen literal graph");
        assert_eq!(reopened, bounded);
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        assert_eq!(
            value_specifications::multiplicity(
                &references,
                references.element(&ends[1]).expect("Property")
            )
            .expect("evaluated bounds")
            .lower_bound()
            .to_string(),
            huge
        );
    }
    model = edit(
        &model,
        &ends[0],
        "label",
        json!("Preserve this end metadata"),
    );
    let undo_snapshot = model.clone();
    let retained_record = model.element(&ends[0]).expect("old owned end").clone();
    let replacement = apply(
        &model,
        &Command::EditAssociationEnds {
            association_id: association.clone(),
            edits: vec![
                AssociationEndEdit::Remove {
                    property_id: ends[0].clone(),
                    disposition: RemovedAssociationEnd::ReparentToClassifier("c".into()),
                },
                AssociationEndEdit::Insert {
                    position: 0,
                    end: AssociationEndDraft {
                        name: "newOwner".into(),
                        type_id: "a".into(),
                        owner: AssociationEndOwner::Association,
                        aggregation: Aggregation::None,
                        multiplicity: Some("1".into()),
                    },
                },
            ],
        },
        5,
    )
    .expect("replace owned end atomically, retaining the old Property as an attribute");
    assert_eq!(
        model, undo_snapshot,
        "the entire pre-edit snapshot remains available for undo"
    );
    model = replacement;
    let replacement_ids = end_ids(&model, &association);
    assert_ne!(replacement_ids[0], ends[0]);
    assert_eq!(replacement_ids[1], ends[1]);
    assert_eq!(model.element(&ends[0]), Some(&retained_record));
    assert_eq!(
        Model::from_project(&model)
            .owner(
                Model::from_project(&model)
                    .element(&ends[0])
                    .expect("retained end")
            )
            .expect("owner")
            .expect("classifier")
            .id(),
        "c"
    );
    let mut with_navigation = model.clone();
    with_navigation
        .relations
        .iter_mut()
        .find(|relation| relation.id == association)
        .expect("Association")
        .properties
        .insert(
            "navigableOwnedEnd".into(),
            json!([{"$ref": replacement_ids[0]}]),
        );
    assert!(
        Model::from_project(&with_navigation)
            .validate()
            .iter()
            .all(|diagnostic| diagnostic.severity != bellatrix_mde::StructuralSeverity::Error)
    );
    assert!(
        apply(
            &with_navigation,
            &Command::EditAssociationEnds {
                association_id: association.clone(),
                edits: vec![AssociationEndEdit::Reparent {
                    property_id: replacement_ids[0].clone(),
                    owner: AssociationEndOwner::Classifier("c".into()),
                }],
            },
            5,
        )
        .is_err(),
        "reparenting must not leave a navigableOwnedEnd outside ownedEnd"
    );
    let retired_id = replacement_ids[0].clone();
    model = apply(
        &model,
        &Command::EditAssociationEnds {
            association_id: association.clone(),
            edits: vec![
                AssociationEndEdit::Remove {
                    property_id: retired_id.clone(),
                    disposition: RemovedAssociationEnd::DeleteAssociationOwned,
                },
                AssociationEndEdit::Insert {
                    position: 0,
                    end: AssociationEndDraft {
                        name: "newOwner".into(),
                        type_id: "a".into(),
                        owner: AssociationEndOwner::Association,
                        aggregation: Aggregation::None,
                        multiplicity: Some("1".into()),
                    },
                },
            ],
        },
        5,
    )
    .expect("delete and replace an owned end in one atomic command");
    let replacement_ids = end_ids(&model, &association);
    assert_ne!(
        replacement_ids[0], retired_id,
        "a replacement must not recycle the deleted identity"
    );
    assert!(model.element(&retired_id).is_none());
    for edits in [
        vec![AssociationEndEdit::Remove {
            property_id: replacement_ids[0].clone(),
            disposition: RemovedAssociationEnd::DeleteAssociationOwned,
        }],
        vec![AssociationEndEdit::Insert {
            position: 2,
            end: AssociationEndDraft {
                name: "third".into(),
                type_id: "a".into(),
                owner: AssociationEndOwner::Association,
                aggregation: Aggregation::None,
                multiplicity: None,
            },
        }],
    ] {
        let before = model.clone();
        assert!(
            apply(
                &model,
                &Command::EditAssociationEnds {
                    association_id: association.clone(),
                    edits
                },
                6
            )
            .is_err()
        );
        assert_eq!(
            model, before,
            "no partial deletion or insertion is published"
        );
    }
    model = apply(
        &model,
        &Command::EditAssociationEnds {
            association_id: association.clone(),
            edits: vec![
                AssociationEndEdit::Retype {
                    property_id: replacement_ids[0].clone(),
                    type_id: "b".into(),
                },
                AssociationEndEdit::Reparent {
                    property_id: ends[1].clone(),
                    owner: AssociationEndOwner::Classifier("b".into()),
                },
            ],
        },
        7,
    )
    .expect("change opposite Type and classifier owner in the same transaction");
    assert_eq!(end_ids(&model, &association), replacement_ids);
    let detached_record = model.element(&ends[1]).expect("class-owned end").clone();
    model = apply(
        &model,
        &Command::EditAssociationEnds {
            association_id: association.clone(),
            edits: vec![
                AssociationEndEdit::Remove {
                    property_id: ends[1].clone(),
                    disposition: RemovedAssociationEnd::KeepClassifierOwned,
                },
                AssociationEndEdit::Attach {
                    position: 1,
                    property_id: ends[0].clone(),
                    owner: Some(AssociationEndOwner::Classifier("b".into())),
                },
                AssociationEndEdit::Retype {
                    property_id: ends[0].clone(),
                    type_id: "c".into(),
                },
                AssociationEndEdit::Move {
                    property_id: ends[0].clone(),
                    position: 0,
                },
            ],
        },
        8,
    )
    .expect("detach a class-owned end without deleting it and reuse another Property identity");
    assert_eq!(model.element(&ends[1]), Some(&detached_record));
    assert_eq!(
        end_ids(&model, &association),
        vec![ends[0].clone(), replacement_ids[0].clone()]
    );
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format).expect("save replaced ends"), format)
            .expect("reopen replacement");
        assert_eq!(reopened, model);
        let restored = decode(
            &encode(&undo_snapshot, format).expect("save undo state"),
            format,
        )
        .expect("restore prior snapshot");
        assert_eq!(restored, undo_snapshot);
        assert_eq!(end_ids(&restored, &association), ends);
    }
    let deleted = apply(
        &model,
        &Command::DeleteElement {
            element_id: association,
        },
        9,
    )
    .expect("delete Association");
    assert!(
        deleted.element(&replacement_ids[0]).is_none(),
        "current Association-owned end cascades"
    );
    assert!(
        deleted.element(&ends[0]).is_some() && deleted.element(&ends[1]).is_some(),
        "retained classifier-owned Properties survive"
    );
    assert!(
        associations::association_of(
            &ReferenceIndex::new(Model::from_project(&deleted)),
            &ends[0]
        )
        .expect("inverse after deletion")
        .is_none()
    );

    let mut nary = apply(
        &deleted,
        &Command::CreateElement {
            diagram_id: "diagram".into(),
            owner_id: Some("root".into()),
            kind: ModelingElementKind::NaryAssociation,
            name: Some("Participants".into()),
            x: 100.0,
            y: 250.0,
        },
        10,
    )
    .expect("create n-ary Association draft");
    let nary_id = nary
        .elements
        .iter()
        .find(|element| element.kind == "uml.nary_association")
        .expect("n-ary identity")
        .id
        .clone();
    let draft_diagnostics = Model::from_project(&nary).validate();
    assert!(draft_diagnostics.iter().any(|diagnostic| diagnostic.subject_id == nary_id
        && diagnostic.code == "MDE_RELATIONSHIP_ENDPOINTS_UNVERIFIED"
        && diagnostic.severity == bellatrix_mde::StructuralSeverity::Warning));
    let mut invalid_ends = nary.clone();
    invalid_ends.elements.iter_mut().find(|element| element.id == nary_id)
        .expect("Association draft").properties.insert("memberEnd".into(), json!([]));
    assert!(Model::from_project(&invalid_ends).validate().iter().any(|diagnostic|
        diagnostic.subject_id == nary_id && diagnostic.code == "UML_RELATIONSHIP_ENDPOINTS"
            && diagnostic.severity == bellatrix_mde::StructuralSeverity::Error));
    nary = apply(
        &nary,
        &Command::EditAssociationEnds {
            association_id: nary_id.clone(),
            edits: ["a", "b", "c"]
                .into_iter()
                .enumerate()
                .map(|(position, type_id)| AssociationEndEdit::Insert {
                    position,
                    end: AssociationEndDraft {
                        name: format!("participant{position}"),
                        type_id: type_id.into(),
                        owner: AssociationEndOwner::Association,
                        aggregation: Aggregation::None,
                        multiplicity: None,
                    },
                })
                .collect(),
        },
        11,
    )
    .expect("materialize all three ends atomically without pretending binary rendering");
    let nary_ids = end_ids(&nary, &nary_id);
    assert_eq!(nary_ids.len(), 3);
    assert!(
        associations::opposite(
            &ReferenceIndex::new(Model::from_project(&nary)),
            &nary_ids[0]
        )
        .expect("n-ary query")
        .is_none()
    );
    nary = apply(
        &nary,
        &Command::EditAssociationEnds {
            association_id: nary_id.clone(),
            edits: vec![AssociationEndEdit::Remove {
                property_id: nary_ids[1].clone(),
                disposition: RemovedAssociationEnd::DeleteAssociationOwned,
            }],
        },
        12,
    )
    .expect("remove one owned end while preserving the two-end minimum");
    assert!(nary.element(&nary_ids[1]).is_none());
    assert_eq!(
        end_ids(&nary, &nary_id),
        vec![nary_ids[0].clone(), nary_ids[2].clone()]
    );
    assert!(Model::from_project(&nary).validate().is_empty());
    for format in [Format::Json, Format::Protobuf] {
        assert_eq!(
            decode(&encode(&nary, format).expect("save n-ary edit"), format)
                .expect("reopen n-ary edit"),
            nary
        );
    }
    // Specialize the Association and redefine its owned end. ownedEnd is a
    // Feature, not an attribute, and its redefinition context is the Association.
    let mut specialized = nary.clone();
    for element in [
        json!({"id":"specialized-association","kind":"uml.nary_association","name":"SpecializedParticipants","properties":{"memberEnd":[{"$ref":"specialized-a"},{"$ref":"specialized-c"}]}}),
        json!({"id":"specialized-a","kind":"uml.attribute","name":"refinedA","properties":{"type":{"$ref":"a"}}}),
        json!({"id":"specialized-c","kind":"uml.attribute","name":"refinedC","properties":{"type":{"$ref":"c"}}}),
    ] {
        specialized
            .elements
            .push(serde_json::from_value(element).expect("import specialization"));
    }
    for (kind, from, to) in [
        ("owns", "root", "specialized-association"),
        ("owns", "specialized-association", "specialized-a"),
        ("owns", "specialized-association", "specialized-c"),
        (
            "uml.generalization",
            "specialized-association",
            nary_id.as_str(),
        ),
    ] {
        specialized.relations.push(bellatrix_mde::Relation {
            id: format!("specialization-{kind}-{to}"),
            kind: kind.into(),
            from: from.into(),
            to: to.into(),
            properties: Default::default(),
        });
    }
    let redefine_end = Command::SetRedefinedElements {
        element_id: "specialized-a".into(),
        redefined_ids: vec![nary_ids[0].clone()],
    };
    specialized =
        apply(&specialized, &redefine_end, 13).expect("redefine inherited Association-owned end");
    let snapshot = encode(&specialized, Format::Json).expect("snapshot");
    assert!(
        apply(
            &specialized,
            &Command::SetOwner {
                element_id: "specialized-a".into(),
                owner_id: Some("c".into())
            },
            14
        )
        .is_err(),
        "end reparenting must preserve a valid redefinition context"
    );
    assert_eq!(
        encode(&specialized, Format::Json).expect("atomic"),
        snapshot
    );
    let mut cyclic = specialized.clone();
    cyclic.relations.push(bellatrix_mde::Relation {
        id: "association-cycle".into(),
        kind: "uml.generalization".into(),
        from: nary_id.clone(),
        to: "specialized-association".into(),
        properties: Default::default(),
    });
    assert!(
        apply(&cyclic, &redefine_end, 15)
            .expect_err("cycle")
            .contains("cycle in UML generalization")
    );
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(
            &encode(&specialized, format).expect("save specialization"),
            format,
        )
        .expect("reopen specialization");
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        let result = bellatrix_mde::uml::classifiers::query(&references, "specialized-association")
            .expect("specialized members");
        let classifier = result.classifier.expect("Association Classifier");
        assert!(classifier.attributes.is_empty());
        assert!(
            classifier
                .all_features
                .iter()
                .any(|feature| feature.id() == "specialized-a")
        );
        assert!(
            !classifier
                .all_features
                .iter()
                .any(|feature| feature.id() == nary_ids[0])
        );
        let inspector = inspect_element(&ModelingSection::from_model(&reopened), "specialized-a")
            .expect("end Inspector");
        assert_eq!(
            inspector
                .properties
                .iter()
                .find(|property| property.path == "uml.redefinitionContext")
                .expect("derived context")
                .value,
            json!([{"$ref":"specialized-association"}])
        );
        assert!(
            Model::from_project(&reopened)
                .validate()
                .iter()
                .all(|issue| issue.severity != bellatrix_mde::StructuralSeverity::Error)
        );
        assert!(
            reopened
                .elements
                .iter()
                .all(|element| !element.properties.contains_key("redefinitionContext"))
        );
    }}

#[test]
fn legacy_migration_preserves_authored_data_and_uses_the_opposite_composite_bound() {
    let mut legacy = project();
    legacy.relations.push(serde_json::from_value(json!({"id":"legacy","kind":"uml.association","from":"a","to":"b","properties":{
        "name":"Authored association", "sourceRole":"owner", "targetRole":"items",
        "sourceMultiplicity":" 01 ", "targetMultiplicity":"*", "end1Aggregation":"composite",
        "custom":{"preserve":"this"}
    }})).expect("legacy"));
    legacy.relations.push(
        serde_json::from_value(
            json!({"id":"owns-legacy","kind":"owns","from":"root","to":"legacy"}),
        )
        .expect("legacy owner"),
    );
    // Existing missing roles remain authorial absence, unlike names generated
    // for a new relation. Reject the indistinguishable migration atomically;
    // explicitly naming one end repairs it without silently naming the other.
    let mut unnamed = legacy.clone();
    let relation = unnamed.relations.iter_mut().find(|relation| relation.id == "legacy")
        .expect("existing legacy relation");
    relation.properties.remove("sourceRole");
    relation.properties.remove("targetRole");
    let unnamed_snapshot = unnamed.clone();
    let error = apply(&unnamed, &Command::MigrateAssociation { association_id: "legacy".into() }, 0)
        .expect_err("migration cannot silently rename absent roles");
    assert!(error.contains("UML_NAMESPACE_MEMBERS_DISTINGUISHABLE"), "{error}");
    assert_eq!(unnamed, unnamed_snapshot);
    let repaired = apply(&unnamed, &Command::SetProperty {
        element_id: "legacy".into(), path: "properties.targetRole".into(), value: json!("authoredTarget"),
    }, 1).expect("author one distinguishing role while materializing");
    let repaired_ends = end_ids(&repaired, "legacy");
    assert_eq!(repaired.element(&repaired_ends[0]).expect("unnamed source").name, "");
    assert_eq!(repaired.element(&repaired_ends[1]).expect("authored target").name, "authoredTarget");
    for format in [Format::Json, Format::Protobuf] {
        assert_eq!(decode(&encode(&repaired, format).expect("save repaired roles"), format)
            .expect("reopen repaired roles"), repaired);
    }
    let before = legacy.clone();
    let model = apply(
        &legacy,
        &Command::MigrateAssociation {
            association_id: "legacy".into(),
        },
        0,
    )
    .expect("migration");
    assert_eq!(legacy, before);
    let ends = end_ids(&model, "legacy");
    assert_eq!(model.element(&ends[0]).expect("source").name, "owner");
    assert_eq!(
        model
            .element(&ends[1])
            .expect("target")
            .properties
            .get("multiplicity"),
        Some(&json!("*"))
    );
    assert!(!associations::is_composite(model.element(&ends[0]).expect("source")).expect("source"));
    assert!(associations::is_composite(model.element(&ends[1]).expect("target")).expect("target"));
    let relation = model
        .relations
        .iter()
        .find(|relation| relation.id == "legacy")
        .expect("same identity");
    assert_eq!(
        relation.properties.get("custom"),
        Some(&json!({"preserve":"this"}))
    );
    assert_eq!(
        relation.properties.get("sourceMultiplicity"),
        Some(&json!(" 01 "))
    );
    assert_eq!(
        apply(
            &model,
            &Command::MigrateAssociation {
                association_id: "legacy".into()
            },
            0
        )
        .expect("idempotent"),
        model
    );
    let result = apply(
        &model,
        &Command::SetProperty {
            element_id: "legacy".into(),
            path: "properties.sourceMultiplicity".into(),
            value: json!("*"),
        },
        1,
    );
    assert!(
        result
            .expect_err("composing opposite at most one")
            .contains("multiplicity_of_composite")
    );
    let result = apply(
        &model,
        &Command::SetProperty {
            element_id: "legacy".into(),
            path: "properties.targetMultiplicity".into(),
            value: json!("3..2"),
        },
        1,
    );
    assert!(
        result
            .expect_err("invalid interval")
            .contains("multiplicity")
    );
    for format in [Format::Json, Format::Protobuf] {
        assert_eq!(
            decode(&encode(&model, format).expect("save"), format).expect("reopen"),
            model
        );
    }
}
