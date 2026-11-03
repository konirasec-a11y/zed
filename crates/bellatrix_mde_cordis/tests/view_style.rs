use bellatrix_mde::{Geometry, ModelingSection, PathSegment, Project, RelationshipRouting, Style};
use bellatrix_mde_cordis::*;
use bellatrix_project::*;
use cordis::{Context, Plugin, PreparedPlugin, Routing, event::QueryOutcome};
use std::sync::Arc;

#[tokio::test]
async fn styles_and_routes_use_revision_boundary_and_rollback()
-> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&service)),
            Arc::clone(&service),
        ))
        .await?;
    let plugin = MdePlugin;
    plugin.prepare(())?;
    let plugin = context
        .spawn(PreparedPlugin::from_input(plugin, ()))
        .await?;
    let id = ProjectId::new("style-test")?;
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    model.id = id.as_str().into();
    model.name = "Styles".into();
    let document = with_model(ProjectDocument::new(id.clone(), "Styles")?, &model)?;
    assert!(matches!(
        context
            .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await?,
        QueryOutcome::Answer(Ok(_))
    ));
    let diagram = model.diagrams.first().ok_or("diagram missing")?;
    let view = diagram.views.first().ok_or("view missing")?;
    let mut style = view.style.clone();
    style.fill = Some("#123456".into());
    let request = ModelingSetViewStyleRequested {
        project_id: id.clone(),
        diagram_id: diagram.id.clone(),
        view_id: view.id.clone(),
        style: style.clone(),
        expected_revision: 0,
    };
    assert!(matches!(
        context
            .query::<ModelingSetViewStyle>(Routing::Unscoped, request.clone())
            .await?,
        QueryOutcome::Answer(Ok(1))
    ));
    let before = service.snapshot(&id)?;
    assert!(matches!(
        context
            .query::<ModelingSetViewStyle>(Routing::Unscoped, request.clone())
            .await?,
        QueryOutcome::Answer(Err(_))
    ));
    let mut invalid = request;
    invalid.expected_revision = 1;
    invalid.style = Style {
        opacity: Some(f64::NAN),
        ..style
    };
    assert!(matches!(
        context
            .query::<ModelingSetViewStyle>(Routing::Unscoped, invalid)
            .await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert_eq!(service.snapshot(&id)?.revision(), 1);
    assert_eq!(
        service.snapshot(&id)?.section(MODELING_NAMESPACE),
        before.section(MODELING_NAMESPACE)
    );
    for (revision, x) in [(1, 10.0), (2, 300.0)] {
        assert!(matches!(
            context
                .query::<ModelingCreateElement>(
                    Routing::Unscoped,
                    ModelingCreateElementRequested {
                        project_id: id.clone(),
                        diagram_id: diagram.id.clone(),
                        owner_id: None,
                        kind: ModelingElementKind::Class,
                        name: Some(format!("Class{revision}")),
                        x,
                        y: 200.0,
                        expected_revision: revision
                    }
                )
                .await?,
            QueryOutcome::Answer(Ok(_))
        ));
    }
    let snapshot = service.snapshot(&id)?;
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?;
    let classes = section
        .elements
        .iter()
        .filter(|element| element.kind == ModelingElementKind::Class.model_kind())
        .collect::<Vec<_>>();
    let source = classes.first().ok_or("source missing")?;
    let target = classes.last().ok_or("target missing")?;
    assert!(matches!(
        context
            .query::<ModelingCreateRelation>(
                Routing::Unscoped,
                ModelingCreateRelationRequested {
                    project_id: id.clone(),
                    diagram_id: diagram.id.clone(),
                    kind: ModelingRelationKind::Association,
                    from: source.id.clone(),
                    to: target.id.clone(),
                    expected_revision: 3
                }
            )
            .await?,
        QueryOutcome::Answer(Ok(4))
    ));
    let snapshot = service.snapshot(&id)?;
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?;
    let edge = section
        .diagrams
        .iter()
        .find(|item| item.id == diagram.id)
        .and_then(|diagram| diagram.views.last())
        .ok_or("edge missing")?;
    assert!(matches!(
        context
            .query::<ModelingSetViewRouting>(
                Routing::Unscoped,
                ModelingSetViewRoutingRequested {
                    project_id: id.clone(),
                    diagram_id: diagram.id.clone(),
                    view_id: edge.id.clone(),
                    routing: RelationshipRouting::Curve,
                    expected_revision: 4
                }
            )
            .await?,
        QueryOutcome::Answer(Ok(5))
    ));
    let snapshot = service.snapshot(&id)?;
    let updated: ModelingSection = serde_json::from_value(
        snapshot
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?;
    assert_eq!(updated.relations, section.relations);
    let edge = updated
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.id == edge.id)
        .ok_or("edge missing")?;
    assert!(
        matches!(&edge.geometry,Geometry::Path {segments,..} if matches!(segments.first(),Some(PathSegment::Cubic {..})))
    );
    let bridge = MdeUiBridge::start(&context, id.clone())?;
    let request = ModelingSetViewStyleRequested {
        project_id: id.clone(),
        diagram_id: diagram.id.clone(),
        view_id: view.id.clone(),
        style: Style::default(),
        expected_revision: 5,
    };
    let (completion, mut receipt) = tokio::sync::watch::channel(None);
    bridge
        .sender()
        .try_send(MdeUiCommand::SetViewStyleTracked {
            request: request.clone(),
            completion,
        })?;
    tokio::time::timeout(std::time::Duration::from_secs(5), receipt.changed()).await??;
    assert_eq!(*receipt.borrow(), Some(Ok(6)));
    assert_eq!(service.snapshot(&id)?.revision(), 6);

    let (completion, mut stale_receipt) = tokio::sync::watch::channel(None);
    bridge
        .sender()
        .try_send(MdeUiCommand::SetViewStyleTracked {
            request: request.clone(),
            completion,
        })?;
    tokio::time::timeout(std::time::Duration::from_secs(5), stale_receipt.changed()).await??;
    assert!(matches!(&*stale_receipt.borrow(), Some(Err(_))));
    assert_eq!(service.snapshot(&id)?.revision(), 6);
    assert_eq!(
        *receipt.borrow(),
        Some(Ok(6)),
        "another command cannot overwrite the receipt"
    );

    let (completion, abandoned_receipt) = tokio::sync::watch::channel(None);
    drop(abandoned_receipt);
    bridge
        .sender()
        .try_send(MdeUiCommand::SetViewStyleTracked {
            request: ModelingSetViewStyleRequested {
                expected_revision: 6,
                style: Style {
                    fill: Some("#abcdef".into()),
                    ..Style::default()
                },
                ..request.clone()
            },
            completion,
        })?;
    let (completion, mut subsequent_receipt) = tokio::sync::watch::channel(None);
    bridge
        .sender()
        .try_send(MdeUiCommand::SetViewStyleTracked {
            request: ModelingSetViewStyleRequested {
                expected_revision: 7,
                ..request.clone()
            },
            completion,
        })?;
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        subsequent_receipt.changed(),
    )
    .await??;
    assert_eq!(
        *subsequent_receipt.borrow(),
        Some(Ok(8)),
        "closing the inspector must not invalidate the preceding command"
    );
    assert_eq!(service.snapshot(&id)?.revision(), 8);
    let (completion, mut no_op_receipt) = tokio::sync::watch::channel(None);
    bridge
        .sender()
        .try_send(MdeUiCommand::SetViewStyleTracked {
            request: ModelingSetViewStyleRequested {
                expected_revision: 8,
                ..request
            },
            completion,
        })?;
    tokio::time::timeout(std::time::Duration::from_secs(5), no_op_receipt.changed()).await??;
    assert_eq!(
        *no_op_receipt.borrow(),
        Some(Ok(8)),
        "an already inherited style is an acknowledged no-op"
    );
    let (completion, mut routing_receipt) = tokio::sync::watch::channel(None);
    bridge
        .sender()
        .try_send(MdeUiCommand::SetViewRoutingTracked {
            request: ModelingSetViewRoutingRequested {
                project_id: id.clone(),
                diagram_id: diagram.id.clone(),
                view_id: edge.id.clone(),
                routing: RelationshipRouting::Straight,
                expected_revision: 8,
            },
            completion,
        })?;
    tokio::time::timeout(std::time::Duration::from_secs(5), routing_receipt.changed()).await??;
    assert_eq!(*routing_receipt.borrow(), Some(Ok(9)));
    let snapshot = service.snapshot(&id)?;
    let routed: ModelingSection = serde_json::from_value(
        snapshot
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?;
    let routed_edge = routed
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.id == edge.id)
        .ok_or("edge missing")?;
    assert_eq!(
        routed_edge.style.routing,
        Some(RelationshipRouting::Straight)
    );
    assert!(
        matches!(&routed_edge.geometry, Geometry::Path { segments, .. }
        if segments.iter().all(|segment| matches!(segment, PathSegment::Line { .. })))
    );
    assert_eq!(routed.relations, updated.relations);
    drop(bridge);
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}

