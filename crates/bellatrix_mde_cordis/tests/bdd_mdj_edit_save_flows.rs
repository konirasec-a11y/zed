use bellatrix_mde::contracts::{ModelingSection, ruml::Contract};
use bellatrix_mde_cordis::{project::*, *};
use cordis::{
    Context, FiberHandle, Plugin, PreparedPlugin, Routing,
    event::{QueryOutcome, observer_sync},
};
use serde_json::json;
use std::{
    convert::Infallible,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const MDJ: &[u8] = include_bytes!("../../bellatrix_mde/fixtures/bdd/staruml/presentation.mdj");
const BLANK: &[u8] = include_bytes!("../../bellatrix_mde/fixtures/blank.json");
const REQUIREMENTS: &str = include_str!("features/mdj_edit_save_lifecycle.feature");

macro_rules! request {
    ($runtime:expr, $event:ty, $payload:expr) => {
        match $runtime
            .context
            .query::<$event>(Routing::Unscoped, $payload)
            .await?
        {
            QueryOutcome::Answer(result) => result,
            QueryOutcome::Miss => {
                return Err(concat!("No responder for ", stringify!($event)).into())
            }
        }
    };
}

macro_rules! author {
    ($runtime:expr, $bridge:expr, $ui:expr, $event:ty, $command:ident, $payload:expr) => {{
        let payload = $payload;
        if $ui {
            submit(&$bridge, MdeUiCommand::$command(payload)).await?;
        } else {
            request!($runtime, $event, payload)?;
        }
    }};
}

struct Runtime {
    context: Context,
    projects: Arc<ProjectService>,
    provider: FiberHandle,
    modeling: FiberHandle,
}

impl Runtime {
    async fn start(scenario: &str) -> Result<Self> {
        assert!(REQUIREMENTS.contains(&format!("Scenario: {scenario}")));
        let context = Context::new();
        let projects = Arc::new(ProjectService::new());
        let provider = context
            .spawn(PreparedPlugin::from_input(
                ProjectPlugin::new(projects.clone()),
                projects.clone(),
            ))
            .await?;
        let plugin = MdePlugin;
        plugin.prepare(())?;
        let modeling = context
            .spawn(PreparedPlugin::from_input(plugin, ()))
            .await?;
        Ok(Self {
            context,
            projects,
            provider,
            modeling,
        })
    }

    async fn prepare(
        &self,
        bytes: &[u8],
        format: ModelingDocumentFormat,
    ) -> Result<ProjectDocument> {
        Ok(request!(
            self,
            ModelingPrepareDocument,
            ModelingPrepareDocumentRequested {
                bytes: bytes.to_vec(),
                format,
            }
        )?
        .document)
    }

    async fn open(
        &self,
        document: ProjectDocument,
        is_saved: bool,
    ) -> Result<Arc<ProjectDocument>> {
        Ok(request!(
            self,
            ProjectImport,
            ProjectImportRequested { document, is_saved }
        )?)
    }

    async fn export(&self, document: &Arc<ProjectDocument>) -> Result<MdjExportReceipt> {
        Ok(request!(
            self,
            ModelingExportMdj,
            ModelingExportMdjRequested {
                document: document.clone(),
            }
        )?)
    }

    async fn reserve(&self, receipt: &MdjExportReceipt, path: &Path) -> Result<ProjectSaveTicket> {
        Ok(request!(
            self,
            ProjectPrepareEncodedSave,
            receipt.prepare_save(path.to_path_buf())
        )?)
    }

    async fn save(
        &self,
        document: &Arc<ProjectDocument>,
        path: &Path,
        baseline: Option<Vec<u8>>,
    ) -> Result<u64> {
        Ok(request!(
            self,
            ProjectSave,
            ProjectSaveRequested {
                document: document.clone(),
                path: path.to_path_buf(),
                expected_bytes: baseline.map(Into::into),
            }
        )?)
    }

    fn unchanged(&self, snapshot: &Arc<ProjectDocument>, dirty: bool) -> Result {
        assert!(Arc::ptr_eq(
            snapshot,
            &self.projects.snapshot(snapshot.id())?
        ));
        assert_eq!(self.projects.is_dirty(snapshot.id())?, dirty);
        Ok(())
    }

    async fn stop(self) -> Result {
        self.modeling.dispose().await?;
        self.provider.dispose().await?;
        Ok(())
    }
}

struct Files(PathBuf);
impl Files {
    fn new() -> Result<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "cordis-mdj-bdd-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("Cannot remove {}: {error}", self.0.display());
        }
    }
}

