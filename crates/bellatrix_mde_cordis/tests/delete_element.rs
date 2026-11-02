use bellatrix_mde_cordis::{MdePlugin, ModelingDeleteElement, ModelingDeleteElementRequested, with_model};
use bellatrix_mde::{ModelingSection, Project};
use bellatrix_project::{ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, PreparedPlugin, Routing};
use std::sync::Arc;

#[tokio::test]
async fn deleting_a_diagram_class_preserves_other_projects_and_revisions() {
    let ctx = Context::new(); let projects = Arc::new(ProjectService::new());
    let provider = ctx.spawn(PreparedPlugin::from_input(ProjectPlugin::new(Arc::clone(&projects)), Arc::clone(&projects))).await.unwrap();
    let mde = ctx.spawn(PreparedPlugin::from_input(MdePlugin, ())).await.unwrap();
    for id in ["delete-a", "delete-b"] {
        let mut model: Project = serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
        model.id = id.into(); model.name = id.into();
        let document = with_model(ProjectDocument::new(ProjectId::new(id).unwrap(), id).unwrap(), &model).unwrap();
        assert!(matches!(ctx.query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document }).await.unwrap(), QueryOutcome::Answer(Ok(_))));
    }
    let a = ProjectId::new("delete-a").unwrap(); let b = ProjectId::new("delete-b").unwrap();
    let request = ModelingDeleteElementRequested { project_id: a.clone(), element_id: "type-task".into(), expected_revision: 0 };
    assert!(matches!(ctx.query::<ModelingDeleteElement>(Routing::Unscoped, request.clone()).await.unwrap(), QueryOutcome::Answer(Ok(1))));
    assert!(matches!(ctx.query::<ModelingDeleteElement>(Routing::Unscoped, request).await.unwrap(), QueryOutcome::Answer(Err(_))));
    let document = projects.snapshot(&a).unwrap();
    assert_eq!(document.revision(), 1); assert_eq!(projects.snapshot(&b).unwrap().revision(), 0);
    let section: ModelingSection = serde_json::from_value(document.section("bellatrix.bellatrix-mde-modeling").unwrap().clone()).unwrap();
    for id in ["type-task", "field-title", "field-completed"] { assert!(!section.elements.iter().any(|element| element.id == id)); }
    assert!(section.elements.iter().any(|element| element.id == "type-list"));
    assert!(section.elements.iter().any(|element| element.id == "file-task"));
    assert!(section.diagrams.iter().all(|diagram| diagram.views.iter().all(|view| view.subject.as_deref() != Some("type-task"))));
    mde.dispose().await.unwrap();
    assert!(matches!(ctx.query::<ModelingDeleteElement>(Routing::Unscoped, ModelingDeleteElementRequested {
        project_id: b.clone(), element_id: "type-task".into(), expected_revision: 0,
    }).await.unwrap(), QueryOutcome::Miss));
    provider.dispose().await.unwrap();
}