#[tokio::test]
async fn reorder_roundtrips_receipt_and_rejects_stale_revision()
-> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&service)),
            Arc::clone(&service),
        ))
        .await?;
    let plugin = MdePlugin;
    plugin.prepare(())?;
    let plugin = context
        .spawn(PreparedPlugin::from_input(plugin, ()))
        .await?;
    let id = ProjectId::new("order-test")?;
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    model.id = id.as_str().into();
    model.name = "Order".into();
    let document = with_model(ProjectDocument::new(id.clone(), "Order")?, &model)?;
    assert!(matches!(
        context
            .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await?,
        QueryOutcome::Answer(Ok(_))
    ));
    let request = ModelingReorderViewRequested {
        project_id: id.clone(),
        diagram_id: "diagram-tasks".into(),
        view_id: "view-task".into(),
        order: bellatrix_mde::commands::ViewOrder::SendToBack,
        expected_revision: 0,
    };
    assert!(matches!(
        context
            .query::<ModelingReorderView>(Routing::Unscoped, request.clone())
            .await?,
        QueryOutcome::Answer(Ok(1))
    ));
    assert!(matches!(
        context
            .query::<ModelingReorderView>(Routing::Unscoped, request)
            .await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert_eq!(service.snapshot(&id)?.revision(), 1);
    drop(plugin);
    drop(provider);
    Ok(())
}
