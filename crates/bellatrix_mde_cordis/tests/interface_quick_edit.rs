use bellatrix_mde::{Geometry, Graph, ModelingSection, Project};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingAddMember, ModelingAddMemberRequested, ModelingCreateElement,
    ModelingCreateElementRequested, ModelingElementKind, ModelingMemberKind, ModelingQuickEdit,
    ModelingQuickEditRequested, ModelingSetNameExpression, ModelingSetNameExpressionRequested,
    UmlQuickCreate, with_model,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin,
    ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

#[tokio::test]
async fn interface_quick_edit_commands_create_real_objects_and_names() {
    let ctx = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = ctx
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&service)),
            Arc::clone(&service),
        ))
        .await
        .unwrap();
    let engine = MdePlugin;
    engine.prepare(()).unwrap();
    let engine = ctx
        .spawn(PreparedPlugin::from_input(engine, ()))
        .await
        .unwrap();
    let id = ProjectId::new("interface-quick-edits").unwrap();
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
    model.id = id.as_str().into();
    model.name = "Interface Quick Edit".into();
    let diagram_id = model.diagrams.first().unwrap().id.clone();
    let document = with_model(
        ProjectDocument::new(id.clone(), model.name.clone()).unwrap(),
        &model,
    )
    .unwrap();
    assert!(matches!(
        ctx.query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(_))
    ));
    assert!(matches!(
        ctx.query::<ModelingCreateElement>(
            Routing::Unscoped,
            ModelingCreateElementRequested {
                owner_id: None,
                project_id: id.clone(),
                diagram_id: diagram_id.clone(),
                kind: ModelingElementKind::Interface,
                name: Some("Interface1".into()),
                x: 160.0,
                y: 160.0,
                expected_revision: 0,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(1))
    ));
    let snapshot = service.snapshot(&id).unwrap();
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let interface_id = section
        .elements
        .iter()
        .find(|element| element.name == "Interface1")
        .unwrap()
        .id
        .clone();
    for (index, action) in [
        UmlQuickCreate::Note,
        UmlQuickCreate::Constraint,
        UmlQuickCreate::Reception,
        UmlQuickCreate::Subtype,
        UmlQuickCreate::Supertype,
        UmlQuickCreate::RealizingClass,
    ]
    .into_iter()
    .enumerate()
    {
        let revision = (index + 1) as u64;
        assert!(
            matches!(ctx.query::<ModelingQuickEdit>(Routing::Unscoped, ModelingQuickEditRequested {
            project_id: id.clone(), diagram_id: diagram_id.clone(), element_id: interface_id.clone(), action, expected_revision: revision,
        }).await.unwrap(), QueryOutcome::Answer(Ok(updated)) if updated == revision + 1)
        );
    }
    let snapshot = service.snapshot(&id).unwrap();
    assert_eq!(snapshot.revision(), 7);
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    Graph::new(&section.to_model(id.as_str(), "Quick Edit"))
        .validate()
        .unwrap();
    for kind in [
        "uml.note",
        "uml.constraint",
        "uml.reception",
        "rust.trait",
        "rust.struct",
    ] {
        assert!(
            section.elements.iter().any(|element| element.kind == kind),
            "{kind}"
        );
    }
    assert_eq!(
        section
            .relations
            .iter()
            .filter(|relation| relation.kind == "uml.generalization")
            .count(),
        2
    );
    assert!(
        section
            .relations
            .iter()
            .any(|relation| relation.kind == "uml.interface_realization")
    );
    for relation in section.relations.iter().filter(|relation| {
        relation.kind.starts_with("uml.")
            && [
                "uml.generalization",
                "uml.interface_realization",
                "uml.note_link",
                "uml.constraint_link",
            ]
            .contains(&relation.kind.as_str())
    }) {
        assert!(
            section
                .diagrams
                .iter()
                .flat_map(|diagram| &diagram.views)
                .any(|view| view.subject.as_deref() == Some(relation.id.as_str())
                    && matches!(&view.geometry, Geometry::Path { .. }))
        );
    }
    assert!(matches!(
        ctx.query::<ModelingSetNameExpression>(
            Routing::Unscoped,
            ModelingSetNameExpressionRequested {
                project_id: id.clone(),
                element_id: interface_id.clone(),
                expression: "<< service >> # CloudIface".into(),
                expected_revision: 7,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(8))
    ));
    assert!(matches!(
        ctx.query::<ModelingSetNameExpression>(
            Routing::Unscoped,
            ModelingSetNameExpressionRequested {
                project_id: id.clone(),
                element_id: interface_id.clone(),
                expression: "Bad".into(),
                expected_revision: 7,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    for (kind, revision) in [
        (ModelingMemberKind::Attribute, 8),
        (ModelingMemberKind::Operation, 9),
    ] {
        assert!(
            matches!(ctx.query::<ModelingAddMember>(Routing::Unscoped, ModelingAddMemberRequested {
            project_id: id.clone(), owner_id: interface_id.clone(), kind, expected_revision: revision,
        }).await.unwrap(), QueryOutcome::Answer(Ok(updated)) if updated == revision + 1)
        );
    }
    let snapshot = service.snapshot(&id).unwrap();
    assert_eq!(snapshot.revision(), 10);
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let interface = section
        .elements
        .iter()
        .find(|element| element.id == interface_id)
        .unwrap();
    assert_eq!(interface.name, "CloudIface");
    assert_eq!(interface.properties.get("stereotype").unwrap(), "service");
    assert_eq!(interface.properties.get("visibility").unwrap(), "protected");
    let view = section
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.subject.as_deref() == Some(interface_id.as_str()))
        .unwrap();
    assert!(
        matches!(&view.geometry, Geometry::Rect { height, .. } if *height >= 76.0 + 3.0 * 23.0)
    );
    Graph::new(&section.to_model(id.as_str(), "Quick Edit"))
        .validate()
        .unwrap();
    engine.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}
