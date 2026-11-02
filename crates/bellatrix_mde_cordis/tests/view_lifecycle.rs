use bellatrix_mde::{ModelingSection, Project};
use bellatrix_mde_cordis::{
    MODELING_NAMESPACE, MdePlugin, ModelingDeleteDiagram, ModelingDeleteDiagramRequested,
    ModelingRemoveView, ModelingRemoveViewRequested, with_model,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin,
    ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, PreparedPlugin, Routing};
use std::sync::Arc;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn section(projects: &ProjectService, id: &ProjectId) -> TestResult<ModelingSection> {
    let document = projects.snapshot(id)?;
    Ok(serde_json::from_value(
        document
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?)
}

async fn exercise_removal(view_id: &str, remaining_views: usize) -> TestResult {
    let context = Context::new();
    let projects = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&projects)),
            Arc::clone(&projects),
        ))
        .await?;
    let mde = context
        .spawn(PreparedPlugin::from_input(MdePlugin, ()))
        .await?;
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    let mut other = model.diagrams.first().ok_or("diagram missing")?.clone();
    other.id = "diagram-copy".into();
    for view in &mut other.views {
        view.id = format!("copy-{}", view.id);
    }
    model.diagrams.push(other.clone());
    let id = ProjectId::new(model.id.clone())?;
    let document = with_model(ProjectDocument::new(id.clone(), &model.name)?, &model)?;
    assert!(matches!(
        context
            .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await?,
        QueryOutcome::Answer(Ok(_))
    ));
    let request = |revision| ModelingRemoveViewRequested {
        project_id: id.clone(),
        diagram_id: "diagram-tasks".into(),
        view_id: view_id.into(),
        expected_revision: revision,
    };
    assert!(matches!(
        context
            .query::<ModelingRemoveView>(Routing::Unscoped, request(0))
            .await?,
        QueryOutcome::Answer(Ok(1))
    ));
    let updated = section(&projects, &id)?;
    assert_eq!(updated.elements, model.elements);
    assert_eq!(updated.relations, model.relations);
    let diagram = updated
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "diagram-tasks")
        .ok_or("diagram missing")?;
    assert_eq!(diagram.views.len(), remaining_views);
    assert!(!diagram.views.iter().any(|view| view.id == view_id));
    assert!(!diagram.views.iter().any(|view| view.id == "view-dep"));
    assert_eq!(
        updated
            .diagrams
            .iter()
            .find(|diagram| diagram.id == other.id),
        Some(&other)
    );
    assert_eq!(updated.diagrams.get(1), model.diagrams.get(1));

    // Both a stale retry and a fresh retry against the removed View are atomic failures.
    for revision in [0, 1] {
        assert!(matches!(
            context
                .query::<ModelingRemoveView>(Routing::Unscoped, request(revision))
                .await?,
            QueryOutcome::Answer(Err(_))
        ));
        assert_eq!(projects.snapshot(&id)?.revision(), 1);
        assert_eq!(section(&projects, &id)?, updated);
    }
    assert!(matches!(
        context
            .query::<ModelingDeleteDiagram>(
                Routing::Unscoped,
                ModelingDeleteDiagramRequested {
                    project_id: id.clone(),
                    diagram_id: "diagram-tasks".into(),
                    expected_revision: 1,
                }
            )
            .await?,
        QueryOutcome::Answer(Ok(2))
    ));
    let deleted = section(&projects, &id)?;
    assert_eq!(deleted.elements, model.elements);
    assert_eq!(deleted.relations, model.relations);
    assert_eq!(
        deleted.diagrams,
        updated
            .diagrams
            .into_iter()
            .filter(|diagram| diagram.id != "diagram-tasks")
            .collect::<Vec<_>>()
    );
    mde.dispose().await?;
    assert!(matches!(
        context
            .query::<ModelingRemoveView>(Routing::Unscoped, request(2))
            .await?,
        QueryOutcome::Miss
    ));
    provider.dispose().await?;
    Ok(())
}

#[tokio::test]
async fn removing_edge_view_preserves_semantics_and_other_diagrams() -> TestResult {
    exercise_removal("view-dep", 2).await
}

#[tokio::test]
async fn removing_endpoint_view_cleans_up_only_local_edge_views() -> TestResult {
    exercise_removal("view-task", 1).await
}

#[tokio::test]
async fn placing_existing_element_across_diagrams_uses_one_project_revision() -> TestResult {
    use bellatrix_mde_cordis::{ModelingPlaceElementView, ModelingPlaceElementViewRequested};
    let context = Context::new();
    let projects = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&projects)),
            Arc::clone(&projects),
        ))
        .await?;
    let mde = context
        .spawn(PreparedPlugin::from_input(MdePlugin, ()))
        .await?;
    let model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    let id = ProjectId::new(model.id.clone())?;
    let document = with_model(ProjectDocument::new(id.clone(), &model.name)?, &model)?;
    assert!(matches!(
        context
            .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await?,
        QueryOutcome::Answer(Ok(_))
    ));
    let request = |diagram_id: &str, revision| ModelingPlaceElementViewRequested {
        project_id: id.clone(),
        diagram_id: diagram_id.into(),
        element_id: "type-list".into(),
        x: 25.0,
        y: 45.0,
        expected_revision: revision,
    };
    assert!(matches!(
        context
            .query::<ModelingPlaceElementView>(Routing::Unscoped, request("diagram-overview", 0))
            .await?,
        QueryOutcome::Answer(Ok(1))
    ));
    let first = section(&projects, &id)?;
    assert_eq!(first.elements, model.elements);
    assert_eq!(first.relations, model.relations);
    assert_eq!(first.diagrams.first(), model.diagrams.first());
    let overview = first
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "diagram-overview")
        .ok_or("overview missing")?;
    assert_eq!(overview.views.len(), 2);
    let placed = overview
        .views
        .iter()
        .find(|view| view.subject.as_deref() == Some("type-list"))
        .ok_or("placed view missing")?;
    assert!(
        !model
            .diagrams
            .iter()
            .flat_map(|diagram| &diagram.views)
            .any(|view| view.id == placed.id)
    );
    assert!(matches!(
        placed.geometry,
        bellatrix_mde::Geometry::Rect {
            x: 25.0,
            y: 45.0,
            ..
        }
    ));

    // An edit in one Diagram invalidates a request based on the old global revision in another.
    assert!(matches!(
        context
            .query::<ModelingPlaceElementView>(Routing::Unscoped, request("diagram-tasks", 0))
            .await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert_eq!(projects.snapshot(&id)?.revision(), 1);
    assert_eq!(section(&projects, &id)?, first);
    assert!(matches!(
        context
            .query::<ModelingPlaceElementView>(Routing::Unscoped, request("diagram-tasks", 1))
            .await?,
        QueryOutcome::Answer(Ok(2))
    ));
    let second = section(&projects, &id)?;
    assert_eq!(second.elements, model.elements);
    assert_eq!(second.relations, model.relations);
    assert_eq!(second.diagrams.get(1), first.diagrams.get(1));
    let tasks = second.diagrams.first().ok_or("tasks missing")?;
    assert_eq!(tasks.views.len(), 4);
    assert_eq!(
        tasks
            .views
            .iter()
            .filter(|view| view.subject.as_deref() == Some("type-list"))
            .count(),
        2
    );
    assert_eq!(projects.snapshot(&id)?.revision(), 2);
    mde.dispose().await?;
    assert!(matches!(
        context
            .query::<ModelingPlaceElementView>(Routing::Unscoped, request("diagram-tasks", 2))
            .await?,
        QueryOutcome::Miss
    ));
    provider.dispose().await?;
    Ok(())
}
