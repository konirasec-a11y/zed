use bellatrix_project::*;
use cordis::event::{QueryOutcome, observer_sync};
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn external_storage_uses_cordis_lifecycle_and_emits_only_confirmed_save() {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let plugin = ProjectPlugin::new(Arc::clone(&service));
    let input = plugin.prepare(()).expect("prepare plugin");
    let fiber = context
        .spawn(PreparedPlugin::from_input(plugin, input))
        .await
        .expect("activate");
    let saved = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&saved);
    let _listener = context
        .on::<ProjectSavedEvent, _>(observer_sync(move |_, event: ProjectSaved| {
            observed.lock().expect("events").push(event);
            Ok::<_, std::convert::Infallible>(())
        }))
        .expect("subscribe");
    let id = ProjectId::new("native-buffer").expect("id");
    let imported = context
        .query::<ProjectImport>(
            Routing::Unscoped,
            ProjectImportRequested {
                document: ProjectDocument::new(id.clone(), "Native").expect("document"),
                is_saved: false,
            },
        )
        .await
        .expect("query");
    assert!(matches!(imported, QueryOutcome::Answer(Ok(_))));
    assert!(service.is_dirty(&id).expect("dirty"));
    let target = std::env::temp_dir().join(format!(
        "bellatrix-cordis-external-{}.json",
        std::process::id()
    ));
    assert!(!target.exists());
    let prepared = context
        .query::<ProjectPrepareExternalSave>(
            Routing::Unscoped,
            ProjectPrepareExternalSaveRequested {
                id: id.clone(),
                expected_revision: 0,
                path: target.clone(),
            },
        )
        .await
        .expect("prepare query");
    let QueryOutcome::Answer(Ok(ticket)) = prepared else {
        panic!("prepare should yield a ticket")
    };
    assert!(saved.lock().expect("events").is_empty());
    let receipt = ProjectAcknowledgeExternalSaveRequested {
        written_bytes: Arc::from(ticket.bytes()),
        ticket,
    };
    let result = context
        .query::<ProjectAcknowledgeExternalSave>(Routing::Unscoped, receipt.clone())
        .await
        .expect("ack query");
    assert!(matches!(result, QueryOutcome::Answer(Ok(0))));
    assert!(!service.is_dirty(&id).expect("dirty"));
    let duplicate = context
        .query::<ProjectAcknowledgeExternalSave>(Routing::Unscoped, receipt)
        .await
        .expect("duplicate query");
    assert!(matches!(
        duplicate,
        QueryOutcome::Answer(Err(ProjectError::InvalidSaveTicket))
    ));
    let events = saved.lock().expect("events").clone();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, id);
    assert_eq!(events[0].revision, 0);
    assert_eq!(events[0].path, target);
    assert!(
        !target.exists(),
        "external-storage path must never perform filesystem I/O"
    );
    fiber.dispose().await.expect("dispose");
    assert!(matches!(
        context
            .query::<ProjectPrepareExternalSave>(
                Routing::Unscoped,
                ProjectPrepareExternalSaveRequested {
                    id,
                    expected_revision: 0,
                    path: target,
                }
            )
            .await
            .expect("query after disposal"),
        QueryOutcome::Miss
    ));
}

#[tokio::test]
async fn snapshot_cleanup_emits_closed_only_for_the_captured_unchanged_document() {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let plugin = ProjectPlugin::new(Arc::clone(&service));
    let input = plugin.prepare(()).expect("plugin");
    let fiber = context
        .spawn(PreparedPlugin::from_input(plugin, input))
        .await
        .expect("activate");
    let closed = Arc::new(Mutex::new(0));
    let observed = Arc::clone(&closed);
    let _listener = context
        .on::<ProjectClosedEvent, _>(observer_sync(move |_, _: ProjectClosed| {
            *observed.lock().expect("events") += 1;
            Ok::<_, std::convert::Infallible>(())
        }))
        .expect("subscribe");
    let id = ProjectId::new("preview-cleanup").expect("id");
    let imported = context
        .query::<ProjectImport>(
            Routing::Unscoped,
            ProjectImportRequested {
                document: ProjectDocument::new(id.clone(), "Preview").expect("document"),
                is_saved: false,
            },
        )
        .await
        .expect("import");
    let QueryOutcome::Answer(Ok(imported)) = imported else {
        panic!("import result")
    };
    let edited = context
        .query::<ProjectReplace>(
            Routing::Unscoped,
            ProjectReplaceRequested {
                id: id.clone(),
                namespace: "example.engine".into(),
                expected_revision: 0,
                state: serde_json::json!({"edit":true}),
            },
        )
        .await
        .expect("edit");
    assert!(matches!(edited, QueryOutcome::Answer(Ok(_))));
    let failed = context
        .query::<ProjectCloseSnapshot>(
            Routing::Unscoped,
            ProjectCloseSnapshotRequested { document: imported },
        )
        .await
        .expect("stale cleanup");
    assert!(matches!(
        failed,
        QueryOutcome::Answer(Err(ProjectError::SessionChanged(_)))
    ));
    assert_eq!(*closed.lock().expect("events"), 0);
    let current = service.snapshot(&id).expect("preserved");
    let closed_result = context
        .query::<ProjectCloseSnapshot>(
            Routing::Unscoped,
            ProjectCloseSnapshotRequested { document: current },
        )
        .await
        .expect("cleanup");
    assert!(matches!(closed_result, QueryOutcome::Answer(Ok(()))));
    assert_eq!(*closed.lock().expect("events"), 1);
    fiber.dispose().await.expect("dispose");
}