fn section(document: &ProjectDocument) -> Result<ModelingSection> {
    Ok(serde_json::from_value(
        document
            .section(MODELING_NAMESPACE)
            .ok_or("Missing canonical modeling section")?
            .clone(),
    )?)
}

fn view_style(document: &ProjectDocument) -> Result<Style> {
    section(document)?
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == "class-view")
        .map(|view| view.style.clone())
        .ok_or_else(|| "Missing imported Class View".into())
}

async fn submit(bridge: &MdeUiBridge, command: MdeUiCommand) -> Result {
    let session = bridge.session();
    let mut activity = session.activity();
    session.submit(command)?;
    tokio::time::timeout(Duration::from_secs(30), async {
        while session.is_pending() {
            activity.changed().await?;
        }
        Ok::<_, Box<dyn Error>>(())
    })
    .await??;
    Ok(())
}

async fn set_style(
    bridge: &MdeUiBridge,
    snapshot: &Arc<ProjectDocument>,
    style: Style,
) -> Result<std::result::Result<u64, String>> {
    let (completion, mut receipt) = tokio::sync::watch::channel(None);
    if let Err(error) = bridge.session().submit(MdeUiCommand::SetViewStyleTracked {
        request: ModelingSetViewStyleRequested {
            project_id: snapshot.id().clone(),
            expected_revision: snapshot.revision(),
            diagram_id: "diagram".into(),
            view_id: "class-view".into(),
            style,
        },
        completion,
    }) {
        return Ok(Err(error));
    }
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(result) = receipt.borrow_and_update().clone() {
                return Ok::<_, Box<dyn Error>>(result);
            }
            receipt.changed().await?;
        }
    })
    .await?
}

