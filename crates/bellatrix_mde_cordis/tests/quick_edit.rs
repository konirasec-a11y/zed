// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2019-2026 Starion Group S.A.
// Operation return scenarios adapted from uml4net OperationExtensionsTestFixture.cs,
// 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d, inside the existing public editing flow.
// Provenance and license: docs/bellatrix/references/uml4net/.

use bellatrix_mde::{
    ModelingSection, Project, element_display_label, element_display_label_with_references,
    inspect_element, member_presentation,
};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingAddMember, ModelingAddMemberRequested, ModelingDeleteElement,
    ModelingDeleteElementRequested, ModelingMemberKind, ModelingMoveParameter,
    ModelingMoveParameterRequested, ModelingQuickEdit, ModelingQuickEditRequested,
    ModelingSetProperty, ModelingSetPropertyRequested, UmlQuickCreate, with_model,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin,
    ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

async fn admit<P: Plugin>(ctx: &Context, plugin: P) -> cordis::FiberHandle
where
    P::Config: Default,
    P::PrepareError: std::fmt::Debug,
{
    let input = plugin.prepare(P::Config::default()).unwrap();
    ctx.spawn(PreparedPlugin::from_input(plugin, input))
        .await
        .unwrap()
}
fn model_document(id: &str) -> ProjectDocument {
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
    model.id = id.into();
    model.name = id.into();
    with_model(
        ProjectDocument::new(ProjectId::new(id).unwrap(), id).unwrap(),
        &model,
    )
    .unwrap()
}

