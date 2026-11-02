use bellatrix_mde_cordis::{MdePlugin, ModelingMoveView, ModelingMoveViewRequested, with_model};
use bellatrix_mde_model::{Geometry, ModelingSection, Project};
use bellatrix_project::{
    ProjectClose, ProjectCloseRequested, ProjectCreate, ProjectCreateRequested, ProjectDocument,
    ProjectId, ProjectOpen, ProjectOpenRequested, ProjectPlugin, ProjectSave, ProjectSaveRequested,
    ProjectService, ProjectStateChangedEvent,
};
use cordis::event::{QueryOutcome, observer_sync};
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::{Arc, Mutex};

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

fn fixture(name: &str) -> ProjectDocument {
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
    model.id = name.into();
    model.name = name.into();
    let doc = ProjectDocument::new(ProjectId::new(name).unwrap(), name).unwrap();
    with_model(doc, &model).unwrap()
}

#[tokio::test]
async fn view_movement_is_public_versioned_and_isolated() {
    let ctx = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = admit(&ctx, ProjectPlugin::new(Arc::clone(&service))).await;
    let modeling = admit(&ctx, MdePlugin).await;
    let seen = Arc::new(Mutex::new(Vec::<(ProjectId, u64)>::new()));
    let capture = Arc::clone(&seen);
    let _listener = ctx
        .on::<ProjectStateChangedEvent, _>(observer_sync(
            move |_, event: bellatrix_project::ProjectStateChanged| {
                capture.lock().unwrap().push((event.id, event.revision));
                Ok::<_, std::convert::Infallible>(())
            },
        ))
        .unwrap();
    for name in ["bellatrix-project-a", "bellatrix-project-b"] {
        ctx.query::<ProjectCreate>(
            Routing::Unscoped,
            ProjectCreateRequested {
                document: fixture(name),
            },
        )
        .await
        .unwrap();
    }
    let a = ProjectId::new("bellatrix-project-a").unwrap();
    let b = ProjectId::new("bellatrix-project-b").unwrap();
    let move_view = |id: ProjectId, revision, dx: f64, dy: f64| ModelingMoveViewRequested {
        project_id: id,
        view_id: "view-task".into(),
        dx,
        dy,
        expected_revision: revision,
    };
    let result = ctx
        .query::<ModelingMoveView>(Routing::Unscoped, move_view(a.clone(), 0, 20.0, 10.0))
        .await
        .unwrap();
    assert!(matches!(result, QueryOutcome::Answer(Ok(1))));
    let unchanged = ctx
        .query::<ModelingMoveView>(Routing::Unscoped, move_view(a.clone(), 1, 0.0, 0.0))
        .await
        .unwrap();
    assert!(matches!(unchanged, QueryOutcome::Answer(Ok(1))));
    let stale = ctx
        .query::<ModelingMoveView>(Routing::Unscoped, move_view(a.clone(), 0, 50.0, 50.0))
        .await
        .unwrap();
    assert!(matches!(stale, QueryOutcome::Answer(Err(_))));
    let invalid = ctx
        .query::<ModelingMoveView>(Routing::Unscoped, move_view(a.clone(), 1, f64::NAN, 0.0))
        .await
        .unwrap();
    assert!(matches!(invalid, QueryOutcome::Answer(Err(_))));
    let document = service.snapshot(&a).unwrap();
    let section: ModelingSection = serde_json::from_value(
        document
            .section("bellatrix.bellatrix-mde-modeling")
            .unwrap()
            .clone(),
    )
    .unwrap();
    let view = section
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.id == "view-task")
        .unwrap();
    assert!(matches!(view.geometry, Geometry::Rect { x, y, .. }
        if x == 545.0 && y == 110.0));
    assert_eq!(service.snapshot(&b).unwrap().revision(), 0);
    assert_eq!(seen.lock().unwrap().as_slice(), &[(a, 1)]);
    modeling.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}

#[tokio::test]
async fn translated_view_roundtrips_through_public_project_save_and_open() {
    let ctx = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = admit(&ctx, ProjectPlugin::new(Arc::clone(&service))).await;
    let modeling = admit(&ctx, MdePlugin).await;
    let id = ProjectId::new("persisted-view").unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped,
        ProjectCreateRequested {
            document: fixture("persisted-view"),
        },
    )
    .await
    .unwrap();
    let moved = ctx
        .query::<ModelingMoveView>(
            Routing::Unscoped,
            ModelingMoveViewRequested {
                project_id: id.clone(),
                view_id: "view-task".into(),
                dx: -30.0,
                dy: 45.0,
                expected_revision: 0,
            },
        )
        .await
        .unwrap();
    assert!(matches!(moved, QueryOutcome::Answer(Ok(1))));
    let directory = std::env::temp_dir().join(format!(
        "bellatrix-move-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("bellatrix-project.bellatrix.json");
    let saved = ctx
        .query::<ProjectSave>(
            Routing::Unscoped,
            ProjectSaveRequested {
                id: id.clone(),
                path: path.clone(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(saved, QueryOutcome::Answer(Ok(1))));
    ctx.query::<ProjectClose>(
        Routing::Unscoped,
        ProjectCloseRequested {
            id: id.clone(),
            discard_unsaved: false,
        },
    )
    .await
    .unwrap();
    let reopened = ctx
        .query::<ProjectOpen>(
            Routing::Unscoped,
            ProjectOpenRequested { path: path.clone() },
        )
        .await
        .unwrap();
    assert!(matches!(reopened, QueryOutcome::Answer(Ok(_))));
    let doc = service.snapshot(&id).unwrap();
    assert_eq!(doc.revision(), 1);
    let section: ModelingSection = serde_json::from_value(
        doc.section("bellatrix.bellatrix-mde-modeling")
            .unwrap()
            .clone(),
    )
    .unwrap();
    let view = section
        .diagrams
        .iter()
        .flat_map(|d| &d.views)
        .find(|v| v.id == "view-task")
        .unwrap();
    assert!(matches!(view.geometry, Geometry::Rect { x, y, .. }
        if x == 495.0 && y == 145.0));
    modeling.dispose().await.unwrap();
    provider.dispose().await.unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
