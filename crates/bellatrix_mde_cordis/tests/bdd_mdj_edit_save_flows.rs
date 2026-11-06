use bellatrix_mde_cordis::{MODELING_NAMESPACE, Style};
use bellatrix_mde_cordis::{
    MdePlugin, MdeUiBridge, MdeUiCommand, ModelingDocumentFormat, ModelingExportMdj,
    ModelingExportMdjRequested, ModelingPrepareDocument, ModelingPrepareDocumentRequested,
    ModelingSetViewStyleRequested,
};
use bellatrix_project::*;
use cordis::{
    Context, FiberHandle, Plugin, PreparedPlugin, Routing,
    event::{QueryOutcome, observer_sync},
};
use serde_json::json;
use std::{
    convert::Infallible,
    error::Error,
    fmt::Display,
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

type FlowResult<T> = Result<T, Box<dyn Error>>;
const SOURCE: &[u8] = include_bytes!("../../bellatrix_mde/fixtures/bdd/staruml/presentation.mdj");
const SPECIFICATION: &str = include_str!("features/mdj_edit_save_lifecycle.feature");

fn answer<T, E: Display>(outcome: QueryOutcome<Result<T, E>>) -> FlowResult<T> {
    match outcome {
        QueryOutcome::Answer(Ok(value)) => Ok(value),
        QueryOutcome::Answer(Err(error)) => Err(std::io::Error::other(error.to_string()).into()),
        QueryOutcome::Miss => Err("Cordis responder unavailable".into()),
    }
}

struct SavedFlow(PathBuf);
impl SavedFlow {
    fn new(name: &str) -> FlowResult<Self> {
        let directory = std::env::temp_dir().join(format!(
            "bellatrix-mdj-save-bdd-{}-{name}",
            std::process::id()
        ));
        fs::create_dir(&directory)?;
        Ok(Self(directory))
    }
}
impl Drop for SavedFlow {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not clean MDJ save BDD directory {}: {error}",
                self.0.display()
            );
        }
    }
}

async fn start() -> FlowResult<(Context, Arc<ProjectService>, FiberHandle, FiberHandle)> {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&service)),
            Arc::clone(&service),
        ))
        .await?;
    let plugin = MdePlugin;
    plugin.prepare(())?;
    let plugin = context
        .spawn(PreparedPlugin::from_input(plugin, ()))
        .await?;
    Ok((context, service, provider, plugin))
}

async fn document(context: &Context, bytes: &[u8]) -> FlowResult<ProjectDocument> {
    Ok(answer(
        context
            .query::<ModelingPrepareDocument>(
                Routing::Unscoped,
                ModelingPrepareDocumentRequested {
                    bytes: bytes.to_vec(),
                    format: ModelingDocumentFormat::StarUml,
                },
            )
            .await?,
    )?
    .document)
}

fn style(document: &ProjectDocument) -> FlowResult<Style> {
    let section = document
        .section(MODELING_NAMESPACE)
        .ok_or("missing MDE section")?;
    let view = section["diagrams"]
        .as_array()
        .ok_or("missing diagrams")?
        .iter()
        .flat_map(|diagram| diagram["views"].as_array().into_iter().flatten())
        .find(|view| view["id"] == "class-view")
        .ok_or("missing View")?;
    Ok(serde_json::from_value(view["style"].clone())?)
}

async fn edit(
    bridge: &MdeUiBridge,
    document: &Arc<ProjectDocument>,
    style: Style,
) -> FlowResult<Result<u64, String>> {
    let (completion, mut result) = tokio::sync::watch::channel(None);
    if let Err(error) = bridge.session().submit(MdeUiCommand::SetViewStyleTracked {
        request: ModelingSetViewStyleRequested {
            project_id: document.id().clone(),
            expected_revision: document.revision(),
            diagram_id: "diagram".into(),
            view_id: "class-view".into(),
            style,
        },
        completion,
    }) {
        return Ok(Err(error));
    }
    loop {
        if let Some(result) = result.borrow_and_update().clone() {
            return Ok(result);
        }
        result.changed().await?;
    }
}