#[tokio::test]
async fn class_quick_edit_creates_owned_attribute_and_operation_through_cordis() {
    let ctx = Context::new();
    let projects = Arc::new(ProjectService::new());
    let storage = admit(&ctx, ProjectPlugin::new(Arc::clone(&projects))).await;
    let mde = admit(&ctx, MdePlugin).await;
    for id in ["quick-edit-a", "quick-edit-b"] {
        assert!(matches!(
            ctx.query::<ProjectCreate>(
                Routing::Unscoped,
                ProjectCreateRequested {
                    document: model_document(id)
                }
            )
            .await
            .unwrap(),
            QueryOutcome::Answer(Ok(_))
        ));
    }
    let a = ProjectId::new("quick-edit-a").unwrap();
    let b = ProjectId::new("quick-edit-b").unwrap();
    let request = |kind, rev| ModelingAddMemberRequested {
        project_id: a.clone(),
        owner_id: "type-list".into(),
        kind,
        expected_revision: rev,
    };
    let attr_req = request(ModelingMemberKind::Attribute, 0);
    assert!(matches!(
        ctx.query::<ModelingAddMember>(Routing::Unscoped, attr_req.clone())
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(1))
    ));
    assert!(matches!(
        ctx.query::<ModelingAddMember>(Routing::Unscoped, attr_req)
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    assert!(matches!(
        ctx.query::<ModelingAddMember>(
            Routing::Unscoped,
            request(ModelingMemberKind::Operation, 1)
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(2))
    ));
    let snapshot = projects.snapshot(&a).unwrap();
    assert_eq!(snapshot.revision(), 2);
    assert_eq!(projects.snapshot(&b).unwrap().revision(), 0);
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    for (name, kind) in [
        ("Attribute1", "rust.field"),
        ("Operation1", "rust.function"),
    ] {
        let element = section
            .elements
            .iter()
            .find(|e| e.name == name && e.kind == kind)
            .unwrap();
        assert!(
            section
                .relations
                .iter()
                .any(|r| r.kind == "owns" && r.from == "type-list" && r.to == element.id)
        );
    }
    assert!(section.elements.iter().any(|e| e.id == "file-task"));
    let attribute = section
        .elements
        .iter()
        .find(|element| element.name == "Attribute1")
        .expect("created attribute");
    let operation = section
        .elements
        .iter()
        .find(|element| element.name == "Operation1")
        .expect("created operation");
    assert_eq!(element_display_label(attribute), "+Attribute1");
    assert_eq!(element_display_label(operation), "+Operation1()");
    let type_id = "type-task".to_owned();
    let inspection = inspect_element(&section, &attribute.id).expect("Property inspector");
    assert!(
        inspection
            .reference_choices_for("properties.type")
            .iter()
            .any(|choice| choice.id == type_id)
    );
    let mut revision = 2;
    for (element_id, path, value) in [
        (&attribute.id, "name", serde_json::json!("limit")),
        (
            &attribute.id,
            "properties.visibility",
            serde_json::json!("private"),
        ),
        (
            &attribute.id,
            "properties.type",
            serde_json::json!({"$ref":type_id}),
        ),
        (&type_id, "name", serde_json::json!("Integer")),
        (
            &attribute.id,
            "properties.defaultValue",
            serde_json::json!("10"),
        ),
        (
            &attribute.id,
            "properties.isReadOnly",
            serde_json::json!(true),
        ),
        (
            &attribute.id,
            "properties.isStatic",
            serde_json::json!(true),
        ),
        (&operation.id, "name", serde_json::json!("find")),
        (&operation.id, "properties.isQuery", serde_json::json!(true)),
        (
            &operation.id,
            "properties.isStatic",
            serde_json::json!(true),
        ),
    ] {
        let result = ctx
            .query::<ModelingSetProperty>(
                Routing::Unscoped,
                ModelingSetPropertyRequested {
                    project_id: a.clone(),
                    element_id: element_id.clone(),
                    path: path.into(),
                    value,
                    expected_revision: revision,
                },
            )
            .await
            .expect("property response");
        revision += 1;
        assert!(matches!(result, QueryOutcome::Answer(Ok(value)) if value == revision));
    }
    let before_invalid = projects.snapshot(&a).expect("before rejected edits");
    for (element_id, value) in [
        (&attribute.id, serde_json::json!({"$ref":"missing"})),
        (&attribute.id, serde_json::json!({"$ref":"file-task"})),
        (&attribute.id, serde_json::json!("Integer")),
        (&operation.id, serde_json::json!({"$ref":type_id})),
    ] {
        assert!(matches!(
            ctx.query::<ModelingSetProperty>(
                Routing::Unscoped,
                ModelingSetPropertyRequested {
                    project_id: a.clone(),
                    element_id: element_id.clone(),
                    path: "properties.type".into(),
                    value,
                    expected_revision: revision
                }
            )
            .await
            .expect("rejected type response"),
            QueryOutcome::Answer(Err(_))
        ));
        let after = projects.snapshot(&a).expect("unchanged snapshot");
        assert_eq!(after.revision(), revision);
        assert_eq!(
            serde_json::to_value(after.as_ref()).expect("after"),
            serde_json::to_value(before_invalid.as_ref()).expect("before")
        );
    }
    let edited = projects.snapshot(&a).expect("edited project");
    let reopened: ProjectDocument =
        serde_json::from_slice(&serde_json::to_vec(edited.as_ref()).expect("serialize project"))
            .expect("reopen project");
    let projected: ModelingSection = serde_json::from_value(
        reopened
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .expect("modeling section")
            .clone(),
    )
    .expect("reopened model");
    let references = bellatrix_mde::uml::references::ReferenceIndex::new(
        bellatrix_mde::uml::root::Model::from_section(&projected),
    );
    let owner = inspect_element(&projected, "type-list").expect("classifier compartments");
    for (member_id, expected, rows) in [
        (
            &attribute.id,
            "-limit: Integer = 10 {readOnly}",
            &owner.attributes,
        ),
        (&operation.id, "+find(): {query}", &owner.operations),
    ] {
        let member = projected
            .elements
            .iter()
            .find(|element| &element.id == member_id)
            .expect("persisted member");
        assert_eq!(
            element_display_label_with_references(member, &references),
            expected
        );
        assert_eq!(
            bellatrix_mde::project_hierarchy(&projected)
                .iter()
                .find(|row| &row.element_id == member_id)
                .expect("tree row")
                .label,
            expected
        );
        assert_eq!(
            inspect_element(&projected, member_id)
                .expect("member inspector")
                .display_label,
            expected
        );
        assert_eq!(
            rows.iter()
                .find(|row| &row.id == member_id)
                .expect("compartment row")
                .display_label,
            expected
        );
        assert!(member_presentation(member).underlined);
    }
    assert_eq!(projects.snapshot(&b).expect("other project").revision(), 0);
    for (element_id, path, value) in [
        (&attribute.id, "properties.type", serde_json::Value::Null),
        (
            &attribute.id,
            "properties.defaultValue",
            serde_json::json!(""),
        ),
        (
            &attribute.id,
            "properties.isReadOnly",
            serde_json::json!(false),
        ),
        (
            &attribute.id,
            "properties.isStatic",
            serde_json::json!(false),
        ),
        (
            &operation.id,
            "properties.isQuery",
            serde_json::json!(false),
        ),
        (
            &operation.id,
            "properties.isStatic",
            serde_json::json!(false),
        ),
        (&operation.id, "name", serde_json::json!("findAgain")),
    ] {
        let result = ctx
            .query::<ModelingSetProperty>(
                Routing::Unscoped,
                ModelingSetPropertyRequested {
                    project_id: a.clone(),
                    element_id: element_id.clone(),
                    path: path.into(),
                    value,
                    expected_revision: revision,
                },
            )
            .await
            .expect("repeat property response");
        revision += 1;
        assert!(matches!(result, QueryOutcome::Answer(Ok(value)) if value == revision));
    }
    let reset = projects.snapshot(&a).expect("reset project");
    let reset: ModelingSection = serde_json::from_value(
        reset
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .expect("reset section")
            .clone(),
    )
    .expect("reset model");
    for (member_id, expected) in [(&attribute.id, "-limit"), (&operation.id, "+findAgain()")] {
        let member = reset
            .elements
            .iter()
            .find(|element| &element.id == member_id)
            .expect("reset member");
        assert_eq!(element_display_label(member), expected);
        assert!(!member_presentation(member).underlined);
    }
    // Continue the same public editing flow into Operation Parameters and persistence.
    let read_section = || -> ModelingSection {
        serde_json::from_value(
            projects
                .snapshot(&a)
                .expect("project")
                .section(bellatrix_mde::MODELING_NAMESPACE)
                .expect("section")
                .clone(),
        )
        .expect("model")
    };
    let no_return = inspect_element(&read_section(), &operation.id).expect("no return yet");
    for (key, expected) in [
        ("isOrdered", serde_json::json!(false)),
        ("isUnique", serde_json::json!(true)),
        ("lower", serde_json::Value::Null),
        ("upper", serde_json::Value::Null),
    ] {
        assert!(
            no_return
                .properties
                .iter()
                .any(|field| field.path == format!("properties.{key}")
                    && field.value == expected
                    && !field.editable)
        );
    }
    let raw_signature = operation.properties.get("signature").cloned();
    let mut parameter_ids = Vec::new();
    for number in 1..=3 {
        let request = ModelingAddMemberRequested {
            project_id: a.clone(),
            owner_id: operation.id.clone(),
            kind: ModelingMemberKind::Parameter,
            expected_revision: revision,
        };
        assert!(
            matches!(ctx.query::<ModelingAddMember>(Routing::Unscoped, request).await.expect("add parameter"),
            QueryOutcome::Answer(Ok(value)) if value == revision + 1)
        );
        revision += 1;
        let section = read_section();
        let inspector = inspect_element(&section, &operation.id).expect("Operation inspector");
        let parameter = inspector
            .parameters
            .iter()
            .find(|parameter| parameter.name == format!("Parameter{number}"))
            .expect("new Parameter identity");
        let parameter_inspector =
            inspect_element(&section, &parameter.id).expect("Parameter inspector");
        let direction = parameter_inspector
            .properties
            .iter()
            .find(|property| property.path == "properties.direction")
            .expect("generated direction");
        assert_eq!(direction.value, serde_json::json!("in"));
        assert_eq!(
            direction.editor,
            bellatrix_mde::PropertyEditor::Choice(&["in", "inout", "out", "return"])
        );
        assert!(
            parameter_inspector
                .properties
                .iter()
                .any(|property| property.path == "properties.type"
                    && property.editor == bellatrix_mde::PropertyEditor::Reference
                    && property.editable)
        );
        parameter_ids.push(parameter.id.clone());
    }
    let [input, context, result] = parameter_ids.as_slice() else {
        panic!("three Parameters");
    };
    for (element_id, path, value) in [
        (input, "name", serde_json::json!("key")),
        (
            input,
            "properties.type",
            serde_json::json!({"$ref":"type-list"}),
        ),
        (
            input,
            "properties.type",
            serde_json::json!({"$ref":type_id}),
        ),
        (context, "name", serde_json::json!("context")),
        (context, "properties.direction", serde_json::json!("inout")),
        (
            context,
            "properties.type",
            serde_json::json!({"$ref":"type-list"}),
        ),
        (result, "name", serde_json::json!("result")),
        (result, "properties.direction", serde_json::json!("return")),
        (result, "properties.isOrdered", serde_json::json!(true)),
        (result, "properties.isUnique", serde_json::json!(false)),
        (result, "properties.multiplicity", serde_json::json!("0..*")),
        (
            result,
            "properties.type",
            serde_json::json!({"$ref":type_id}),
        ),
    ] {
        assert!(
            matches!(ctx.query::<ModelingSetProperty>(Routing::Unscoped, ModelingSetPropertyRequested {
            project_id: a.clone(), element_id: element_id.clone(), path: path.into(), value,
            expected_revision: revision,
        }).await.expect("parameter edit"), QueryOutcome::Answer(Ok(value)) if value == revision + 1)
        );
        revision += 1;
    }
    for value in ["0..capacity", "0..*"] {
        assert!(
            matches!(ctx.query::<ModelingSetProperty>(Routing::Unscoped, ModelingSetPropertyRequested {
            project_id: a.clone(), element_id: result.clone(), path: "properties.multiplicity".into(), value: serde_json::json!(value), expected_revision: revision,
        }).await.expect("authored multiplicity"), QueryOutcome::Answer(Ok(value)) if value == revision + 1)
        );
        revision += 1;
        let current = read_section();
        let inspector = inspect_element(&current, &operation.id).expect("return derivation");
        if value == "0..capacity" {
            assert!(inspector.properties.iter().any(|field| {
                field.path == "properties.lower"
                    && field
                        .value
                        .as_str()
                        .is_some_and(|text| text.contains("not evaluated"))
            }));
            assert!(
                bellatrix_mde::uml::root::Model::from_section(&current)
                    .validate()
                    .iter()
                    .any(|issue| issue.code == "MDE_PARAMETER_MULTIPLICITY_UNSUPPORTED")
            );
        }
    }
    let before_invalid = projects.snapshot(&a).expect("before invalid return");
    for (element_id, path, value) in [
        (context, "properties.direction", serde_json::json!("return")),
        (result, "properties.multiplicity", serde_json::json!("3..2")),
        (&operation.id, "properties.lower", serde_json::json!("7")),
        (
            &operation.id,
            "properties.isUnique",
            serde_json::json!(true),
        ),
        (
            context,
            "properties.direction",
            serde_json::json!("sideways"),
        ),
        (
            input,
            "properties.type",
            serde_json::json!({"$ref":"missing"}),
        ),
        (
            input,
            "properties.type",
            serde_json::json!({"$ref":operation.id}),
        ),
        (
            &operation.id,
            "properties.type",
            serde_json::json!({"$ref":type_id}),
        ),
    ] {
        assert!(matches!(
            ctx.query::<ModelingSetProperty>(
                Routing::Unscoped,
                ModelingSetPropertyRequested {
                    project_id: a.clone(),
                    element_id: element_id.clone(),
                    path: path.into(),
                    value,
                    expected_revision: revision,
                }
            )
            .await
            .expect("rejected edit"),
            QueryOutcome::Answer(Err(_))
        ));
        let after = projects.snapshot(&a).expect("atomic state");
        assert_eq!(
            serde_json::to_value(after.as_ref()).expect("after"),
            serde_json::to_value(before_invalid.as_ref()).expect("before")
        );
    }
    let section = read_section();
    let inspector = inspect_element(&section, &operation.id).expect("derived return");
    assert_eq!(
        inspector.display_label,
        "+findAgain(key: Integer, inout context: TaskList): Integer"
    );
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "properties.type"
                && property.value == serde_json::json!({"$ref":type_id})
                && !property.editable)
    );
    for (key, expected) in [
        ("isOrdered", serde_json::json!(true)),
        ("isUnique", serde_json::json!(false)),
        ("lower", serde_json::json!("0")),
        ("upper", serde_json::json!("*")),
    ] {
        assert!(
            inspector
                .properties
                .iter()
                .any(|field| field.path == format!("properties.{key}")
                    && field.value == expected
                    && !field.editable)
        );
    }
    let mut with_body = section.to_model(a.as_str(), a.as_str());
    with_body.elements.push(serde_json::from_value(serde_json::json!({"id":"return-body", "kind":"uml.constraint", "name":"Result constraint"})).expect("Constraint"));
    with_body.relations.push(serde_json::from_value(serde_json::json!({"id":"owns-return-body", "kind":"owns", "from":operation.id, "to":"return-body"})).expect("Constraint owner"));
    let imported_operation = with_body
        .elements
        .iter_mut()
        .find(|element| element.id == operation.id)
        .expect("Operation");
    imported_operation.properties.insert(
        "bodyCondition".into(),
        serde_json::json!({"$ref":"return-body"}),
    );
    imported_operation
        .properties
        .insert("isQuery".into(), serde_json::json!(true));
    imported_operation
        .properties
        .insert("isOrdered".into(), serde_json::json!(false));
    assert!(
        bellatrix_mde::uml::root::Model::from_project(&with_body)
            .validate()
            .iter()
            .any(|issue| issue.code == "MDE_OPERATION_CONDITION_UNSUPPORTED")
    );
    let imported_inspector =
        inspect_element(&ModelingSection::from_model(&with_body), &operation.id)
            .expect("imported return cache");
    assert!(imported_inspector.properties.iter().any(
        |field| field.path == "properties.isOrdered" && field.value == serde_json::json!(true)
    ));
    assert!(
        imported_inspector
            .properties
            .iter()
            .any(|field| field.path == "legacy.operation.isOrdered"
                && field.value == serde_json::json!(false))
    );
    let before_body_edit = with_body.clone();
    let rejected = bellatrix_mde::commands::apply(
        &with_body,
        &bellatrix_mde::commands::Command::SetProperty {
            element_id: operation.id.clone(),
            path: "properties.isQuery".into(),
            value: serde_json::json!(false),
        },
        revision,
    )
    .expect_err("bodyCondition requires query Operation");
    assert!(rejected.contains("only_body_for_query"));
    assert_eq!(with_body, before_body_edit);
    let mut invalid_loaded = section.to_model(a.as_str(), a.as_str());
    invalid_loaded.root_model = None;
    invalid_loaded
        .elements
        .iter_mut()
        .find(|element| element.id == *context)
        .expect("second return candidate")
        .properties
        .insert("direction".into(), serde_json::json!("return"));
    assert!(
        bellatrix_mde::validate_structural_model(&invalid_loaded)
            .iter()
            .any(|issue| issue.code == "UML_OPERATION_PARAMETERS")
    );
    let unrelated_edges = section
        .relations
        .iter()
        .filter(|relation| !(relation.kind == "owns" && relation.from == operation.id))
        .cloned()
        .collect::<Vec<_>>();
    let move_request = ModelingMoveParameterRequested {
        project_id: a.clone(),
        parameter_id: context.clone(),
        up: true,
        expected_revision: revision,
    };
    assert!(
        matches!(ctx.query::<ModelingMoveParameter>(Routing::Unscoped, move_request.clone()).await.expect("reorder"),
        QueryOutcome::Answer(Ok(value)) if value == revision + 1)
    );
    revision += 1;
    assert!(matches!(
        ctx.query::<ModelingMoveParameter>(Routing::Unscoped, move_request)
            .await
            .expect("stale reorder"),
        QueryOutcome::Answer(Err(_))
    ));
    let section = read_section();
    assert_eq!(
        section
            .relations
            .iter()
            .filter(|relation| !(relation.kind == "owns" && relation.from == operation.id))
            .cloned()
            .collect::<Vec<_>>(),
        unrelated_edges
    );
    let project = section.to_model(a.as_str(), a.as_str());
    for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
        let reopened = bellatrix_mde::decode(
            &bellatrix_mde::encode(&project, format).expect("encode"),
            format,
        )
        .expect("reopen");
        let reopened = ModelingSection::from_model(&reopened);
        let references = bellatrix_mde::uml::references::ReferenceIndex::new(
            bellatrix_mde::uml::root::Model::from_section(&reopened),
        );
        let parameters =
            bellatrix_mde::uml::operations::owned_parameters(&references, &operation.id)
                .expect("ordered parameters");
        assert_eq!(
            parameters
                .iter()
                .map(|parameter| parameter.id.as_str())
                .collect::<Vec<_>>(),
            vec![context.as_str(), input.as_str(), result.as_str()]
        );
        assert_eq!(
            bellatrix_mde::uml::operations::return_type(&references, &operation.id)
                .expect("return Type")
                .expect("typed")
                .id(),
            type_id
        );
        let inspector = inspect_element(&reopened, &operation.id).expect("reopened Inspector");
        assert_eq!(
            inspector.display_label,
            "+findAgain(inout context: TaskList, key: Integer): Integer"
        );
        let persisted = reopened
            .elements
            .iter()
            .find(|element| element.id == operation.id)
            .expect("Operation");
        assert_eq!(
            persisted.properties.get("signature"),
            raw_signature.as_ref()
        );
        assert!(!persisted.properties.contains_key("type"));
        assert!(!persisted.properties.contains_key("ownedParameter"));
        for key in ["isOrdered", "isUnique", "lower", "upper"] {
            assert!(
                !persisted.properties.contains_key(key),
                "derived return fields are never persisted"
            );
        }
        let returned =
            bellatrix_mde::uml::operations::return_properties(&references, &operation.id)
                .expect("reopened return properties");
        assert!(returned.is_ordered && !returned.is_unique);
        assert_eq!(
            returned
                .lower()
                .expect("lower")
                .expect("return exists")
                .to_string(),
            "0"
        );
        assert!(
            returned
                .upper()
                .expect("upper")
                .expect("return exists")
                .is_unlimited()
        );
        assert!(
            parameters
                .iter()
                .all(|parameter| !parameter.properties.contains_key("operation"))
        );
        let tree = bellatrix_mde::project_hierarchy(&reopened);
        assert!(
            tree.iter().position(|row| row.element_id == *context)
                < tree.iter().position(|row| row.element_id == *input)
        );
        assert_eq!(
            tree.iter()
                .find(|row| row.element_id == operation.id)
                .expect("tree Operation")
                .label,
            inspector.display_label
        );
        let classifier = inspect_element(&reopened, "type-list").expect("classifier");
        assert_eq!(
            classifier
                .operations
                .iter()
                .find(|member| member.id == operation.id)
                .expect("operation row")
                .display_label,
            inspector.display_label
        );
    }
    for (element_id, path, value) in [
        (&type_id, "name", serde_json::json!("Number")),
        (result, "properties.type", serde_json::Value::Null),
        (
            result,
            "properties.type",
            serde_json::json!({"$ref":type_id}),
        ),
    ] {
        assert!(
            matches!(ctx.query::<ModelingSetProperty>(Routing::Unscoped, ModelingSetPropertyRequested {
            project_id: a.clone(), element_id: element_id.clone(), path: path.into(), value,
            expected_revision: revision,
        }).await.expect("live type edit"), QueryOutcome::Answer(Ok(value)) if value == revision + 1)
        );
        revision += 1;
        let inspector = inspect_element(&read_section(), &operation.id).expect("live Operation");
        assert!(inspector.display_label.contains("key: Number"));
        if element_id == result
            && inspector
                .properties
                .iter()
                .any(|property| property.path == "properties.type" && property.value.is_null())
        {
            assert!(inspector.display_label.ends_with(')'));
        } else {
            assert!(inspector.display_label.ends_with(": Number"));
        }
    }
    for parameter_id in [result, input] {
        assert!(
            matches!(ctx.query::<ModelingDeleteElement>(Routing::Unscoped, ModelingDeleteElementRequested {
            project_id: a.clone(), element_id: parameter_id.clone(), expected_revision: revision,
        }).await.expect("delete Parameter"), QueryOutcome::Answer(Ok(value)) if value == revision + 1)
        );
        revision += 1;
    }
    let final_section = read_section();
    let inspector = inspect_element(&final_section, &operation.id).expect("remaining Parameters");
    for (key, expected) in [
        ("isOrdered", serde_json::json!(false)),
        ("isUnique", serde_json::json!(true)),
        ("lower", serde_json::Value::Null),
        ("upper", serde_json::Value::Null),
    ] {
        assert!(
            inspector
                .properties
                .iter()
                .any(|field| field.path == format!("properties.{key}")
                    && field.value == expected
                    && !field.editable)
        );
    }
    assert_eq!(
        inspector.display_label,
        "+findAgain(inout context: TaskList)"
    );
    assert_eq!(
        inspector
            .parameters
            .iter()
            .map(|parameter| &parameter.id)
            .collect::<Vec<_>>(),
        vec![context]
    );
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "properties.type" && property.value.is_null())
    );
    assert_eq!(
        projects.snapshot(&b).expect("isolated project").revision(),
        0
    );
    mde.dispose().await.unwrap();
    assert!(matches!(
        ctx.query::<ModelingAddMember>(
            Routing::Unscoped,
            ModelingAddMemberRequested {
                project_id: b,
                owner_id: "type-list".into(),
                kind: ModelingMemberKind::Attribute,
                expected_revision: 0
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Miss
    ));
    storage.dispose().await.unwrap();
}

#[tokio::test]
async fn class_specializations_use_the_same_versioned_cordis_event() {
    let context = Context::new();
    let projects = Arc::new(ProjectService::new());
    let storage = admit(&context, ProjectPlugin::new(Arc::clone(&projects))).await;
    let engine = admit(&context, MdePlugin).await;
    let project_id = ProjectId::new("quick-edit-specialized-class").unwrap();
    assert!(matches!(
        context
            .query::<ProjectCreate>(
                Routing::Unscoped,
                ProjectCreateRequested {
                    document: model_document(project_id.as_str()),
                }
            )
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(_))
    ));
    let section: ModelingSection = serde_json::from_value(
        projects
            .snapshot(&project_id)
            .unwrap()
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let diagram_id = section.diagrams[0].id.clone();
    let actions = [
        UmlQuickCreate::TemplateParameter,
        UmlQuickCreate::ProvidedInterface,
        UmlQuickCreate::RequiredInterface,
        UmlQuickCreate::AssociatedClass,
        UmlQuickCreate::AggregatedClass,
        UmlQuickCreate::CompositedClass,
        UmlQuickCreate::Port,
        UmlQuickCreate::Part,
    ];
    for (index, action) in actions.into_iter().enumerate() {
        let revision = index as u64;
        assert!(
            matches!(
                context.query::<ModelingQuickEdit>(Routing::Unscoped,ModelingQuickEditRequested {
                    project_id:project_id.clone(),diagram_id:diagram_id.clone(),
                    element_id:"type-task".into(),action,expected_revision:revision,
                }).await.unwrap(),
                QueryOutcome::Answer(Ok(value)) if value==revision+1
            ),
            "failed {action:?}"
        );
    }
    let document = projects.snapshot(&project_id).unwrap();
    assert_eq!(document.revision(), 8);
    let section: ModelingSection = serde_json::from_value(
        document
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    for kind in ["uml.classifier_template_parameter", "uml.port", "uml.part"] {
        let node = section.elements.iter().find(|e| e.kind == kind).unwrap();
        assert!(section.relations.iter().any(|r| r.kind == "owns"
            && (r.from == "type-task"
                || (kind == "uml.classifier_template_parameter"
                    && section.elements.iter().any(|owner| owner.id == r.from
                        && owner.kind == "uml.redefinable_template_signature")))
            && r.to == node.id));
    }
    for kind in ["uml.association", "uml.interface_realization", "uml.usage"] {
        assert!(section.relations.iter().any(|r| r.kind == kind), "{kind}");
    }
    let associations = section
        .relations
        .iter()
        .filter(|r| r.kind == "uml.association")
        .collect::<Vec<_>>();
    assert_eq!(associations.len(), 3);
    for aggregation in ["shared", "composite"] {
        assert!(
            associations.iter().any(|r| r
                .properties
                .get("end1Aggregation")
                .and_then(|v| v.as_str())
                == Some(aggregation)),
            "{aggregation}"
        );
    }
    assert_eq!(
        section
            .elements
            .iter()
            .filter(|e| e.kind == "rust.trait" && e.id.starts_with("mde-"))
            .count(),
        2
    );
    engine.dispose().await.unwrap();
    storage.dispose().await.unwrap();
}
