use bellatrix_mde::{Project, Scope};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingRename, ModelingRenameRequested, ModelingSetExplorerParent,
    ModelingSetExplorerParentRequested, ModelingSetOwner, ModelingSetOwnerRequested, with_model,
};
use bellatrix_mde_cordis::{ModelingRun, ModelingRunRequested, ModelingTaskKind};
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

    // The same public event path moves an existing element without duplicating its edge.
    let hierarchy: Project = serde_json::from_value(serde_json::json!({
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
            document: make_document(&hierarchy),
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
    quick_project.id = "quick-tool-flow".into();
    let diagram = quick_project.diagrams.iter_mut().find(|diagram| diagram.id == "diagram-tasks").unwrap();
    let mut repeated = diagram.views.iter().find(|view| view.id == "view-task").unwrap().clone();
    repeated.id = "view-task-repeat".into();
    repeated.geometry.translate(300.0, 150.0);
    diagram.views.push(repeated);
    diagram.views.push(serde_json::from_value(serde_json::json!({
        "id":"visual-only", "geometry":{"kind":"rect","x":700.0,"y":100.0,"width":80.0,"height":50.0},
        "draft":{"notation_kind":"uml.class"}
    })).unwrap());
    let quick_id = ProjectId::new(quick_project.id.clone()).unwrap();
    assert!(matches!(ctx.query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested {
        document: make_document(&quick_project),
    }).await.unwrap(), QueryOutcome::Answer(Ok(_))));
    assert!(matches!(ctx.query::<bellatrix_mde_cordis::ModelingCreateElementOrVisualDraft>(Routing::Unscoped,
        bellatrix_mde_cordis::ModelingCreateElementRequested {
            project_id: quick_id.clone(), diagram_id: "diagram-tasks".into(),
            kind: bellatrix_mde::commands::ModelingElementKind::Class, name: Some("Quick class".into()),
            owner_id: None, x: 80.0, y: 90.0, expected_revision: 0,
        }).await.unwrap(), QueryOutcome::Answer(Ok(1))));
    assert!(matches!(ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationOrVisualDraft>(Routing::Unscoped,
        bellatrix_mde_cordis::ModelingCreateRelationRequested {
            project_id: quick_id.clone(), diagram_id: "diagram-tasks".into(),
            kind: bellatrix_mde::ModelingRelationKind::Association, from: "type-list".into(),
            to: "type-task".into(), expected_revision: 1,
        }).await.unwrap(), QueryOutcome::Answer(Ok(2))));
    let quick_snapshot = service.snapshot(&quick_id).unwrap();
    let quick_section: bellatrix_mde::ModelingSection = serde_json::from_value(
        quick_snapshot.section("bellatrix.mde").unwrap().clone()).unwrap();
    let diagram = quick_section.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks").unwrap();
    let inventory = bellatrix_mde::visual_components(&quick_section);
    assert_eq!(inventory.iter().filter(|component| component.diagram_id == diagram.id).count(), diagram.views.len());
    assert!(inventory.iter().any(|component| component.diagram_id == diagram.id && component.label == "Quick class"));
    let new_edge = diagram.views.last().unwrap();
    assert!(matches!(&new_edge.geometry, bellatrix_mde::Geometry::Path { segments, .. } if !segments.is_empty()));
    assert!(inventory.iter().any(|component| component.view_id == new_edge.id));

    let exact_request = |source: &str, target: &str, from: Option<&str>, to: Option<&str>, revision| {
        bellatrix_mde_cordis::ModelingCreateRelationAtViewsRequested {
            project_id: quick_id.clone(), diagram_id: "diagram-tasks".into(),
            kind: bellatrix_mde::ModelingRelationKind::Association,
            source_view_id: source.into(), target_view_id: target.into(),
            from: from.map(str::to_owned), to: to.map(str::to_owned), expected_revision: revision,
        }
    };
    assert!(matches!(ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(Routing::Unscoped,
        exact_request("view-task-repeat", "visual-only", Some("type-task"), None, 2)).await.unwrap(), QueryOutcome::Answer(Ok(3))));
    assert!(matches!(ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(Routing::Unscoped,
        exact_request("visual-only", "view-task-repeat", None, Some("type-task"), 3)).await.unwrap(), QueryOutcome::Answer(Ok(4))));
    assert!(matches!(ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(Routing::Unscoped,
        exact_request("missing", "visual-only", Some("type-task"), None, 4)).await.unwrap(), QueryOutcome::Answer(Err(_))));
    assert!(matches!(ctx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(Routing::Unscoped,
        exact_request("view-task", "visual-only", Some("type-task"), None, 2)).await.unwrap(), QueryOutcome::Answer(Err(_))));
    let exact_snapshot = service.snapshot(&quick_id).unwrap();
    assert_eq!(exact_snapshot.revision(), 4);
    let exact_section: bellatrix_mde::ModelingSection = serde_json::from_value(exact_snapshot.section("bellatrix.mde").unwrap().clone()).unwrap();
    let exact_diagram = exact_section.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks").unwrap();
    let edges = &exact_diagram.views[exact_diagram.views.len() - 2..];
    assert_eq!(edges[0].endpoints.as_ref().and_then(|ends| ends.source_view_id.as_deref()), Some("view-task-repeat"));
    assert_eq!(edges[0].endpoints.as_ref().and_then(|ends| ends.target_view_id.as_deref()), Some("visual-only"));
    assert_eq!(edges[1].endpoints.as_ref().and_then(|ends| ends.source_view_id.as_deref()), Some("visual-only"));
    assert_eq!(edges[1].endpoints.as_ref().and_then(|ends| ends.target_view_id.as_deref()), Some("view-task-repeat"));
    assert_eq!(exact_section.relations, quick_section.relations);

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
