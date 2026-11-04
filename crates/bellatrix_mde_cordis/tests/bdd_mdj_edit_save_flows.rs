use bellatrix_mde::{
    MODELING_NAMESPACE, ModelingSection, Project, Style, commands::Command, mdj_conversion,
};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingApplyMdjEdit, ModelingApplyMdjEditRequested, ModelingExportMdj,
    ModelingExportMdjRequested, with_model,
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

fn document() -> FlowResult<ProjectDocument> {
    let model = mdj_conversion::import(SOURCE)?.project;
    Ok(with_model(
        ProjectDocument::new(ProjectId::new(model.id.clone())?, model.name.clone())?,
        &model,
    )
    .map_err(std::io::Error::other)?)
}

fn model(document: &ProjectDocument) -> FlowResult<Project> {
    let section: ModelingSection = serde_json::from_value(
        document
            .section(MODELING_NAMESPACE)
            .ok_or("missing MDE section")?
            .clone(),
    )?;
    Ok(section.to_model(document.id().as_str(), document.name()))
}

fn style(document: &ProjectDocument) -> FlowResult<Style> {
    model(document)?
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == "class-view")
        .map(|view| view.style.clone())
        .ok_or_else(|| "missing View".into())
}

async fn edit(
    context: &Context,
    document: &Arc<ProjectDocument>,
    style: Style,
) -> FlowResult<QueryOutcome<Result<u64, String>>> {
    Ok(context
        .query::<ModelingApplyMdjEdit>(
            Routing::Unscoped,
            ModelingApplyMdjEditRequested {
                document: Arc::clone(document),
                command: Command::SetViewStyle {
                    diagram_id: "diagram".into(),
                    view_id: "class-view".into(),
                    style,
                },
            },
        )
        .await?)
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

#[tokio::test]
async fn mdj_edit_commit_and_external_save_acknowledgement_follow_the_actual_file() -> FlowResult<()>
{
    assert!(SPECIFICATION.contains(
        "Scenario: Commit only edits with an MDJ inverse and acknowledge only actual saved bytes"
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
                    document: document()?,
                    is_saved: true,
                },
            )
            .await?,
    )?;
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
    assert!(!service.is_dirty(&id)?);
    assert_eq!(answer(edit(&context, &opened, style(&opened)?).await?)?, 0);
    assert!(Arc::ptr_eq(&opened, &service.snapshot(&id)?));
    assert!(!service.is_dirty(&id)?);
    assert_eq!(changes.load(Ordering::SeqCst), 0);
    let mut unsupported = style(&opened)?;
    unsupported.opacity = Some(0.5);
    assert!(matches!(
        edit(&context, &opened, unsupported).await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert!(Arc::ptr_eq(&opened, &service.snapshot(&id)?));
    assert!(!service.is_dirty(&id)?);
    assert_eq!(changes.load(Ordering::SeqCst), 0);
    let mut supported = style(&opened)?;
    supported.fill = Some("#ABCDEF".into());
    assert_eq!(answer(edit(&context, &opened, supported).await?)?, 1);
    let edited = service.snapshot(&id)?;
    assert!(service.is_dirty(&id)?);
    assert_eq!(changes.load(Ordering::SeqCst), 1);
    assert_eq!(answer(edit(&context, &edited, style(&edited)?).await?)?, 1);
    assert!(Arc::ptr_eq(&edited, &service.snapshot(&id)?));
    assert!(service.is_dirty(&id)?);
    assert_eq!(changes.load(Ordering::SeqCst), 1);
    let exported = export(&context, &edited).await?;
    assert!(exported.matches_snapshot(&edited));
    assert_eq!(
        style(
            &with_model(
                ProjectDocument::new(id.clone(), edited.name())?,
                &mdj_conversion::import(exported.bytes())?.project
            )
            .map_err(std::io::Error::other)?
        )?,
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
    fs::write(&destination, ticket.bytes())?;
    let reopened = fs::read(&destination)?;
    assert_eq!(
        mdj_conversion::export(&mdj_conversion::import(&reopened)?.project)?,
        reopened
    );
    assert_eq!(
        answer(
            context
                .query::<ProjectAcknowledgeExternalSave>(
                    Routing::Unscoped,
                    ProjectAcknowledgeExternalSaveRequested {
                        ticket,
                        written_bytes: reopened.into()
                    }
                )
                .await?
        )?,
        1
    );
    assert!(!service.is_dirty(&id)?);
    assert_eq!(saved_events.load(Ordering::SeqCst), 1);
    assert_eq!(fs::read(&source_path)?, SOURCE);
    let ticket = answer(
        context
            .query::<ProjectPrepareEncodedSave>(
                Routing::Unscoped,
                exported.prepare_save(saved.0.join("mismatch.mdj")),
            )
            .await?,
    )?;
    assert!(matches!(
        context
            .query::<ProjectAcknowledgeExternalSave>(
                Routing::Unscoped,
                ProjectAcknowledgeExternalSaveRequested {
                    ticket,
                    written_bytes: Arc::from(&b"not the prepared file"[..])
                }
            )
            .await?,
        QueryOutcome::Answer(Err(ProjectError::SaveContentMismatch))
    ));
    assert!(service.is_dirty(&id)?);
    assert_eq!(saved_events.load(Ordering::SeqCst), 1);
    changed_listener.remove();
    saved_listener.remove();
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}

#[tokio::test]
async fn mdj_save_preserves_newer_edits_and_rejects_closed_session_receipts() -> FlowResult<()> {
    assert!(SPECIFICATION.contains(
        "Scenario: Preserve newer edits and reject receipts from a closed document session"
    ));
    let saved = SavedFlow::new("sessions")?;
    let (context, service, provider, plugin) = start().await?;
    let original = document()?;
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
    assert_eq!(answer(edit(&context, &opened, changed).await?)?, 1);
    fs::write(ticket.path(), ticket.bytes())?;
    let written_bytes = fs::read(ticket.path())?.into();
    assert!(matches!(
        context
            .query::<ProjectAcknowledgeExternalSave>(
                Routing::Unscoped,
                ProjectAcknowledgeExternalSaveRequested {
                    ticket,
                    written_bytes
                }
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
        edit(&context, &opened, style(&opened)?).await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert!(Arc::ptr_eq(&reopened, &service.snapshot(&id)?));
    assert!(!service.is_dirty(&id)?);
    assert_eq!(export(&context, &reopened).await?.bytes(), SOURCE);
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}

#[tokio::test]
async fn mdj_interchange_rejects_extra_sections_without_restricting_native_documents()
-> FlowResult<()> {
    assert!(
        SPECIFICATION
            .contains("Scenario: Keep native authoring distinct from lossless MDJ editing")
    );
    let saved = SavedFlow::new("native-sections")?;
    let (context, service, provider, plugin) = start().await?;
    let document = document()?.with_section("example.extra", json!({"future": [null, true]}))?;
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
    assert!(export(&context, &opened).await.is_err());
    assert!(matches!(
        edit(&context, &opened, style(&opened)?).await?,
        QueryOutcome::Answer(Err(_))
    ));
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
                        id: id.clone(),
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
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}
