use bellatrix_mde_cordis::{
    MdePlugin, ModelingCreateElement, ModelingCreateElementRequested, ModelingElementKind,
    with_model,
};
use bellatrix_mde_model::{Geometry, ModelingSection, Project};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin, ProjectService,
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

fn document(name: &str) -> ProjectDocument {
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
    model.id = name.into();
    model.name = name.into();
    let doc = ProjectDocument::new(ProjectId::new(name).unwrap(), name).unwrap();
    with_model(doc, &model).unwrap()
}

fn request(id: &ProjectId, revision: u64) -> ModelingCreateElementRequested {
    ModelingCreateElementRequested {
        project_id: id.clone(),
        diagram_id: "diagram-classes".into(),
        kind: ModelingElementKind::Class,
        name: Some("NewTicket".into()),
        x: 140.0,
        y: 170.0,
        expected_revision: revision,
    }
}

#[tokio::test]
async fn class_creation_changes_one_project_and_rejects_stale_revision() {
    let ctx = Context::new();
    let projects = Arc::new(ProjectService::new());
    let provider = admit(&ctx, ProjectPlugin::new(Arc::clone(&projects))).await;
    let mde = admit(&ctx, MdePlugin).await;
    let a = ProjectId::new("created-a").unwrap();
    let b = ProjectId::new("created-b").unwrap();
    for name in ["created-a", "created-b"] {
        let result = ctx.query::<ProjectCreate>(
            Routing::Unscoped,
            ProjectCreateRequested { document: document(name) },
        ).await.unwrap();
        assert!(matches!(result, QueryOutcome::Answer(Ok(_))));
    }

    let current = projects.snapshot(&a).unwrap();
    let section: ModelingSection = serde_json::from_value(
        current.section("bellatrix.bellatrix-mde-modeling").unwrap().clone()
    ).unwrap();
    let diagram = section.diagrams.first().unwrap().id.clone();
    let mut first_request = request(&a, 0);
    first_request.diagram_id = diagram;
    let first = ctx.query::<ModelingCreateElement>(Routing::Unscoped, first_request.clone())
        .await.unwrap();
    assert!(matches!(first, QueryOutcome::Answer(Ok(1))));
    let stale = ctx.query::<ModelingCreateElement>(Routing::Unscoped, first_request)
        .await.unwrap();
    assert!(matches!(stale, QueryOutcome::Answer(Err(_))));
    assert_eq!(projects.snapshot(&b).unwrap().revision(), 0);

    let updated = projects.snapshot(&a).unwrap();
    assert_eq!(updated.revision(), 1);
    let section: ModelingSection = serde_json::from_value(
        updated.section("bellatrix.bellatrix-mde-modeling").unwrap().clone()
    ).unwrap();
    let created = section.elements.iter().find(|e| e.name == "NewTicket").unwrap();
    assert_eq!(created.kind, "rust.struct");
    let view = section.diagrams.iter().flat_map(|d| &d.views)
        .find(|v| v.subject.as_deref() == Some(created.id.as_str())).unwrap();
    assert!(matches!(&view.geometry, Geometry::Rect { x, y, .. } if *x == 140.0 && *y == 170.0));

    mde.dispose().await.unwrap();
    let missing = ctx.query::<ModelingCreateElement>(
        Routing::Unscoped, request(&b, 0)
    ).await.unwrap();
    assert!(matches!(missing, QueryOutcome::Miss));
    assert_eq!(projects.snapshot(&a).unwrap().revision(), 1);
    provider.dispose().await.unwrap();
}

#[tokio::test]
async fn rejects_invalid_geometry_and_missing_diagram() {
    let ctx = Context::new();
    let projects = Arc::new(ProjectService::new());
    let provider = admit(&ctx, ProjectPlugin::new(Arc::clone(&projects))).await;
    let mde = admit(&ctx, MdePlugin).await;
    let id = ProjectId::new("invalid-create").unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped,
        ProjectCreateRequested { document: document("invalid-create") },
    ).await.unwrap();

    let nan = ctx.query::<ModelingCreateElement>(
        Routing::Unscoped,
        ModelingCreateElementRequested {
            x: f64::NAN,
            ..request(&id, 0)
        },
    ).await.unwrap();
    assert!(matches!(nan, QueryOutcome::Answer(Err(_))));
    let missing = ctx.query::<ModelingCreateElement>(
        Routing::Unscoped,
        request(&id, 0)
    ).await.unwrap();
    assert!(matches!(missing, QueryOutcome::Answer(Err(_))));
    assert_eq!(projects.snapshot(&id).unwrap().revision(), 0);
    mde.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}
