#[path = "support/events.rs"]
mod events;
use bellatrix_mde_cordis::{
    MdePlugin, MdeUiBridge, ModelingRename, ModelingRenameRequested, ModelingSetExplorerParent,
    ModelingSetExplorerParentRequested, ModelingSetOwner, ModelingSetOwnerRequested,
};
use bellatrix_mde_cordis::{
    ModelingExecutionTrace, ModelingExecutionTraceEvent, ModelingRun, ModelingRunRequested,
    ModelingTaskKind,
};
use bellatrix_project::{
    ProjectClose, ProjectCloseRequested, ProjectCreate, ProjectCreateRequested, ProjectDocument,
    ProjectId, ProjectOpen, ProjectOpenRequested, ProjectPlugin, ProjectReplace,
    ProjectReplaceRequested, ProjectSave, ProjectSaveRequested, ProjectService,
    ProjectStateChanged, ProjectStateChangedEvent,
};
use cordis::event::{QueryOutcome, observer_sync};
use cordis::{Context, FiberState, Plugin, PreparedPlugin, Routing};
use serde_json::Value;
use std::sync::{Arc, Mutex};

fn fixture() -> Value {
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

async fn make_document(context: &Context, model: &Value) -> ProjectDocument {
    events::prepare(context, model).await.unwrap()
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
    b["id"] = "bellatrix-project-other".into();
    b["name"] = "Other Project".into();
    let a_id = ProjectId::new(a["id"].as_str().unwrap()).unwrap();
    let b_id = ProjectId::new(b["id"].as_str().unwrap()).unwrap();
    for model in [&a, &b] {
        let response = ctx
            .query::<ProjectCreate>(
                Routing::Unscoped,
                ProjectCreateRequested {
                    document: make_document(&ctx, model).await,
                },
            )
            .await
            .unwrap();
        assert!(matches!(response, QueryOutcome::Answer(Ok(_))));
    }
    let bridge_a = MdeUiBridge::start(&ctx, a_id.clone()).unwrap();
    let bridge_b = MdeUiBridge::start(&ctx, b_id.clone()).unwrap();
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
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !bridge_a
            .execution_journal()
            .snapshot()
            .events
            .iter()
            .any(|event| event.action == "command.apply")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(bridge_b.execution_journal().snapshot().events.is_empty());
    assert_eq!(service.snapshot(&a_id).unwrap().revision(), 1);
    assert_eq!(service.snapshot(&b_id).unwrap().revision(), 0);
    {
        let changes = observed.lock().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].id, a_id);
        assert_eq!(changes[0].revision, 1);
    }

    // The same public event path moves an existing element without duplicating its edge.
    let hierarchy: Value = serde_json::from_value(serde_json::json!({
        "id": "hierarchy", "name": "Hierarchy", "root_model": "model",
        "elements": [
            {"id": "model", "kind": "uml.model", "name": "Model"},
            {"id": "package", "kind": "uml.package", "name": "Package"},
            {"id": "class", "kind": "uml.class", "name": "Class"},
            {"id": "other", "kind": "uml.class", "name": "Other"}
        ],
        "relations": [
            {"id": "owns-package", "kind": "owns", "from": "model", "to": "package"},
            {"id": "owns-class", "kind": "owns", "from": "model", "to": "class"},
            {"id": "owns-other", "kind": "owns", "from": "model", "to": "other"}
        ]
    }))
    .unwrap();
    let hierarchy_id = ProjectId::new("hierarchy").unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped,
        ProjectCreateRequested {
            document: make_document(&ctx, &hierarchy).await,
        },
    )
    .await
    .unwrap();
    let ownership = |owner: &str, expected_revision| ModelingSetOwnerRequested {
        project_id: hierarchy_id.clone(),
        element_id: "class".into(),
        owner_id: Some(owner.into()),
        expected_revision,
    };
    assert!(matches!(
        ctx.query::<ModelingSetOwner>(Routing::Unscoped, ownership("package", 0))
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(1))
    ));
    assert!(matches!(
        ctx.query::<ModelingSetOwner>(Routing::Unscoped, ownership("model", 0))
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    assert!(matches!(
        ctx.query::<ModelingSetOwner>(Routing::Unscoped, ownership("class", 1))
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    let snapshot = service.snapshot(&hierarchy_id).unwrap();
    assert_eq!(snapshot.revision(), 1);
    let section = snapshot.section("bellatrix.mde").unwrap();
    let edges = section["relations"].as_array().unwrap();
    let class_edges: Vec<_> = edges
        .iter()
        .filter(|edge| edge["kind"] == "owns" && edge["to"] == "class")
        .collect();
    assert_eq!(class_edges.len(), 1);
    assert_eq!(class_edges[0]["id"], "owns-class");
    assert_eq!(class_edges[0]["from"], "package");

    // Explorer organization accepts a class under another class without changing UML ownership.
    let placement =
        |subject: &str, parent: &str, expected_revision| ModelingSetExplorerParentRequested {
            project_id: hierarchy_id.clone(),
            subject_id: subject.into(),
            parent_id: Some(parent.into()),
            parent_occurrence: None,
            expected_revision,
        };
    assert!(matches!(
        ctx.query::<ModelingSetExplorerParent>(Routing::Unscoped, placement("class", "other", 1))
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(2))
    ));
    assert!(matches!(
        ctx.query::<ModelingSetExplorerParent>(Routing::Unscoped, placement("other", "class", 2))
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    assert!(matches!(
        ctx.query::<ModelingSetExplorerParent>(Routing::Unscoped, placement("class", "model", 1))
            .await
            .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    let snapshot = service.snapshot(&hierarchy_id).unwrap();
    assert_eq!(snapshot.revision(), 2);
    let section = snapshot.section("bellatrix.mde").unwrap();
    assert_eq!(section["explorer_parents"]["class"], "other");
    let class_edge = section["relations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| edge["id"] == "owns-class")
        .unwrap();
    assert_eq!(class_edge["from"], "package");

    // The Quick Tool event path publishes a View that the shared Explorer inventory sees.
    let mut quick_project = fixture();
    quick_project["id"] = "quick-tool-flow".into();
    let diagram = quick_project["diagrams"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|diagram| diagram["id"] == "diagram-tasks")
        .unwrap();
    let mut repeated = diagram["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|view| view["id"] == "view-task")
        .unwrap()
        .clone();
    repeated["id"] = "view-task-repeat".into();
    repeated["geometry"]["x"] = (repeated["geometry"]["x"].as_f64().unwrap() + 300.0).into();
    repeated["geometry"]["y"] = (repeated["geometry"]["y"].as_f64().unwrap() + 150.0).into();
    diagram["views"].as_array_mut().unwrap().push(repeated);
    diagram["views"].as_array_mut().unwrap().push(serde_json::json!({
        "id":"visual-only", "geometry":{"kind":"rect","x":700.0,"y":100.0,"width":80.0,"height":50.0},
        "draft":{"notation_kind":"uml.class"}
    }));
    let quick_id = ProjectId::new(quick_project["id"].as_str().unwrap()).unwrap();
    assert!(matches!(
        ctx.query::<ProjectCreate>(
            Routing::Unscoped,
            ProjectCreateRequested {
                document: make_document(&ctx, &quick_project).await,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(_))
    ));
    assert!(matches!(
        ctx.query::<bellatrix_mde_cordis::ModelingCreateElementOrVisualDraft>(
            Routing::Unscoped,
            bellatrix_mde_cordis::ModelingCreateElementRequested {
                project_id: quick_id.clone(),
                diagram_id: "diagram-tasks".into(),
                kind: bellatrix_mde_cordis::ModelingElementKind::Class,
                name: Some("Quick class".into()),
                owner_id: None,
                x: 80.0,
                y: 90.0,
                expected_revision: 0,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(1))
    ));
    assert!(matches!(
        ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationOrVisualDraft>(
            Routing::Unscoped,
            bellatrix_mde_cordis::ModelingCreateRelationRequested {
                owner_id: None,
                project_id: quick_id.clone(),
                diagram_id: "diagram-tasks".into(),
                kind: bellatrix_mde_cordis::ModelingRelationKind::Association,
                from: "type-list".into(),
                to: "type-task".into(),
                expected_revision: 1,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(2))
    ));
    let quick_snapshot = service.snapshot(&quick_id).unwrap();
    let quick_section = events::projection(&ctx, &service, &quick_id)
        .await
        .unwrap()
        .section;
    let diagram = quick_section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "diagram-tasks")
        .unwrap();
    let inventory = quick_section
        .explorer
        .added
        .iter()
        .filter_map(|row| row.view.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(
        inventory
            .iter()
            .filter(|component| component.diagram_id == diagram.id)
            .count(),
        diagram.views.len()
    );
    assert!(inventory.iter().any(|component| component.diagram_id == diagram.id && component.label == "Quick class"));
    let new_edge = diagram.views.last().unwrap();
    assert!(
        matches!(&new_edge.geometry, bellatrix_mde_cordis::Geometry::Path { segments, .. } if !segments.is_empty())
    );
    assert!(
        inventory
            .iter()
            .any(|component| component.view_id == new_edge.id)
    );

    let exact_request =
        |source: &str, target: &str, from: Option<&str>, to: Option<&str>, revision| {
            bellatrix_mde_cordis::ModelingCreateRelationAtViewsRequested {
                owner_id: None,
                project_id: quick_id.clone(),
                diagram_id: "diagram-tasks".into(),
                kind: bellatrix_mde_cordis::ModelingRelationKind::Association,
                source_view_id: source.into(),
                target_view_id: target.into(),
                from: from.map(str::to_owned),
                to: to.map(str::to_owned),
                expected_revision: revision,
            }
        };
    assert!(matches!(
        ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(
            Routing::Unscoped,
            exact_request(
                "view-task-repeat",
                "visual-only",
                Some("type-task"),
                None,
                2
            )
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(3))
    ));
    assert!(matches!(
        ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(
            Routing::Unscoped,
            exact_request(
                "visual-only",
                "view-task-repeat",
                None,
                Some("type-task"),
                3
            )
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(4))
    ));
    assert!(matches!(
        ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(
            Routing::Unscoped,
            exact_request("missing", "visual-only", Some("type-task"), None, 4)
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    assert!(matches!(
        ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(
            Routing::Unscoped,
            exact_request("view-task", "visual-only", Some("type-task"), None, 2)
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Err(_))
    ));
    let exact_snapshot = service.snapshot(&quick_id).unwrap();
    assert_eq!(exact_snapshot.revision(), 4);
    let exact_section = events::projection(&ctx, &service, &quick_id)
        .await
        .unwrap()
        .section;
    let exact_diagram = exact_section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "diagram-tasks")
        .unwrap();
    let edges = &exact_diagram.views[exact_diagram.views.len() - 2..];
    assert_eq!(
        edges[0]
            .endpoints
            .as_ref()
            .and_then(|ends| ends.source_view_id.as_deref()),
        Some("view-task-repeat")
    );
    assert_eq!(
        edges[0]
            .endpoints
            .as_ref()
            .and_then(|ends| ends.target_view_id.as_deref()),
        Some("visual-only")
    );
    assert_eq!(
        edges[1]
            .endpoints
            .as_ref()
            .and_then(|ends| ends.source_view_id.as_deref()),
        Some("visual-only")
    );
    assert_eq!(
        edges[1]
            .endpoints
            .as_ref()
            .and_then(|ends| ends.target_view_id.as_deref()),
        Some("view-task-repeat")
    );
    assert_eq!(
        exact_snapshot.section("bellatrix.mde").unwrap()["relations"],
        quick_snapshot.section("bellatrix.mde").unwrap()["relations"]
    );

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
    let id = ProjectId::new(model["id"].as_str().unwrap()).unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped,
        ProjectCreateRequested {
            document: make_document(&ctx, &model).await,
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
                document: service.snapshot(&id).unwrap(),
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
    let bridge = MdeUiBridge::start(&ctx, id.clone()).unwrap();
    let journal = bridge.execution_journal();
    let mut journal_changes = journal.subscribe();
    let trace_events = Arc::new(Mutex::new(
        Vec::<bellatrix_mde_cordis::ExecutionEvent>::new(),
    ));
    let trace_observer = Arc::clone(&trace_events);
    let _trace_listener = ctx
        .on::<ModelingExecutionTraceEvent, _>(observer_sync(
            move |_, batch: ModelingExecutionTrace| {
                trace_observer.lock().unwrap().extend(batch.events);
                Ok::<_, std::convert::Infallible>(())
            },
        ))
        .unwrap();
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
                    element_ids: None,
                    skeleton: false,
                },
            },
        )
        .await
        .unwrap();
    assert!(matches!(generated, QueryOutcome::Answer(Ok(_))));
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let observer_finished = trace_events.lock().unwrap().last().is_some_and(|event| {
                event.action == "generate_rust"
                    && event.phase == bellatrix_mde_cordis::TracePhase::Finish
            });
            let journal_finished = journal.snapshot().events.back().is_some_and(|event| {
                event.action == "generate_rust"
                    && event.phase == bellatrix_mde_cordis::TracePhase::Finish
            });
            if observer_finished && journal_finished {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    {
        let events = trace_events.lock().unwrap();
        assert!(events.windows(2).all(|pair| pair[0].run_id == pair[1].run_id && pair[0].sequence < pair[1].sequence));
        assert!(events.iter().all(|event| event.component == "cordis"));
        assert!(events.iter().any(|event| event.action == "generate_rust"
            && event.phase == bellatrix_mde_cordis::TracePhase::Start));
        assert_eq!(
            events.last().map(|event| event.action.as_str()),
            Some("generate_rust")
        );
    }
    let snapshot = journal.snapshot();
    assert!(journal_changes.has_changed().unwrap());
    journal_changes.borrow_and_update();
    assert!(
        snapshot
            .events
            .iter()
            .any(|event| event.action == "generate_rust")
    );
    assert!(
        snapshot
            .events
            .iter()
            .zip(snapshot.events.iter().skip(1))
            .all(|(first, second)| first.sequence < second.sequence)
    );
    journal.clear();
    assert!(journal_changes.has_changed().unwrap());
    assert!(journal.snapshot().events.is_empty());
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
                document: service.snapshot(&id).unwrap(),
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
    let id = ProjectId::new(model["id"].as_str().unwrap()).unwrap();
    ctx.query::<ProjectCreate>(
        Routing::Unscoped,
        ProjectCreateRequested {
            document: ProjectDocument::new(id.clone(), model["name"].as_str().unwrap()).unwrap(),
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