async fn export(
    context: &Context,
    document: &Arc<ProjectDocument>,
) -> FlowResult<bellatrix_mde_cordis::MdjExportReceipt> {
    answer(
        context
            .query::<ModelingExportMdj>(
                Routing::Unscoped,
                ModelingExportMdjRequested {
                    document: Arc::clone(document),
                },
            )
            .await?,
    )
}

async fn reexport(bytes: &[u8]) -> FlowResult<Vec<u8>> {
    let (context, _service, provider, plugin) = start().await?;
    let reopened = answer(
        context
            .query::<ProjectImport>(
                Routing::Unscoped,
                ProjectImportRequested {
                    document: document(&context, bytes).await?,
                    is_saved: true,
                },
            )
            .await?,
    )?;
    let exported = export(&context, &reopened).await?.bytes().to_vec();
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(exported)
}

#[tokio::test]
async fn native_edits_save_natively_and_experimental_export_preserves_dirty_state() -> FlowResult<()>
{
    assert!(SPECIFICATION.contains(
        "Scenario: Edit the native document and keep experimental MDJ export separate from saving"
    ));
    let saved = SavedFlow::new("receipts")?;
    let source_path = saved.0.join("source.mdj");
    fs::write(&source_path, SOURCE)?;
    let (context, service, provider, plugin) = start().await?;
    let opened = answer(
        context
            .query::<ProjectImport>(
                Routing::Unscoped,
                ProjectImportRequested {
                    document: document(&context, SOURCE).await?,
                    is_saved: false,
                },
            )
            .await?,
    )?;
    let bridge = MdeUiBridge::start(&context, opened.id().clone())?;
    let id = opened.id().clone();
    let changes = Arc::new(AtomicUsize::new(0));
    let saved_events = Arc::new(AtomicUsize::new(0));
    let changes_observer = Arc::clone(&changes);
    let saved_observer = Arc::clone(&saved_events);
    let changed_listener = context.on::<ProjectStateChangedEvent, _>(observer_sync(
        move |_, _: ProjectStateChanged| {
            changes_observer.fetch_add(1, Ordering::SeqCst);
            Ok::<_, Infallible>(())
        },
    ))?;
    let saved_listener =
        context.on::<ProjectSavedEvent, _>(observer_sync(move |_, _: ProjectSaved| {
            saved_observer.fetch_add(1, Ordering::SeqCst);
            Ok::<_, Infallible>(())
        }))?;
    assert_eq!(export(&context, &opened).await?.bytes(), SOURCE);
    assert!(service.is_dirty(&id)?);
    assert_eq!(edit(&bridge, &opened, style(&opened)?).await??, 0);
    assert!(Arc::ptr_eq(&opened, &service.snapshot(&id)?));
    assert_eq!(changes.load(Ordering::SeqCst), 0);
    let mut native_style = style(&opened)?;
    native_style.opacity = Some(0.5);
    assert_eq!(edit(&bridge, &opened, native_style).await??, 1);
    let native_edit = service.snapshot(&id)?;
    assert!(export(&context, &native_edit).await.is_err());
    assert!(service.is_dirty(&id)?);
    let mut supported = style(&opened)?;
    supported.fill = Some("#ABCDEF".into());
    assert_eq!(edit(&bridge, &native_edit, supported).await??, 2);
    let edited = service.snapshot(&id)?;
    assert_eq!(changes.load(Ordering::SeqCst), 2);
    assert_eq!(edit(&bridge, &edited, style(&edited)?).await??, 2);
    assert!(Arc::ptr_eq(&edited, &service.snapshot(&id)?));
    assert_eq!(changes.load(Ordering::SeqCst), 2);
    let exported = export(&context, &edited).await?;
    assert!(exported.matches_snapshot(&edited));
    assert_eq!(
        style(&document(&context, exported.bytes()).await?)?,
        style(&edited)?
    );
    let destination = saved.0.join("edited.mdj");
    let ticket = answer(
        context
            .query::<ProjectPrepareEncodedSave>(
                Routing::Unscoped,
                exported.prepare_save(destination.clone()),
            )
            .await?,
    )?;
    assert!(service.is_dirty(&id)?);
    assert_eq!(saved_events.load(Ordering::SeqCst), 0);
    assert!(matches!(
        context
            .query::<ProjectPrepareEncodedSave>(
                Routing::Unscoped,
                exported.prepare_save(destination.clone())
            )
            .await?,
        QueryOutcome::Answer(Err(ProjectError::SaveInProgress))
    ));
    drop(ticket);
    assert!(service.is_dirty(&id)?);
    let ticket = answer(
        context
            .query::<ProjectPrepareEncodedSave>(
                Routing::Unscoped,
                exported.prepare_save(destination.clone()),
            )
            .await?,
    )?;
    let published = answer(
        context
            .query::<ProjectPublishNewFile>(
                Routing::Unscoped,
                ProjectPublishNewFileRequested { ticket },
            )
            .await?,
    )?;
    drop(published);
    let reopened = fs::read(&destination)?;
    assert_eq!(reexport(&reopened).await?, reopened);
    assert!(service.is_dirty(&id)?);
    assert_eq!(saved_events.load(Ordering::SeqCst), 0);
    assert_eq!(fs::read(&source_path)?, SOURCE);
    let ticket = answer(
        context
            .query::<ProjectPrepareEncodedSave>(
                Routing::Unscoped,
                exported.prepare_save(destination.clone()),
            )
            .await?,
    )?;
    assert!(matches!(
        context
            .query::<ProjectPublishNewFile>(
                Routing::Unscoped,
                ProjectPublishNewFileRequested { ticket }
            )
            .await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert_eq!(fs::read(&destination)?, reopened);
    assert!(service.is_dirty(&id)?);
    let native_path = saved.0.join("edited.bellatrix.json");
    let baseline = serde_json::to_vec_pretty(opened.as_ref())?;
    let external = serde_json::to_vec_pretty(native_edit.as_ref())?;
    fs::write(&native_path, &external)?;
    let conflicted = context
        .query::<ProjectSave>(
            Routing::Unscoped,
            ProjectSaveRequested {
                expected_bytes: Some(baseline.into()),
                document: Arc::clone(&edited),
                path: native_path.clone(),
            },
        )
        .await?;
    assert!(matches!(
        conflicted,
        QueryOutcome::Answer(Err(ProjectError::DestinationChanged(_)))
    ));
    assert_eq!(fs::read(&native_path)?, external);
    assert!(service.is_dirty(&id)?);
    assert_eq!(saved_events.load(Ordering::SeqCst), 0);
    for entry in fs::read_dir(&saved.0)? {
        assert!(
            !entry?
                .file_name()
                .to_string_lossy()
                .starts_with(".bellatrix-")
        );
    }
    assert_eq!(
        answer(
            context
                .query::<ProjectSave>(
                    Routing::Unscoped,
                    ProjectSaveRequested {
                        expected_bytes: Some(external.into()),
                        document: Arc::clone(&edited),
                        path: native_path.clone()
                    }
                )
                .await?
        )?,
        2
    );
    let native: ProjectDocument = serde_json::from_slice(&fs::read(native_path)?)?;
    assert_eq!(&native, edited.as_ref());
    assert!(!service.is_dirty(&id)?);
    assert_eq!(saved_events.load(Ordering::SeqCst), 1);
    changed_listener.remove();
    saved_listener.remove();
    drop(bridge);
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}

#[tokio::test]
async fn native_session_preserves_newer_edits_and_rejects_closed_session_receipts() -> FlowResult<()>
{
    assert!(SPECIFICATION.contains(
        "Scenario: Preserve newer edits and reject receipts from a closed document session"
    ));
    let saved = SavedFlow::new("sessions")?;
    let (context, service, provider, plugin) = start().await?;
    let original = document(&context, SOURCE).await?;
    let opened = answer(
        context
            .query::<ProjectImport>(
                Routing::Unscoped,
                ProjectImportRequested {
                    document: original.clone(),
                    is_saved: true,
                },
            )
            .await?,
    )?;
    let id = opened.id().clone();
    let bridge = MdeUiBridge::start(&context, id.clone())?;
    let receipt = export(&context, &opened).await?;
    let ticket = answer(
        context
            .query::<ProjectPrepareEncodedSave>(
                Routing::Unscoped,
                receipt.prepare_save(saved.0.join("older.mdj")),
            )
            .await?,
    )?;
    let mut changed = style(&opened)?;
    changed.fill = Some("#ABCDEF".into());
    assert_eq!(edit(&bridge, &opened, changed).await??, 1);
    assert!(matches!(
        context
            .query::<ProjectPublishNewFile>(
                Routing::Unscoped,
                ProjectPublishNewFileRequested { ticket }
            )
            .await?,
        QueryOutcome::Answer(Err(ProjectError::Conflict {
            expected: 0,
            current: 1
        }))
    ));
    assert!(service.is_dirty(&id)?);
    assert_eq!(
        style(service.snapshot(&id)?.as_ref())?.fill.as_deref(),
        Some("#ABCDEF")
    );
    answer(
        context
            .query::<ProjectClose>(
                Routing::Unscoped,
                ProjectCloseRequested {
                    id: id.clone(),
                    discard_unsaved: true,
                },
            )
            .await?,
    )?;
    let reopened = answer(
        context
            .query::<ProjectImport>(
                Routing::Unscoped,
                ProjectImportRequested {
                    document: original,
                    is_saved: true,
                },
            )
            .await?,
    )?;
    assert!(!receipt.matches_snapshot(&reopened));
    assert!(matches!(
        context
            .query::<ProjectPrepareEncodedSave>(
                Routing::Unscoped,
                receipt.prepare_save(saved.0.join("stale.mdj"))
            )
            .await?,
        QueryOutcome::Answer(Err(ProjectError::SessionChanged(_)))
    ));
    assert!(matches!(
        context
            .query::<ProjectReplaceSnapshot>(
                Routing::Unscoped,
                ProjectReplaceSnapshotRequested {
                    document: Arc::clone(&opened),
                    namespace: MODELING_NAMESPACE.into(),
                    state: json!({})
                }
            )
            .await?,
        QueryOutcome::Answer(Err(ProjectError::SessionChanged(_)))
    ));
    assert!(Arc::ptr_eq(&reopened, &service.snapshot(&id)?));
    assert!(export(&context, &opened).await.is_err());
    assert!(matches!(
        edit(&bridge, &opened, style(&opened)?).await?,
        Err(_)
    ));
    assert!(Arc::ptr_eq(&reopened, &service.snapshot(&id)?));
    assert!(!service.is_dirty(&id)?);
    assert_eq!(export(&context, &reopened).await?.bytes(), SOURCE);
    drop(bridge);
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}

#[tokio::test]
async fn mdj_interchange_rejects_extra_sections_without_restricting_native_documents()
-> FlowResult<()> {
    assert!(SPECIFICATION.contains(
        "Scenario: Preserve complete native authoring when experimental MDJ export is unavailable"
    ));
    let saved = SavedFlow::new("native-sections")?;
    let (context, service, provider, plugin) = start().await?;
    let document = document(&context, SOURCE)
        .await?
        .with_section("example.extra", json!({"future": [null, true]}))?;
    let opened = answer(
        context
            .query::<ProjectImport>(
                Routing::Unscoped,
                ProjectImportRequested {
                    document,
                    is_saved: true,
                },
            )
            .await?,
    )?;
    let id = opened.id().clone();
    let bridge = MdeUiBridge::start(&context, id.clone())?;
    assert!(export(&context, &opened).await.is_err());
    assert_eq!(edit(&bridge, &opened, style(&opened)?).await??, 0);
    assert!(Arc::ptr_eq(&opened, &service.snapshot(&id)?));
    let mut native_style = style(&opened)?;
    native_style.opacity = Some(0.5);
    assert_eq!(
        answer(
            context
                .query::<bellatrix_mde_cordis::ModelingSetViewStyle>(
                    Routing::Unscoped,
                    bellatrix_mde_cordis::ModelingSetViewStyleRequested {
                        project_id: id.clone(),
                        expected_revision: opened.revision(),
                        diagram_id: "diagram".into(),
                        view_id: "class-view".into(),
                        style: native_style,
                    },
                )
                .await?
        )?,
        1
    );
    let edited = service.snapshot(&id)?;
    assert!(service.is_dirty(&id)?);
    let path = saved.0.join("complete-native.json");
    assert_eq!(
        answer(
            context
                .query::<ProjectSave>(
                    Routing::Unscoped,
                    ProjectSaveRequested {
                        expected_bytes: None,
                        document: Arc::clone(&edited),
                        path: path.clone()
                    }
                )
                .await?
        )?,
        1
    );
    let native: ProjectDocument = serde_json::from_slice(&fs::read(path)?)?;
    assert_eq!(&native, &*edited);
    assert_eq!(style(&native)?.opacity, Some(0.5));
    assert_eq!(
        native.section("example.extra"),
        Some(&json!({"future": [null, true]}))
    );
    assert!(!service.is_dirty(&id)?);
    drop(bridge);
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}

async fn dispatch_authoring(
    context: &Context,
    bridge: &MdeUiBridge,
    through_ui: bool,
    command: MdeUiCommand,
) -> FlowResult<()> {
    if through_ui {
        let session = bridge.session();
        let mut activity = session.activity();
        session.submit(command)?;
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while session.is_pending() {
                activity.changed().await?;
            }
            Ok::<_, Box<dyn Error>>(())
        })
        .await??;
        return Ok(());
    }
    use bellatrix_mde_cordis::*;
    match command {
        MdeUiCommand::CreateElement(request) => {
            answer(
                context
                    .query::<ModelingCreateElement>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::CreateElementOrVisualDraft(request) => {
            answer(
                context
                    .query::<ModelingCreateElementOrVisualDraft>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::CreateModelElement(request) => {
            answer(
                context
                    .query::<ModelingCreateModelElement>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::CreateRelation(request) => {
            answer(
                context
                    .query::<ModelingCreateRelation>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::CreateRelationOrVisualDraft(request) => {
            answer(
                context
                    .query::<ModelingCreateRelationOrVisualDraft>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::CreateRelationAtViewsOrVisualDraft(request) => {
            answer(
                context
                    .query::<ModelingCreateRelationAtViewsOrVisualDraft>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::QuickEdit(request) => {
            answer(
                context
                    .query::<ModelingQuickEdit>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::SetProperty(request) => {
            answer(
                context
                    .query::<ModelingSetProperty>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::AddMember(request) => {
            answer(
                context
                    .query::<ModelingAddMember>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        MdeUiCommand::MoveParameter(request) => {
            answer(
                context
                    .query::<ModelingMoveParameter>(Routing::Unscoped, request)
                    .await?,
            )?;
        }
        _ => return Err("unsupported authoring scenario command".into()),
    }
    Ok(())
}

#[tokio::test]
async fn class_authoring_matches_between_cordis_events_and_ui_session() -> FlowResult<()> {
    use bellatrix_mde_cordis::*;
    assert!(SPECIFICATION.contains(
        "Scenario: Create identical classes and relationships through events and the UI session"
    ));
    let mut snapshots = Vec::new();
    for through_ui in [false, true] {
        let (context, service, provider, plugin) = start().await?;
        let prepared = answer(
            context
                .query::<ModelingPrepareDocument>(
                    Routing::Unscoped,
                    ModelingPrepareDocumentRequested {
                        bytes: include_bytes!("../../bellatrix_mde/fixtures/blank.json").to_vec(),
                        format: ModelingDocumentFormat::ModelJson,
                    },
                )
                .await?,
        )?;
        let opened = answer(
            context
                .query::<ProjectImport>(
                    Routing::Unscoped,
                    ProjectImportRequested {
                        document: prepared.document,
                        is_saved: false,
                    },
                )
                .await?,
        )?;
        let id = opened.id().clone();
        let bridge = MdeUiBridge::start(&context, id.clone())?;
        let first = ModelingCreateElementRequested {
            project_id: id.clone(),
            diagram_id: "classes".into(),
            owner_id: None,
            kind: ModelingElementKind::Class,
            name: Some("Invoice".into()),
            x: 100.0,
            y: 100.0,
            expected_revision: 0,
        };
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::CreateElement(first.clone()),
        )
        .await?;
        let second = ModelingCreateElementRequested {
            name: Some("Document".into()),
            x: 400.0,
            expected_revision: 1,
            ..first.clone()
        };
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::CreateElementOrVisualDraft(second),
        )
        .await?;
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::CreateModelElement(ModelingCreateModelElementRequested {
                project_id: id.clone(),
                owner_id: Some("model".into()),
                kind: ModelingElementKind::Interface,
                name: Some("Exportable".into()),
                expected_revision: 2,
            }),
        )
        .await?;
        let document = service.snapshot(&id)?;
        assert_eq!(document.revision(), 3);
        let projection = answer(
            context
                .query::<ModelingReadProjection>(
                    Routing::Unscoped,
                    ModelingReadProjectionRequested {
                        document: Arc::clone(&document),
                    },
                )
                .await?,
        )?
        .section;
        let invoice = projection
            .elements
            .iter()
            .find(|element| element.name == "Invoice")
            .ok_or("created Invoice missing")?
            .id
            .clone();
        let target = projection
            .elements
            .iter()
            .find(|element| element.name == "Document")
            .ok_or("created Document missing")?
            .id
            .clone();
        let view_id = |subject: &str| -> FlowResult<String> {
            projection
                .diagrams
                .iter()
                .flat_map(|diagram| diagram.all_views())
                .find(|view| view.subject.as_deref() == Some(subject))
                .map(|view| view.id.clone())
                .ok_or_else(|| "created Class View missing".into())
        };
        let relationship = ModelingCreateRelationRequested {
            project_id: id.clone(),
            diagram_id: "classes".into(),
            owner_id: Some("model".into()),
            kind: ModelingRelationKind::Association,
            from: invoice.clone(),
            to: target.clone(),
            expected_revision: 3,
        };
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::CreateRelation(relationship.clone()),
        )
        .await?;
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::CreateRelationOrVisualDraft(ModelingCreateRelationRequested {
                kind: ModelingRelationKind::Generalization,
                owner_id: Some(invoice.clone()),
                expected_revision: 4,
                ..relationship
            }),
        )
        .await?;
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::CreateRelationAtViewsOrVisualDraft(
                ModelingCreateRelationAtViewsRequested {
                    project_id: id.clone(),
                    diagram_id: "classes".into(),
                    owner_id: Some("model".into()),
                    kind: ModelingRelationKind::Dependency,
                    from: Some(invoice.clone()),
                    to: Some(target.clone()),
                    source_view_id: view_id(&invoice)?,
                    target_view_id: view_id(&target)?,
                    expected_revision: 5,
                },
            ),
        )
        .await?;
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::SetProperty(ModelingSetPropertyRequested {
                project_id: id.clone(),
                element_id: invoice.clone(),
                path: "name".into(),
                value: json!("Invoice edited"),
                expected_revision: 6,
            }),
        )
        .await?;
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::AddMember(ModelingAddMemberRequested {
                project_id: id.clone(),
                owner_id: invoice.clone(),
                kind: ModelingMemberKind::Operation,
                expected_revision: 7,
            }),
        )
        .await?;
        let operation_section: bellatrix_mde_contracts::ModelingSection = serde_json::from_value(
            service
                .snapshot(&id)?
                .section(MODELING_NAMESPACE)
                .ok_or("missing operation section")?
                .clone(),
        )?;
        let operation_id = operation_section
            .registry
            .entries()
            .values()
            .find(|entry| entry.type_name() == "Operation")
            .ok_or("added Operation missing")?
            .id
            .clone();
        for expected_revision in [8, 9] {
            dispatch_authoring(
                &context,
                &bridge,
                through_ui,
                MdeUiCommand::AddMember(ModelingAddMemberRequested {
                    project_id: id.clone(),
                    owner_id: operation_id.clone(),
                    kind: ModelingMemberKind::Parameter,
                    expected_revision,
                }),
            )
            .await?;
        }
        let parameter_section: bellatrix_mde_contracts::ModelingSection = serde_json::from_value(
            service
                .snapshot(&id)?
                .section(MODELING_NAMESPACE)
                .ok_or("missing parameter section")?
                .clone(),
        )?;
        let Some(bellatrix_mde_contracts::ruml::Contract::Operation(operation)) = parameter_section
            .registry
            .get_entry(&operation_id)
            .map(|entry| &entry.contract)
        else {
            return Err("parameter owner must remain an Operation".into());
        };
        let [first_parameter, second_parameter] = operation.owned_parameter.as_slice() else {
            return Err("Operation must own two Parameters".into());
        };
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::MoveParameter(ModelingMoveParameterRequested {
                project_id: id.clone(),
                parameter_id: second_parameter.clone(),
                up: true,
                expected_revision: 10,
            }),
        )
        .await?;
        dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::QuickEdit(ModelingQuickEditRequested {
                project_id: id.clone(),
                diagram_id: "classes".into(),
                element_id: invoice.clone(),
                view_id: Some(view_id(&invoice)?),
                action: UmlQuickCreate::Note,
                expected_revision: 11,
            }),
        )
        .await?;
        let authored = service.snapshot(&id)?;
        assert_eq!(authored.revision(), 12);
        let section = authored
            .section(MODELING_NAMESPACE)
            .ok_or("missing authored section")?;
        let typed: bellatrix_mde_contracts::ModelingSection =
            serde_json::from_value(section.clone())?;
        for (metaclass, expected) in [
            ("Class", 2),
            ("Interface", 1),
            ("Association", 1),
            ("Generalization", 1),
            ("Dependency", 1),
            ("Operation", 1),
            ("Parameter", 2),
            ("Comment", 1),
        ] {
            assert_eq!(
                typed
                    .registry
                    .entries()
                    .values()
                    .filter(|entry| entry.type_name() == metaclass)
                    .count(),
                expected
            );
        }
        assert!(
            typed
                .diagrams
                .iter()
                .flat_map(|diagram| diagram.all_views())
                .all(|view| view.draft.is_none())
        );
        let Some(bellatrix_mde_contracts::ruml::Contract::Operation(operation)) = typed
            .registry
            .get_entry(&operation_id)
            .map(|entry| &entry.contract)
        else {
            return Err("edited Operation missing".into());
        };
        assert_eq!(
            operation.owned_parameter,
            vec![second_parameter.clone(), first_parameter.clone()]
        );
        let Some(bellatrix_mde_contracts::ruml::Contract::Class(class)) = typed
            .registry
            .get_entry(&invoice)
            .map(|entry| &entry.contract)
        else {
            return Err("edited Class missing".into());
        };
        assert_eq!(class.name.as_deref(), Some("Invoice edited"));
        if through_ui {
            let (completion, receipt) = tokio::sync::watch::channel(None);
            dispatch_authoring(
                &context,
                &bridge,
                through_ui,
                MdeUiCommand::SetPropertyTracked {
                    request: ModelingSetPropertyRequested {
                        project_id: id.clone(),
                        element_id: invoice.clone(),
                        path: "name".into(),
                        value: json!("Invoice edited"),
                        expected_revision: 12,
                    },
                    completion,
                },
            )
            .await?;
            assert_eq!(*receipt.borrow(), Some(Ok(12)));
            assert!(Arc::ptr_eq(&authored, &service.snapshot(&id)?));
        }
        for format in ["json", "protobuf"] {
            let bytes = bellatrix_mde::encode_model_section(
                id.as_str(),
                authored.name(),
                &section.to_string(),
                format,
            )?;
            let reopened = bellatrix_mde::prepare_model_document(
                &bellatrix_mde::decode_model_document(&bytes, format)?,
            )?;
            let restored: bellatrix_mde_contracts::ModelingSection =
                serde_json::from_str(&reopened.section_json)?;
            assert_eq!(restored, typed);
        }
        let stale = dispatch_authoring(
            &context,
            &bridge,
            through_ui,
            MdeUiCommand::CreateElement(first),
        )
        .await;
        if through_ui {
            stale?;
        } else {
            assert!(stale.is_err());
        }
        assert!(Arc::ptr_eq(&authored, &service.snapshot(&id)?));
        snapshots.push(section.clone());
        drop(bridge);
        plugin.dispose().await?;
        provider.dispose().await?;
    }
    assert_eq!(snapshots.first(), snapshots.get(1));
    Ok(())
}