#[tokio::test]
async fn experimental_export_and_native_save_have_separate_lifecycles() -> Result {
    let runtime = Runtime::start(
        "Edit the native document and keep experimental MDJ export separate from saving",
    )
    .await?;
    let files = Files::new()?;
    let source = files.path("source.mdj");
    fs::write(&source, MDJ)?;
    let opened = runtime
        .open(
            runtime
                .prepare(MDJ, ModelingDocumentFormat::StarUml)
                .await?,
            false,
        )
        .await?;
    let bridge = MdeUiBridge::start(&runtime.context, opened.id().clone())?;
    let changes = Arc::new(AtomicUsize::new(0));
    let saves = Arc::new(AtomicUsize::new(0));
    let changed_listener = runtime
        .context
        .on::<ProjectStateChangedEvent, _>(observer_sync({
            let changes = changes.clone();
            move |_, _: ProjectStateChanged| {
                changes.fetch_add(1, Ordering::SeqCst);
                Ok::<_, Infallible>(())
            }
        }))?;
    let saved_listener = runtime.context.on::<ProjectSavedEvent, _>(observer_sync({
        let saves = saves.clone();
        move |_, _: ProjectSaved| {
            saves.fetch_add(1, Ordering::SeqCst);
            Ok::<_, Infallible>(())
        }
    }))?;
    assert_eq!(runtime.export(&opened).await?.bytes(), MDJ);
    assert_eq!(set_style(&bridge, &opened, view_style(&opened)?).await??, 0);
    runtime.unchanged(&opened, true)?;
    assert_eq!(changes.load(Ordering::SeqCst), 0);

    let mut unsupported = view_style(&opened)?;
    unsupported.opacity = Some(0.5);
    assert_eq!(set_style(&bridge, &opened, unsupported).await??, 1);
    let native_edit = runtime.projects.snapshot(opened.id())?;
    assert_eq!(view_style(&native_edit)?.opacity, Some(0.5));
    let Err(error) = runtime.export(&native_edit).await else {
        return Err("An unrepresentable native style was silently exported".into());
    };
    assert!(error.to_string().contains("cannot represent"));
    runtime.unchanged(&native_edit, true)?;
    assert_eq!(
        set_style(&bridge, &native_edit, view_style(&native_edit)?).await??,
        1
    );
    runtime.unchanged(&native_edit, true)?;
    assert_eq!(changes.load(Ordering::SeqCst), 1);

    let mut supported = view_style(&opened)?;
    supported.fill = Some("#ABCDEF".into());
    assert_eq!(set_style(&bridge, &native_edit, supported).await??, 2);
    let edited = runtime.projects.snapshot(opened.id())?;
    assert_eq!(set_style(&bridge, &edited, view_style(&edited)?).await??, 2);
    runtime.unchanged(&edited, true)?;
    assert_eq!(changes.load(Ordering::SeqCst), 2);
    let receipt = runtime.export(&edited).await?;
    assert!(receipt.matches_snapshot(&edited));
    let converted = runtime
        .prepare(receipt.bytes(), ModelingDocumentFormat::StarUml)
        .await?;
    assert_eq!(view_style(&converted)?, view_style(&edited)?);
    let destination = files.path("export.mdj");
    let reservation = runtime.reserve(&receipt, &destination).await?;
    assert!(matches!(
        request!(
            runtime,
            ProjectPrepareEncodedSave,
            receipt.prepare_save(destination.clone())
        ),
        Err(ProjectError::SaveInProgress)
    ));
    runtime.unchanged(&edited, true)?;
    assert_eq!(saves.load(Ordering::SeqCst), 0);
    drop(reservation);
    assert!(!destination.exists());
    runtime.unchanged(&edited, true)?;
    assert_eq!(saves.load(Ordering::SeqCst), 0);
    let ticket = runtime.reserve(&receipt, &destination).await?;
    drop(request!(
        runtime,
        ProjectPublishNewFile,
        ProjectPublishNewFileRequested { ticket }
    )?);
    let published = fs::read(&destination)?;
    assert_eq!(published, receipt.bytes());
    let reader = Runtime::start(
        "Edit the native document and keep experimental MDJ export separate from saving",
    )
    .await?;
    let reopened = reader
        .open(
            reader
                .prepare(&published, ModelingDocumentFormat::StarUml)
                .await?,
            true,
        )
        .await?;
    assert_eq!(reader.export(&reopened).await?.bytes(), published);
    reader.stop().await?;
    runtime.unchanged(&edited, true)?;
    assert_eq!(fs::read(&source)?, MDJ);
    assert_eq!(saves.load(Ordering::SeqCst), 0);
    let ticket = runtime.reserve(&receipt, &destination).await?;
    assert!(
        request!(
            runtime,
            ProjectPublishNewFile,
            ProjectPublishNewFileRequested { ticket }
        )
        .is_err()
    );
    assert_eq!(fs::read(&destination)?, published);
    runtime.unchanged(&edited, true)?;

    let native_path = files.path("native.json");
    let baseline = serde_json::to_vec_pretty(opened.as_ref())?;
    fs::write(&native_path, &baseline)?;
    let external = serde_json::to_vec_pretty(native_edit.as_ref())?;
    fs::write(&native_path, &external)?;
    assert!(matches!(
        request!(
            runtime,
            ProjectSave,
            ProjectSaveRequested {
                document: edited.clone(),
                path: native_path.clone(),
                expected_bytes: Some(baseline.into()),
            }
        ),
        Err(ProjectError::DestinationChanged(_))
    ));
    assert_eq!(fs::read(&native_path)?, external);
    runtime.unchanged(&edited, true)?;
    assert_eq!(saves.load(Ordering::SeqCst), 0);
    for entry in fs::read_dir(&files.0)? {
        assert!(
            !entry?
                .file_name()
                .to_string_lossy()
                .starts_with(".bellatrix-")
        );
    }
    assert_eq!(
        runtime.save(&edited, &native_path, Some(external)).await?,
        2
    );
    let persisted: ProjectDocument = serde_json::from_slice(&fs::read(native_path)?)?;
    assert_eq!(&persisted, edited.as_ref());
    runtime.unchanged(&edited, false)?;
    assert_eq!(saves.load(Ordering::SeqCst), 1);
    assert_eq!(changes.load(Ordering::SeqCst), 2);
    changed_listener.remove();
    saved_listener.remove();
    drop(bridge);
    runtime.stop().await
}

