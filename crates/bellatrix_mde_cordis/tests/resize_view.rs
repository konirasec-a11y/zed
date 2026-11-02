use bellatrix_mde_cordis::{
    MdePlugin, ModelingResizeView, ModelingResizeViewRequested, with_model,
};
use bellatrix_mde::{Geometry, ModelingSection, Project};
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
    let prepared = PreparedPlugin::from_input(plugin, input);
    ctx.spawn(prepared).await.unwrap()
}

#[tokio::test]
async fn rectangle_resize_is_versioned_and_persisted() {
    let ctx = Context::new();
    let service = Arc::new(ProjectService::new());
    let project = admit(&ctx, ProjectPlugin::new(Arc::clone(&service))).await;
    let mde = admit(&ctx, MdePlugin).await;
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
    model.id = "resized-project".into();
    model.name = "Resized project".into();
    let id = ProjectId::new(model.id.clone()).unwrap();
    let doc = ProjectDocument::new(id.clone(), model.name.clone()).unwrap();
    let doc = with_model(doc, &model).unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped, ProjectCreateRequested { document: doc }
    ).await.unwrap();

    let resize = |width, height, revision| ModelingResizeViewRequested {
        project_id: id.clone(),
        view_id: "view-task".into(),
        width,
        height,
        expected_revision: revision,
    };
    let reply = ctx.query::<ModelingResizeView>(
        Routing::Unscoped, resize(240.0, 140.0, 0)
    ).await.unwrap();
    assert!(matches!(reply, QueryOutcome::Answer(Ok(1))));

    let stale = ctx.query::<ModelingResizeView>(
        Routing::Unscoped, resize(190.0, 130.0, 0)
    ).await.unwrap();
    assert!(matches!(stale, QueryOutcome::Answer(Err(_))));
    let invalid = ctx.query::<ModelingResizeView>(
        Routing::Unscoped, resize(f64::NAN, 20.0, 1)
    ).await.unwrap();
    assert!(matches!(invalid, QueryOutcome::Answer(Err(_))));

    let persisted = service.snapshot(&id).unwrap();
    assert_eq!(persisted.revision(), 1);
    let section: ModelingSection = serde_json::from_value(
        persisted.section("bellatrix.bellatrix-mde-modeling").unwrap().clone()
    ).unwrap();
    let view = section.diagrams.iter().flat_map(|d| &d.views)
        .find(|v| v.id == "view-task").unwrap();
    assert!(matches!(&view.geometry, Geometry::Rect { width, height, .. }
        if *width == 240.0 && *height == 140.0));
    mde.dispose().await.unwrap();
    project.dispose().await.unwrap();
}
