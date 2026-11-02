//! Acceptance: the StarUML class-diagram tools create typed persisted nodes
//! and relationships through the same public Cordis revision boundary.
use bellatrix_mde::{Geometry, ModelingSection, Project, relation_kind_for_creation};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingCreateElement, ModelingCreateElementRequested, ModelingCreateRelation,
    ModelingCreateRelationRequested, ModelingDeleteElement, ModelingDeleteElementRequested,
    ModelingElementKind, ModelingMoveView, ModelingMoveViewRequested, ModelingRelationKind,
    with_model,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin,
    ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

#[tokio::test]
async fn complete_class_diagram_toolset_is_revisioned_and_editable() {
    let cx = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = ProjectPlugin::new(Arc::clone(&service));
    let provider = cx
        .spawn(PreparedPlugin::from_input(provider, Arc::clone(&service)))
        .await
        .unwrap();
    let plugin = MdePlugin;
    plugin.prepare(()).unwrap();
    let plugin = cx
        .spawn(PreparedPlugin::from_input(plugin, ()))
        .await
        .unwrap();
    let id = ProjectId::new("uml-acceptance").unwrap();
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
    model.id = id.as_str().into();
    model.name = "UML acceptance".into();
    let document = with_model(
        ProjectDocument::new(id.clone(), model.name.clone()).unwrap(),
        &model,
    )
    .unwrap();
    assert!(matches!(
        cx.query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(_))
    ));
    let diagram_id = model.diagrams.first().unwrap().id.clone();
    let nodes = ModelingElementKind::BASIC
        .iter()
        .chain(ModelingElementKind::ADVANCED)
        .chain(ModelingElementKind::PACKAGES)
        .chain(ModelingElementKind::ANNOTATIONS);
    let mut revision = 0_u64;
    for (index, kind) in nodes.copied().enumerate() {
        let request = ModelingCreateElementRequested {
            project_id: id.clone(),
            diagram_id: diagram_id.clone(),
            kind,
            name: Some(format!("Uml_{}", kind.label().replace(' ', "_"))),
            x: 40.0 + index as f64 * 8.0,
            y: 30.0 + index as f64 * 12.0,
            expected_revision: revision,
        };
        revision += 1;
        assert!(matches!(
            cx.query::<ModelingCreateElement>(Routing::Unscoped,request).await.unwrap(),
            QueryOutcome::Answer(Ok(n)) if n==revision
        ));
    }
    let snapshot = service.snapshot(&id).unwrap();
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let node = |kind: ModelingElementKind| {
        section
            .elements
            .iter()
            .find(|e| e.kind == kind.model_kind() && e.id.starts_with("mde-element-"))
            .unwrap()
            .id
            .clone()
    };
    let class = node(ModelingElementKind::Class);
    let interface = node(ModelingElementKind::Interface);
    let package = node(ModelingElementKind::Package);
    let template = node(ModelingElementKind::Template);
    let rejected = cx
        .query::<ModelingCreateRelation>(
            Routing::Unscoped,
            ModelingCreateRelationRequested {
                project_id: id.clone(),
                diagram_id: diagram_id.clone(),
                kind: ModelingRelationKind::Generalization,
                from: class.clone(),
                to: interface.clone(),
                expected_revision: revision,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(rejected, QueryOutcome::Answer(Err(ref error)) if error.contains("UML_GENERALIZATION_TYPE"))
    );
    let unchanged = service.snapshot(&id).unwrap();
    assert_eq!(unchanged.revision(), revision);
    assert_eq!(
        unchanged.section(bellatrix_mde::MODELING_NAMESPACE),
        snapshot.section(bellatrix_mde::MODELING_NAMESPACE)
    );
    let types = ModelingRelationKind::BASIC
        .iter()
        .chain(ModelingRelationKind::ADVANCED)
        .chain(ModelingRelationKind::PACKAGES);
    let mut connector_ids = Vec::new();
    for kind in types.copied() {
        let (from, to) = match kind {
            ModelingRelationKind::Generalization => (class.clone(), "type-task".into()),
            ModelingRelationKind::InterfaceRealization => (class.clone(), interface.clone()),
            ModelingRelationKind::Containment => (package.clone(), class.clone()),
            ModelingRelationKind::TemplateBinding => (template.clone(), class.clone()),
            _ => (class.clone(), interface.clone()),
        };
        revision += 1;
        assert!(matches!(
            cx.query::<ModelingCreateRelation>(Routing::Unscoped,ModelingCreateRelationRequested{
                project_id:id.clone(),diagram_id:diagram_id.clone(),kind,
                from,to,expected_revision:revision-1,
            }).await.unwrap(),QueryOutcome::Answer(Ok(n)) if n==revision
        ));
        connector_ids.push(relation_kind_for_creation(kind));
    }
    let final_snapshot = service.snapshot(&id).unwrap();
    let result: ModelingSection = serde_json::from_value(
        final_snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_eq!(final_snapshot.revision(), 24);
    for (kind, aggregation) in connector_ids {
        let relation = result
            .relations
            .iter()
            .find(|relation| {
                relation.kind == kind
                    && relation
                        .properties
                        .get("end1Aggregation")
                        .and_then(serde_json::Value::as_str)
                        == aggregation
            })
            .unwrap();
        assert!(
            result
                .diagrams
                .iter()
                .flat_map(|d| &d.views)
                .any(|v| v.subject.as_deref() == Some(relation.id.as_str())
                    && matches!(&v.geometry, Geometry::Path { .. }))
        );
    }
    // Deleting a connector must not cascade through its endpoint classes.
    let target = result
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "uml.association"
                && relation
                    .properties
                    .get("end1Aggregation")
                    .and_then(serde_json::Value::as_str)
                    == Some("shared")
        })
        .unwrap()
        .id
        .clone();
    assert!(matches!(
        cx.query::<ModelingDeleteElement>(
            Routing::Unscoped,
            ModelingDeleteElementRequested {
                project_id: id.clone(),
                element_id: target,
                expected_revision: revision,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(25))
    ));
    let final_doc = service.snapshot(&id).unwrap();
    let section: ModelingSection = serde_json::from_value(
        final_doc
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert!(section.elements.iter().any(|e| e.id == class));
    assert!(section.elements.iter().any(|e| e.id == interface));
    let class_view = section
        .diagrams
        .iter()
        .flat_map(|d| &d.views)
        .find(|v| v.subject.as_deref() == Some(class.as_str()))
        .unwrap()
        .id
        .clone();
    let edge_id = section
        .relations
        .iter()
        .find(|r| r.kind == "uml.generalization")
        .unwrap()
        .id
        .clone();
    let previous = section
        .diagrams
        .iter()
        .flat_map(|d| &d.views)
        .find(|v| v.subject.as_deref() == Some(edge_id.as_str()))
        .unwrap()
        .geometry
        .clone();
    assert!(matches!(
        cx.query::<ModelingMoveView>(
            Routing::Unscoped,
            ModelingMoveViewRequested {
                project_id: id.clone(),
                view_id: class_view,
                dx: 45.0,
                dy: 25.0,
                expected_revision: 25,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(26))
    ));
    let updated = service.snapshot(&id).unwrap();
    let after: ModelingSection = serde_json::from_value(
        updated
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let rerouted = after
        .diagrams
        .iter()
        .flat_map(|d| &d.views)
        .find(|v| v.subject.as_deref() == Some(edge_id.as_str()))
        .unwrap();
    assert_ne!(
        previous, rerouted.geometry,
        "auto-routed UML connector must follow its node"
    );
    assert_eq!(updated.revision(), 26);
    plugin.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}
