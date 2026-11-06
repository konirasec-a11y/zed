use bellatrix_project::{
    ProjectClose, ProjectCloseRequested, ProjectCreate, ProjectCreateRequested, ProjectDocument,
    ProjectId, ProjectOpen, ProjectOpenRequested, ProjectPlugin, ProjectReplace,
    ProjectReplaceRequested, ProjectSave, ProjectSaveRequested, ProjectSavedEvent, ProjectService,
    ProjectStateChangedEvent,
};
use cordis::event::{QueryOutcome, observer_sync};
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
static CASE: AtomicU64 = AtomicU64::new(0);

fn document(id: &str) -> ProjectDocument {
    ProjectDocument::new(ProjectId::new(id).unwrap(), id)
        .unwrap()
        .with_section("future.engine", serde_json::json!({"opaque":{"a":[1,2,3]}}))
        .unwrap()
}
fn temp_dir() -> std::path::PathBuf {
    let name = format!(
        "bellatrix-bellatrix-project-{}-{}",
        std::process::id(),
        CASE.fetch_add(1, Ordering::Relaxed)
    );
    let path = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&path).unwrap();
    path
}
async fn admit(ctx: &Context, service: Arc<ProjectService>) -> cordis::FiberHandle {
    let plugin = ProjectPlugin::new(service);
    let input = plugin.prepare(()).unwrap();
    ctx.spawn(PreparedPlugin::from_input(plugin, input))
        .await
        .unwrap()
}

#[tokio::test]
async fn unknown_engine_state_survives_and_identical_write_is_not_an_event() {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let fiber = admit(&context, Arc::clone(&service)).await;
    let events = Arc::new(Mutex::new(0));
    let observed = Arc::clone(&events);
    let _listener = context
        .on::<ProjectStateChangedEvent, _>(observer_sync(
            move |_, _: bellatrix_project::ProjectStateChanged| {
                *observed.lock().unwrap() += 1;
                Ok::<_, std::convert::Infallible>(())
            },
        ))
        .unwrap();
    let doc = document("todo-a");
    let id = doc.id().clone();
    context
        .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document: doc })
        .await
        .unwrap();
    let old = service
        .snapshot(&id)
        .unwrap()
        .section("future.engine")
        .unwrap()
        .clone();
    let same = context
        .query::<ProjectReplace>(
            Routing::Unscoped,
            ProjectReplaceRequested {
                id: id.clone(),
                namespace: "future.engine".into(),
                expected_revision: 0,
                state: old.clone(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(same, QueryOutcome::Answer(Ok(_))));
    assert_eq!(*events.lock().unwrap(), 0);
    assert_eq!(service.snapshot(&id).unwrap().revision(), 0);
    let path = temp_dir();
    let target = path.join("todo.bellatrix.json");
    context
        .query::<ProjectSave>(
            Routing::Unscoped,
            ProjectSaveRequested {
                expected_bytes: None,
                document: service.snapshot(&id).unwrap(),
                path: target.clone(),
            },
        )
        .await
        .unwrap();
    context
        .query::<ProjectClose>(
            Routing::Unscoped,
            ProjectCloseRequested {
                id: id.clone(),
                discard_unsaved: false,
            },
        )
        .await
        .unwrap();
    context
        .query::<ProjectOpen>(Routing::Unscoped, ProjectOpenRequested { path: target })
        .await
        .unwrap();
    assert_eq!(
        service.snapshot(&id).unwrap().section("future.engine"),
        Some(&old)
    );
    fiber.dispose().await.unwrap();
    std::fs::remove_dir_all(path).unwrap();
}

#[tokio::test]
async fn one_project_cannot_overwrite_another_projects_file() {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let fiber = admit(&context, Arc::clone(&service)).await;
    for id in ["todo-a", "todo-b"] {
        context
            .query::<ProjectCreate>(
                Routing::Unscoped,
                ProjectCreateRequested {
                    document: document(id),
                },
            )
            .await
            .unwrap();
    }
    let events = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&events);
    let _listener = context
        .on::<ProjectSavedEvent, _>(observer_sync(
            move |_, event: bellatrix_project::ProjectSaved| {
                observed.lock().unwrap().push(event);
                Ok::<_, std::convert::Infallible>(())
            },
        ))
        .unwrap();
    let path = temp_dir();
    let destination = path.join("bellatrix-project.json");
    let a = ProjectId::new("todo-a").unwrap();
    let b = ProjectId::new("todo-b").unwrap();
    let a_save = context
        .query::<ProjectSave>(
            Routing::Unscoped,
            ProjectSaveRequested {
                expected_bytes: None,
                document: service.snapshot(&a).unwrap(),
                path: destination.clone(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(a_save, QueryOutcome::Answer(Ok(0))));
    let b_save = context
        .query::<ProjectSave>(
            Routing::Unscoped,
            ProjectSaveRequested {
                expected_bytes: None,
                document: service.snapshot(&b).unwrap(),
                path: destination.clone(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(b_save, QueryOutcome::Answer(Err(_))));
    assert!(service.is_dirty(&b).unwrap());
    assert_eq!(events.lock().unwrap().len(), 1);
    let bytes = std::fs::read(&destination).unwrap();
    let persisted: ProjectDocument = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(persisted.id(), &a);
    fiber.dispose().await.unwrap();
    std::fs::remove_dir_all(path).unwrap();
}
