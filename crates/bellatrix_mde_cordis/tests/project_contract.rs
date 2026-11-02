use bellatrix_mde_cordis::{MdePlugin, ModelingRename, ModelingRenameRequested, with_model};
use bellatrix_mde_cordis::{ModelingRun, ModelingRunRequested, ModelingTaskKind};
use bellatrix_mde::{Project, Scope};
use bellatrix_project::{
    ProjectClose, ProjectCloseRequested, ProjectCreate, ProjectCreateRequested, ProjectDocument,
    ProjectId, ProjectOpen, ProjectOpenRequested, ProjectPlugin, ProjectReplace,
    ProjectReplaceRequested, ProjectSave, ProjectSaveRequested, ProjectService,
    ProjectStateChanged, ProjectStateChangedEvent,
};
use cordis::event::{QueryOutcome, observer_sync};
use cordis::{Context, FiberState, Plugin, PreparedPlugin, Routing};
use std::sync::{Arc, Mutex};

fn fixture() -> Project {
    serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap()
}

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

fn make_document(model: &Project) -> ProjectDocument {
    let doc = ProjectDocument::new(
        ProjectId::new(model.id.clone()).unwrap(),
        model.name.clone(),
    )
    .unwrap();
    with_model(doc, model).unwrap()
}

#[tokio::test]
async fn public_event_updates_only_requested_project_and_survives_modeling_dispose() {
    let ctx = Context::new();
    let engine = admit(&ctx, MdePlugin).await;
    assert_eq!(engine.state(), FiberState::Pending);
    let service = Arc::new(ProjectService::new());
    let storage = admit(&ctx, ProjectPlugin::new(Arc::clone(&service))).await;
    assert_eq!(engine.ready().await.unwrap(), FiberState::Active);

    let observed = Arc::new(Mutex::new(Vec::<ProjectStateChanged>::new()));
    let observer = Arc::clone(&observed);
    let _listener = ctx
        .on::<ProjectStateChangedEvent, _>(observer_sync(move |_, change: ProjectStateChanged| {
            observer.lock().unwrap().push(change);
            Ok::<_, std::convert::Infallible>(())
        }))
        .unwrap();

    let a = fixture();
    let mut b = a.clone();
    b.id = "bellatrix-project-other".into();
    b.name = "Other Project".into();
    let a_id = ProjectId::new(a.id.clone()).unwrap();
    let b_id = ProjectId::new(b.id.clone()).unwrap();
    for model in [&a, &b] {
        let response = ctx
            .query::<ProjectCreate>(
                Routing::Unscoped,
                ProjectCreateRequested {
                    document: make_document(model),
                },
            )
            .await
            .unwrap();
        assert!(matches!(response, QueryOutcome::Answer(Ok(_))));
    }
    let result = ctx
        .query::<ModelingRename>(
            Routing::Unscoped,
            ModelingRenameRequested {
                project_id: a_id.clone(),
                element_id: "type-task".into(),
                new_name: "Ticket".into(),
                expected_revision: 0,
            },
        )
        .await
        .unwrap();
    assert!(matches!(result, QueryOutcome::Answer(Ok(1))));
    assert_eq!(service.snapshot(&a_id).unwrap().revision(), 1);
    assert_eq!(service.snapshot(&b_id).unwrap().revision(), 0);
    {
        let changes = observed.lock().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].id, a_id);
        assert_eq!(changes[0].revision, 1);
    }

    engine.dispose().await.unwrap();
    let missed = ctx
        .query::<ModelingRename>(
            Routing::Unscoped,
            ModelingRenameRequested {
                project_id: b_id.clone(),
                element_id: "type-task".into(),
                new_name: "Still Task".into(),
                expected_revision: 0,
            },
        )
        .await
        .unwrap();
    assert!(matches!(missed, QueryOutcome::Miss));
    assert_eq!(service.snapshot(&b_id).unwrap().revision(), 0);
    storage.dispose().await.unwrap();
}

