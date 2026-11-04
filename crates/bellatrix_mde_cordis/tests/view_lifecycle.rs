#[path = "support/events.rs"]
mod events;
use serde_json::Value;
use bellatrix_mde_cordis::Geometry;
use bellatrix_mde_cordis::{
    MODELING_NAMESPACE, MdePlugin, ModelingDeleteDiagram, ModelingDeleteDiagramRequested,
    ModelingRemoveView, ModelingRemoveViewRequested,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectId, ProjectPlugin,
    ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, PreparedPlugin, Routing};
use std::sync::Arc;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn section(projects: &ProjectService, id: &ProjectId) -> TestResult<Value> {
    events::section(projects, id)
}

fn array<'a>(value: &'a Value, field: &str) -> TestResult<&'a Vec<Value>> {
    value.get(field).and_then(Value::as_array).ok_or_else(|| format!("{field} missing").into())
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
    let mut model: Value =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    let mut other = array(&model, "diagrams")?.first().ok_or("diagram missing")?.clone();
    other["id"] = "diagram-copy".into();
    for view in other["views"].as_array_mut().ok_or("views missing")? {
        view["id"] = format!("copy-{}", view["id"].as_str().ok_or("view id missing")?).into();
    }
    model["diagrams"].as_array_mut().ok_or("diagrams missing")?.push(other.clone());
    let id = ProjectId::new(model["id"].as_str().ok_or("id missing")?)?;
    let document = events::prepare(&context, &model).await?;
    let model = document.section(MODELING_NAMESPACE).ok_or("section missing")?.clone();
    let other = array(&model, "diagrams")?.iter().find(|diagram| diagram["id"] == "diagram-copy").ok_or("copied diagram missing")?.clone();
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
    assert_eq!(updated["elements"], model["elements"]);
    assert_eq!(updated["relations"], model["relations"]);
    let diagram = array(&updated, "diagrams")?
        .iter()
        .find(|diagram| diagram["id"] == "diagram-tasks")
        .ok_or("diagram missing")?;
    assert_eq!(array(diagram, "views")?.len(), remaining_views);
    assert!(!array(diagram, "views")?.iter().any(|view| view["id"] == view_id));
    assert!(!array(diagram, "views")?.iter().any(|view| view["id"] == "view-dep"));
    assert_eq!(
        array(&updated, "diagrams")?
            .iter()
            .find(|diagram| diagram["id"] == other["id"]),
        Some(&other)
    );
    assert_eq!(array(&updated, "diagrams")?.get(1), array(&model, "diagrams")?.get(1));

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
    assert_eq!(deleted["elements"], model["elements"]);
    assert_eq!(deleted["relations"], model["relations"]);
    assert_eq!(
        array(&deleted, "diagrams")?.clone(),
        array(&updated, "diagrams")?
            .iter()
            .filter(|diagram| diagram["id"] != "diagram-tasks")
            .cloned().collect::<Vec<_>>()
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
    let model: Value =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    let id = ProjectId::new(model["id"].as_str().ok_or("id missing")?)?;
    let document = events::prepare(&context, &model).await?;
    let model = document.section(MODELING_NAMESPACE).ok_or("section missing")?.clone();
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
    assert_eq!(first["elements"], model["elements"]);
    assert_eq!(first["relations"], model["relations"]);
    assert_eq!(array(&first, "diagrams")?.first(), array(&model, "diagrams")?.first());
    let overview = array(&first, "diagrams")?
        .iter()
        .find(|diagram| diagram["id"] == "diagram-overview")
        .ok_or("overview missing")?;
    assert_eq!(array(overview, "views")?.len(), 2);
    let placed = array(overview, "views")?
        .iter()
        .find(|view| view.get("subject").and_then(Value::as_str) == Some("type-list"))
        .ok_or("placed view missing")?;
    assert!(
        !array(&model, "diagrams")?
            .iter()
            .flat_map(|diagram| diagram["views"].as_array().unwrap())
            .any(|view| view["id"] == placed["id"])
    );
    assert!(matches!(
        serde_json::from_value::<Geometry>(placed["geometry"].clone())?,
        Geometry::Rect {
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
    assert_eq!(second["elements"], model["elements"]);
    assert_eq!(second["relations"], model["relations"]);
    assert_eq!(array(&second, "diagrams")?.get(1), array(&first, "diagrams")?.get(1));
    let tasks = array(&second, "diagrams")?.first().ok_or("tasks missing")?;
    assert_eq!(array(tasks, "views")?.len(), 4);
    assert_eq!(
        array(tasks, "views")?
            .iter()
            .filter(|view| view.get("subject").and_then(Value::as_str) == Some("type-list"))
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
