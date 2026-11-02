use bellatrix_mde::{Geometry, ModelingSection, Project};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingCreateElement, ModelingCreateElementRequested, ModelingElementKind,
    with_model,
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
fn document(name: &str) -> ProjectDocument {
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
    model.id = name.into();
    model.name = name.into();
    with_model(
        ProjectDocument::new(ProjectId::new(name).unwrap(), name).unwrap(),
        &model,
    )
    .unwrap()
}
fn request(id: &ProjectId, revision: u64) -> ModelingCreateElementRequested {
    ModelingCreateElementRequested {
        owner_id: None,
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
        assert!(matches!(
            ctx.query::<ProjectCreate>(
                Routing::Unscoped,
                ProjectCreateRequested {
                    document: document(name)
                }
            )
            .await
            .unwrap(),
            QueryOutcome::Answer(Ok(_))
        ));
    }
    let current = projects.snapshot(&a).unwrap();
    let section: ModelingSection = serde_json::from_value(
        current
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let mut first_request = request(&a, 0);
    first_request.diagram_id = section.diagrams.first().unwrap().id.clone();
    assert!(matches!(
        ctx.query::<ModelingCreateElement>(Routing::Unscoped, first_request.clone())
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(1))
    ));
    assert!(matches!(
        ctx.query::<ModelingCreateElement>(Routing::Unscoped, first_request)
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    assert_eq!(projects.snapshot(&b).unwrap().revision(), 0);
    let updated = projects.snapshot(&a).unwrap();
    assert_eq!(updated.revision(), 1);
    let section: ModelingSection = serde_json::from_value(
        updated
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let created = section
        .elements
        .iter()
        .find(|element| element.name == "NewTicket")
        .unwrap();
    assert_eq!(created.kind, "rust.struct");
    let view = section
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.subject.as_deref() == Some(created.id.as_str()))
        .unwrap();
    assert!(matches!(&view.geometry, Geometry::Rect { x, y, .. } if *x == 140.0 && *y == 170.0));
    mde.dispose().await.unwrap();
    assert!(matches!(
        ctx.query::<ModelingCreateElement>(Routing::Unscoped, request(&b, 0))
            .await
            .unwrap(),
        QueryOutcome::Miss
    ));
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
        ProjectCreateRequested {
            document: document("invalid-create"),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        ctx.query::<ModelingCreateElement>(
            Routing::Unscoped,
            ModelingCreateElementRequested {
                x: f64::NAN,
                ..request(&id, 0)
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    assert!(matches!(
        ctx.query::<ModelingCreateElement>(Routing::Unscoped, request(&id, 0))
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    assert_eq!(projects.snapshot(&id).unwrap().revision(), 0);
    mde.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}

#[tokio::test]
async fn component_and_artifact_creation_require_explicit_owner_and_commit_atomically()
-> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new();
    let projects = Arc::new(ProjectService::new());
    let provider = admit(&context, ProjectPlugin::new(Arc::clone(&projects))).await;
    let mde = admit(&context, MdePlugin).await;
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    for (id, kind) in [
        ("package-owner", "uml.package"),
        ("model-owner", "uml.model"),
    ] {
        model
            .elements
            .push(serde_json::from_value(serde_json::json!({
                "id": id, "kind": kind, "name": id
            }))?);
    }
    // Context must never silently select semantic ownership.
    model
        .diagrams
        .first_mut()
        .ok_or("diagram missing")?
        .model_elements = vec!["package-owner".into()];
    let id = ProjectId::new(model.id.clone())?;
    let document = with_model(ProjectDocument::new(id.clone(), &model.name)?, &model)?;
    assert!(matches!(
        context
            .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await?,
        QueryOutcome::Answer(Ok(_))
    ));
    let request = |kind, owner: Option<&str>, revision| ModelingCreateElementRequested {
        project_id: id.clone(),
        diagram_id: "diagram-tasks".into(),
        kind,
        name: Some("Owned element".into()),
        owner_id: owner.map(str::to_owned),
        x: 20.0,
        y: 30.0,
        expected_revision: revision,
    };
    for kind in [
        ModelingElementKind::Component,
        ModelingElementKind::Artifact,
    ] {
        for owner in [None, Some("missing"), Some("type-task")] {
            assert!(matches!(
                context
                    .query::<ModelingCreateElement>(Routing::Unscoped, request(kind, owner, 0))
                    .await?,
                QueryOutcome::Answer(Err(_))
            ));
            let unchanged = projects.snapshot(&id)?;
            assert_eq!(unchanged.revision(), 0);
            let section: ModelingSection = serde_json::from_value(
                unchanged
                    .section(bellatrix_mde::MODELING_NAMESPACE)
                    .ok_or("section missing")?
                    .clone(),
            )?;
            assert_eq!(section, ModelingSection::from_model(&model));
        }
    }
    for (index, (kind, model_kind, owner)) in [
        (
            ModelingElementKind::Component,
            "uml.component",
            "package-owner",
        ),
        (ModelingElementKind::Artifact, "uml.artifact", "model-owner"),
    ]
    .into_iter()
    .enumerate()
    {
        let revision = index as u64;
        assert!(
            matches!(context.query::<ModelingCreateElement>(Routing::Unscoped,
            request(kind, Some(owner), revision)).await?, QueryOutcome::Answer(Ok(next)) if next == revision + 1)
        );
        let updated = projects.snapshot(&id)?;
        let state = updated
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone();
        let section: ModelingSection = serde_json::from_value(state.clone())?;
        assert_eq!(section.elements.len(), model.elements.len() + index + 1);
        assert_eq!(section.relations.len(), model.relations.len() + index + 1);
        let created = section
            .elements
            .iter()
            .find(|element| element.kind == model_kind)
            .ok_or("created semantic element missing")?;
        assert!(
            section
                .relations
                .iter()
                .any(|relation| relation.kind == "owns"
                    && relation.from == owner
                    && relation.to == created.id)
        );
        let diagram = section.diagrams.first().ok_or("diagram missing")?;
        assert_eq!(diagram.model_elements, vec!["package-owner"]);
        assert_eq!(
            diagram.views.len(),
            model.diagrams.first().ok_or("diagram missing")?.views.len() + index + 1
        );
        assert!(
            diagram
                .views
                .iter()
                .any(|view| view.subject.as_deref() == Some(created.id.as_str()))
        );
        assert_eq!(section.diagrams.get(1), model.diagrams.get(1));
        assert!(matches!(
            context
                .query::<ModelingCreateElement>(
                    Routing::Unscoped,
                    request(kind, Some(owner), revision)
                )
                .await?,
            QueryOutcome::Answer(Err(_))
        ));
        let unchanged = projects.snapshot(&id)?;
        assert_eq!(unchanged.revision(), revision + 1);
        assert_eq!(
            unchanged.section(bellatrix_mde::MODELING_NAMESPACE),
            Some(&state)
        );
    }
    mde.dispose().await?;
    provider.dispose().await?;
    Ok(())
}
