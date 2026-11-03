use bellatrix_mde::{
    ModelingSection, Project, element_display_label, inspect_element, member_presentation,
};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingAddMember, ModelingAddMemberRequested, ModelingMemberKind,
    ModelingQuickEdit, ModelingQuickEditRequested, ModelingSetProperty,
    ModelingSetPropertyRequested, UmlQuickCreate, with_model,
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
            serde_json::json!("Integer"),
        ),
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
        assert_eq!(element_display_label(member), expected);
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
        (&attribute.id, "properties.type", serde_json::json!("")),
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