#[tokio::test]
async fn revision_conflict_save_close_reopen_and_missing_provider() {
    let ctx = Context::new();
    let service = Arc::new(ProjectService::new());
    let storage = admit(&ctx, ProjectPlugin::new(Arc::clone(&service))).await;
    let engine = admit(&ctx, MdePlugin).await;
    let model = fixture();
    let id = ProjectId::new(model.id.clone()).unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped,
        ProjectCreateRequested {
            document: make_document(&model),
        },
    )
    .await
    .unwrap();

    let replace = |expected_revision, value| ProjectReplaceRequested {
        id: id.clone(),
        namespace: "custom.settings".into(),
        expected_revision,
        state: value,
    };
    let first = ctx
        .query::<ProjectReplace>(
            Routing::Unscoped,
            replace(0, serde_json::json!({"enabled":true})),
        )
        .await
        .unwrap();
    assert!(matches!(first, QueryOutcome::Answer(Ok(_))));
    let stale = ctx
        .query::<ProjectReplace>(
            Routing::Unscoped,
            replace(0, serde_json::json!({"enabled":false})),
        )
        .await
        .unwrap();
    assert!(matches!(stale, QueryOutcome::Answer(Err(_))));
    let unchanged = ctx
        .query::<ProjectReplace>(
            Routing::Unscoped,
            replace(1, serde_json::json!({"enabled":true})),
        )
        .await
        .unwrap();
    assert!(matches!(unchanged, QueryOutcome::Answer(Ok(_))));
    assert_eq!(service.snapshot(&id).unwrap().revision(), 1);
    assert!(service.is_dirty(&id).unwrap());

    let directory = std::env::temp_dir().join(format!(
        "bellatrix-test-{}-{:?}",
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
    assert!(!service.is_dirty(&id).unwrap());
    let closed = ctx
        .query::<ProjectClose>(
            Routing::Unscoped,
            ProjectCloseRequested {
                id: id.clone(),
                discard_unsaved: false,
            },
        )
        .await
        .unwrap();
    assert!(matches!(closed, QueryOutcome::Answer(Ok(()))));
    let opened = ctx
        .query::<ProjectOpen>(
            Routing::Unscoped,
            ProjectOpenRequested { path: path.clone() },
        )
        .await
        .unwrap();
    assert!(matches!(opened, QueryOutcome::Answer(Ok(_))));
    assert_eq!(service.snapshot(&id).unwrap().revision(), 1);
    assert!(
        service
            .snapshot(&id)
            .unwrap()
            .section("custom.settings")
            .is_some()
    );
    let destination = directory.join("generated-rust");
    let generated = ctx
        .query::<ModelingRun>(
            Routing::Unscoped,
            ModelingRunRequested {
                project_id: id.clone(),
                correlation_id: 70,
                expected_revision: 1,
                destination: destination.clone(),
                replace_generated: false,
                task: ModelingTaskKind::GenerateRust {
                    scope: Scope::Project,
                    skeleton: false,
                },
            },
        )
        .await
        .unwrap();
    assert!(matches!(generated, QueryOutcome::Answer(Ok(_))));
    assert!(destination.join("Cargo.toml").exists());
    let documentation = directory.join("documentation");
    let exported = ctx
        .query::<ModelingRun>(
            Routing::Unscoped,
            ModelingRunRequested {
                project_id: id.clone(),
                correlation_id: 71,
                expected_revision: 1,
                destination: documentation.clone(),
                replace_generated: false,
                task: ModelingTaskKind::ExportHtml,
            },
        )
        .await
        .unwrap();
    assert!(matches!(exported, QueryOutcome::Answer(Ok(_))));
    assert!(documentation.join("index.html").exists());

    storage.dispose().await.unwrap();
    assert_eq!(engine.ready().await.unwrap(), FiberState::Pending);
    assert!(matches!(
        ctx.query::<ProjectSave>(
            Routing::Unscoped,
            ProjectSaveRequested {
                id: id.clone(),
                path: path.clone()
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Miss
    ));
    let recovered = admit(&ctx, ProjectPlugin::new(Arc::clone(&service))).await;
    assert_eq!(engine.ready().await.unwrap(), FiberState::Active);
    assert_eq!(service.snapshot(&id).unwrap().revision(), 1);
    engine.dispose().await.unwrap();
    recovered.dispose().await.unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn two_concurrent_edits_cannot_commit_the_same_revision() {
    let ctx = Context::new();
    let service = Arc::new(ProjectService::new());
    let handle = admit(&ctx, ProjectPlugin::new(Arc::clone(&service))).await;
    let model = fixture();
    let id = ProjectId::new(model.id.clone()).unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped,
        ProjectCreateRequested {
            document: make_document(&model),
        },
    )
    .await
    .unwrap();
    let left = ctx.query::<ProjectReplace>(
        Routing::Unscoped,
        ProjectReplaceRequested {
            id: id.clone(),
            namespace: "engine.a".into(),
            expected_revision: 0,
            state: serde_json::json!({"value":1}),
        },
    );
    let right = ctx.query::<ProjectReplace>(
        Routing::Unscoped,
        ProjectReplaceRequested {
            id: id.clone(),
            namespace: "engine.b".into(),
            expected_revision: 0,
            state: serde_json::json!({"value":2}),
        },
    );
    let (a, b) = tokio::join!(left, right);
    let values = [a.unwrap(), b.unwrap()];
    let success = values
        .iter()
        .filter(|v| matches!(v, QueryOutcome::Answer(Ok(_))))
        .count();
    let failed = values
        .iter()
        .filter(|v| matches!(v, QueryOutcome::Answer(Err(_))))
        .count();
    assert_eq!((success, failed), (1, 1));
    assert_eq!(service.snapshot(&id).unwrap().revision(), 1);
    handle.dispose().await.unwrap();
}
