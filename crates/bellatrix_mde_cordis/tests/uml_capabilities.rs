#[path = "support/events.rs"]
mod events;
use bellatrix_mde_cordis::{
    MdePlugin, ModelingQuickEdit, ModelingQuickEditRequested, UmlQuickCreate,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectId, ProjectPlugin, ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use serde_json::Value;
use std::sync::Arc;

#[tokio::test]
async fn common_quick_edit_is_revisioned_project_scoped_and_disposed() {
    let context = Context::new();
    let projects = Arc::new(ProjectService::new());
    let provider = ProjectPlugin::new(Arc::clone(&projects));
    let provider_input = provider.prepare(()).unwrap();
    let provider = context
        .spawn(PreparedPlugin::from_input(provider, provider_input))
        .await
        .unwrap();
    let engine = MdePlugin;
    engine.prepare(()).unwrap();
    let engine = context
        .spawn(PreparedPlugin::from_input(engine, ()))
        .await
        .unwrap();
    let mut active_diagram = String::new();
    for name in ["capabilities-a", "capabilities-b"] {
        let mut model: Value =
            serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
        model["id"] = name.into();
        model["name"] = name.into();
        model["elements"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|element| element["id"] == "type-task")
            .expect("classifier")["properties"]["isActive"] = true.into();
        active_diagram = model["diagrams"][0]["id"].as_str().unwrap().to_owned();
        let document = events::prepare(&context, &model).await.unwrap();
        assert!(matches!(
            context
                .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
                .await
                .unwrap(),
            QueryOutcome::Answer(Ok(_))
        ));
    }
    let project_id = ProjectId::new("capabilities-a").unwrap();
    let untouched_id = ProjectId::new("capabilities-b").unwrap();
    let request = ModelingQuickEditRequested {
        project_id: project_id.clone(),
        diagram_id: active_diagram.clone(),
        element_id: "type-task".into(),
        action: UmlQuickCreate::Note,
        expected_revision: 0,
    };
    assert!(matches!(
        context
            .query::<ModelingQuickEdit>(Routing::Unscoped, request.clone())
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(1))
    ));
    assert!(matches!(
        context
            .query::<ModelingQuickEdit>(Routing::Unscoped, request)
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    for (action, expected_revision) in [
        (UmlQuickCreate::Constraint, 1),
        (UmlQuickCreate::Reception, 2),
        (UmlQuickCreate::Subtype, 3),
        (UmlQuickCreate::Supertype, 4),
    ] {
        assert!(
            matches!(context.query::<ModelingQuickEdit>(Routing::Unscoped, ModelingQuickEditRequested {
            project_id: project_id.clone(), diagram_id: active_diagram.clone(), element_id: "type-task".into(), action, expected_revision,
        }).await.unwrap(), QueryOutcome::Answer(Ok(revision)) if revision == expected_revision + 1)
        );
    }
    let document = projects.snapshot(&project_id).unwrap();
    assert_eq!(document.revision(), 5);
    assert_eq!(projects.snapshot(&untouched_id).unwrap().revision(), 0);
    let section = events::projection(&context, &projects, &project_id)
        .await
        .unwrap()
        .section;
    for kind in ["uml.note", "uml.constraint", "uml.reception"] {
        assert!(section.elements.iter().any(|element| element.kind == kind));
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
            .filter(|relation| relation.kind == "uml.generalization")
            .all(|relation| [relation.from.as_str(), relation.to.as_str()]
                .iter()
                .all(|id| section
                    .elements
                    .iter()
                    .any(|element| element.id == *id && element.kind == "rust.struct")))
    );
    assert!(
        section
            .elements
            .iter()
            .any(|element| element.id == "file-task")
    );
    engine.dispose().await.unwrap();
    assert!(matches!(
        context
            .query::<ModelingQuickEdit>(
                Routing::Unscoped,
                ModelingQuickEditRequested {
                    project_id: project_id.clone(),
                    diagram_id: active_diagram,
                    element_id: "type-task".into(),
                    action: UmlQuickCreate::Note,
                    expected_revision: 5,
                }
            )
            .await
            .unwrap(),
        QueryOutcome::Miss
    ));
    provider.dispose().await.unwrap();
}