#[tokio::test]
async fn receipts_and_commands_cannot_cross_revisions_or_sessions() -> Result {
    let runtime =
        Runtime::start("Preserve newer edits and reject receipts from a closed document session")
            .await?;
    let files = Files::new()?;
    let original = runtime
        .prepare(MDJ, ModelingDocumentFormat::StarUml)
        .await?;
    let opened = runtime.open(original.clone(), true).await?;
    let bridge = MdeUiBridge::start(&runtime.context, opened.id().clone())?;
    let receipt = runtime.export(&opened).await?;
    let destination = files.path("older.mdj");
    let ticket = runtime.reserve(&receipt, &destination).await?;
    let mut style = view_style(&opened)?;
    style.fill = Some("#ABCDEF".into());
    assert_eq!(set_style(&bridge, &opened, style).await??, 1);
    let newer = runtime.projects.snapshot(opened.id())?;
    assert!(matches!(
        request!(
            runtime,
            ProjectPublishNewFile,
            ProjectPublishNewFileRequested { ticket }
        ),
        Err(ProjectError::Conflict {
            expected: 0,
            current: 1
        })
    ));
    assert!(!destination.exists());
    runtime.unchanged(&newer, true)?;
    assert_eq!(view_style(&newer)?.fill.as_deref(), Some("#ABCDEF"));
    request!(
        runtime,
        ProjectClose,
        ProjectCloseRequested {
            id: opened.id().clone(),
            discard_unsaved: true,
        }
    )?;
    let reopened = runtime.open(original, true).await?;
    assert_eq!(reopened.id(), opened.id());
    assert_eq!(reopened.revision(), opened.revision());
    assert!(!receipt.matches_snapshot(&reopened));
    assert!(matches!(
        request!(
            runtime,
            ProjectPrepareEncodedSave,
            receipt.prepare_save(files.path("stale.mdj"))
        ),
        Err(ProjectError::SessionChanged(_))
    ));
    assert!(matches!(
        request!(
            runtime,
            ProjectReplaceSnapshot,
            ProjectReplaceSnapshotRequested {
                document: opened.clone(),
                namespace: MODELING_NAMESPACE.into(),
                state: json!({}),
            }
        ),
        Err(ProjectError::SessionChanged(_))
    ));
    runtime.unchanged(&reopened, false)?;
    assert!(runtime.export(&opened).await.is_err());
    assert!(
        set_style(&bridge, &opened, view_style(&opened)?)
            .await?
            .is_err()
    );
    runtime.unchanged(&reopened, false)?;
    assert!(!files.path("stale.mdj").exists());
    assert_eq!(runtime.export(&reopened).await?.bytes(), MDJ);
    drop(bridge);
    runtime.stop().await
}

#[tokio::test]
async fn native_sections_survive_authoring_when_mdj_export_is_unsupported() -> Result {
    let runtime = Runtime::start(
        "Preserve complete native authoring when experimental MDJ export is unavailable",
    )
    .await?;
    let files = Files::new()?;
    let extra = json!({"future": [null, true]});
    let document = runtime
        .prepare(MDJ, ModelingDocumentFormat::StarUml)
        .await?
        .with_section("example.extra", extra.clone())?;
    let opened = runtime.open(document, true).await?;
    let bridge = MdeUiBridge::start(&runtime.context, opened.id().clone())?;
    let rejected = runtime.export(&opened).await;
    let Err(error) = rejected else {
        return Err("Extra native section was silently exported".into());
    };
    assert!(error.to_string().contains("other platform sections"));
    runtime.unchanged(&opened, false)?;
    assert_eq!(set_style(&bridge, &opened, view_style(&opened)?).await??, 0);
    runtime.unchanged(&opened, false)?;
    let mut style = view_style(&opened)?;
    style.opacity = Some(0.5);
    assert_eq!(
        request!(
            runtime,
            ModelingSetViewStyle,
            ModelingSetViewStyleRequested {
                project_id: opened.id().clone(),
                expected_revision: 0,
                diagram_id: "diagram".into(),
                view_id: "class-view".into(),
                style,
            }
        )?,
        1
    );
    let edited = runtime.projects.snapshot(opened.id())?;
    runtime.unchanged(&edited, true)?;
    let path = files.path("complete-native.json");
    assert_eq!(runtime.save(&edited, &path, None).await?, 1);
    let persisted: ProjectDocument = serde_json::from_slice(&fs::read(path)?)?;
    assert_eq!(&persisted, edited.as_ref());
    assert_eq!(view_style(&persisted)?.opacity, Some(0.5));
    assert_eq!(persisted.section("example.extra"), Some(&extra));
    runtime.unchanged(&edited, false)?;
    drop(bridge);
    runtime.stop().await
}

fn identity(section: &ModelingSection, metaclass: &str) -> Result<String> {
    section
        .registry
        .entries()
        .values()
        .find(|entry| entry.type_name() == metaclass)
        .map(|entry| entry.id.clone())
        .ok_or_else(|| format!("Missing {metaclass}").into())
}

