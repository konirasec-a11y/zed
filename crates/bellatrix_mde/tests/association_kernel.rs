// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2019-2026 Starion Group S.A.
// Modified fixtures from uml4net AssociationExtensionsTestFixture.cs and
// PropertyExtensionsTestFixture.cs, 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d.
// Bellatrix adds transactional commands, validation and persistence assertions.
// See docs/bellatrix/references/uml4net/{README.md,LICENSE,NOTICE}.

use bellatrix_mde::{
    Format, ModelingRelationKind, ModelingSection, Project,
    commands::{Command, apply},
    decode, encode, inspect_element, relationship_presentation,
    uml::{associations, references::ReferenceIndex, root::Model},
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
    assert!(Model::from_project(&model).validate().is_empty());
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format).expect("save"), format).expect("reopen");
        assert_eq!(reopened, model);
        assert_eq!(end_ids(&reopened, &association), ends);
        assert!(Model::from_project(&reopened).validate().is_empty());
    }
    let deleted = apply(
        &model,
        &Command::DeleteElement {
            element_id: association,
        },
        5,
    )
    .expect("delete Association");
    assert!(
        deleted.element(&ends[0]).is_none(),
        "Association-owned end cascades"
    );
    assert!(
        deleted.element(&ends[1]).is_some(),
        "Classifier-owned Property survives"
    );
    assert!(
        associations::association_of(
            &ReferenceIndex::new(Model::from_project(&deleted)),
            &ends[1]
        )
        .expect("inverse after deletion")
        .is_none()
    );
}




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