#[tokio::test]
async fn event_and_ui_authoring_produce_the_same_canonical_document() -> Result {
    let mut outcomes = Vec::new();
    for through_ui in [false, true] {
        let runtime = Runtime::start(
            "Create identical classes and relationships through events and the UI session",
        )
        .await?;
        let prepared = runtime
            .prepare(BLANK, ModelingDocumentFormat::ModelJson)
            .await?;
        let opened = runtime.open(prepared, false).await?;
        let id = opened.id().clone();
        let bridge = MdeUiBridge::start(&runtime.context, id.clone())?;
        let first_request = ModelingCreateElementRequested {
            project_id: id.clone(),
            diagram_id: "classes".into(),
            owner_id: None,
            kind: ModelingElementKind::Class,
            name: Some("Invoice".into()),
            x: 100.0,
            y: 100.0,
            expected_revision: 0,
        };
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingCreateElement,
            CreateElement,
            first_request.clone()
        );
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingCreateElementOrVisualDraft,
            CreateElementOrVisualDraft,
            ModelingCreateElementRequested {
                name: Some("Document".into()),
                x: 400.0,
                expected_revision: 1,
                ..first_request.clone()
            }
        );
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingCreateModelElement,
            CreateModelElement,
            ModelingCreateModelElementRequested {
                project_id: id.clone(),
                owner_id: Some("model".into()),
                kind: ModelingElementKind::Interface,
                name: Some("Exportable".into()),
                expected_revision: 2,
            }
        );
        let classes = runtime.projects.snapshot(&id)?;
        assert_eq!(classes.revision(), 3);
        let projection = request!(
            runtime,
            ModelingReadProjection,
            ModelingReadProjectionRequested { document: classes }
        )?
        .section;
        let named = |name: &str| -> Result<String> {
            projection
                .elements
                .iter()
                .find(|element| element.name == name)
                .map(|element| element.id.clone())
                .ok_or_else(|| format!("Missing {name}").into())
        };
        let invoice = named("Invoice")?;
        let document = named("Document")?;
        let view_for = |subject: &str| -> Result<String> {
            projection
                .diagrams
                .iter()
                .flat_map(|diagram| diagram.all_views())
                .find(|view| view.subject.as_deref() == Some(subject))
                .map(|view| view.id.clone())
                .ok_or_else(|| "Missing Class View".into())
        };
        let invoice_view = view_for(&invoice)?;
        let document_view = view_for(&document)?;
        let association = ModelingCreateRelationRequested {
            project_id: id.clone(),
            diagram_id: "classes".into(),
            owner_id: Some("model".into()),
            kind: ModelingRelationKind::Association,
            from: invoice.clone(),
            to: document.clone(),
            expected_revision: 3,
        };
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingCreateRelation,
            CreateRelation,
            association.clone()
        );
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingCreateRelationOrVisualDraft,
            CreateRelationOrVisualDraft,
            ModelingCreateRelationRequested {
                kind: ModelingRelationKind::Generalization,
                owner_id: Some(invoice.clone()),
                expected_revision: 4,
                ..association
            }
        );
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingCreateRelationAtViewsOrVisualDraft,
            CreateRelationAtViewsOrVisualDraft,
            ModelingCreateRelationAtViewsRequested {
                project_id: id.clone(),
                diagram_id: "classes".into(),
                owner_id: Some("model".into()),
                kind: ModelingRelationKind::Dependency,
                from: Some(invoice.clone()),
                to: Some(document.clone()),
                source_view_id: invoice_view.clone(),
                target_view_id: document_view.clone(),
                expected_revision: 5,
            }
        );
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingSetProperty,
            SetProperty,
            ModelingSetPropertyRequested {
                project_id: id.clone(),
                element_id: invoice.clone(),
                path: "name".into(),
                value: json!("Invoice edited"),
                expected_revision: 6,
            }
        );
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingAddMember,
            AddMember,
            ModelingAddMemberRequested {
                project_id: id.clone(),
                owner_id: invoice.clone(),
                kind: ModelingMemberKind::Operation,
                expected_revision: 7,
            }
        );
        let operation_id = identity(
            &section(runtime.projects.snapshot(&id)?.as_ref())?,
            "Operation",
        )?;
        for expected_revision in [8, 9] {
            author!(
                runtime,
                bridge,
                through_ui,
                ModelingAddMember,
                AddMember,
                ModelingAddMemberRequested {
                    project_id: id.clone(),
                    owner_id: operation_id.clone(),
                    kind: ModelingMemberKind::Parameter,
                    expected_revision,
                }
            );
        }
        let members = section(runtime.projects.snapshot(&id)?.as_ref())?;
        let Some(Contract::Operation(operation)) = members
            .registry
            .get_entry(&operation_id)
            .map(|entry| &entry.contract)
        else {
            return Err("Missing Operation with parameters".into());
        };
        let [first_parameter, second_parameter] = operation.owned_parameter.as_slice() else {
            return Err("Operation must own exactly two Parameters".into());
        };
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingMoveParameter,
            MoveParameter,
            ModelingMoveParameterRequested {
                project_id: id.clone(),
                parameter_id: second_parameter.clone(),
                up: true,
                expected_revision: 10,
            }
        );
        author!(
            runtime,
            bridge,
            through_ui,
            ModelingQuickEdit,
            QuickEdit,
            ModelingQuickEditRequested {
                project_id: id.clone(),
                diagram_id: "classes".into(),
                element_id: invoice.clone(),
                view_id: Some(invoice_view.clone()),
                action: UmlQuickCreate::Note,
                expected_revision: 11,
            }
        );
        let authored = runtime.projects.snapshot(&id)?;
        assert_eq!(authored.revision(), 12);
        runtime.unchanged(&authored, true)?;
        let canonical = section(&authored)?;
        for (metaclass, count) in [
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
                canonical
                    .registry
                    .entries()
                    .values()
                    .filter(|entry| entry.type_name() == metaclass)
                    .count(),
                count,
                "{metaclass}"
            );
        }
        assert!(
            canonical
                .diagrams
                .iter()
                .flat_map(|diagram| diagram.all_views())
                .all(|view| view.draft.is_none())
        );
        let Some(Contract::Class(class)) = canonical
            .registry
            .get_entry(&invoice)
            .map(|entry| &entry.contract)
        else {
            return Err("Missing authored Class".into());
        };
        assert_eq!(class.name.as_deref(), Some("Invoice edited"));
        assert!(class.owned_operation.contains(&operation_id));
        let Some(Contract::Operation(operation)) = canonical
            .registry
            .get_entry(&operation_id)
            .map(|entry| &entry.contract)
        else {
            return Err("Missing authored Operation".into());
        };
        assert_eq!(
            operation.owned_parameter,
            vec![second_parameter.clone(), first_parameter.clone()]
        );
        let dependency_id = identity(&canonical, "Dependency")?;
        let dependency_view = canonical
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.subject.as_deref() == Some(dependency_id.as_str()))
            .ok_or("Missing Dependency View")?;
        let endpoints = dependency_view
            .endpoints
            .as_ref()
            .ok_or("Missing selected View endpoints")?;
        assert_eq!(
            endpoints.source_view_id.as_deref(),
            Some(invoice_view.as_str())
        );
        assert_eq!(
            endpoints.target_view_id.as_deref(),
            Some(document_view.as_str())
        );
        let Some(Contract::Model(model)) = canonical
            .registry
            .get_entry("model")
            .map(|entry| &entry.contract)
        else {
            return Err("Missing root Model".into());
        };
        assert!(model.packaged_element.contains(&dependency_id));
        let generalization_id = identity(&canonical, "Generalization")?;
        assert!(class.generalization.contains(&generalization_id));

        if through_ui {
            let (completion, receipt) = tokio::sync::watch::channel(None);
            submit(
                &bridge,
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
            runtime.unchanged(&authored, true)?;
        }
        for (format_name, format) in [
            ("json", ModelingDocumentFormat::ModelJson),
            ("protobuf", ModelingDocumentFormat::ModelProtobuf),
        ] {
            let encoded = bellatrix_mde::encode_model_section(
                id.as_str(),
                authored.name(),
                &serde_json::to_string(&canonical)?,
                format_name,
            )?;
            let reopened = runtime.prepare(&encoded, format).await?;
            assert_eq!(
                section(&reopened)?,
                canonical,
                "{format_name} must retain ownership and selected View endpoints"
            );
        }
        if through_ui {
            submit(&bridge, MdeUiCommand::CreateElement(first_request)).await?;
        } else {
            assert!(request!(runtime, ModelingCreateElement, first_request).is_err());
        }
        runtime.unchanged(&authored, true)?;
        outcomes.push(canonical);
        drop(bridge);
        runtime.stop().await?;
    }
    let [events, ui] = outcomes.as_slice() else {
        return Err("Both authoring routes must run".into());
    };
    assert_eq!(events, ui);
    Ok(())
}
