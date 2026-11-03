use crate::*;
use cordis::event::responder;
use cordis::{Context, Plugin, Routing};
use std::{convert::Infallible, io, sync::Arc};

pub struct ProjectPlugin {
    service: Arc<ProjectService>,
}

impl ProjectPlugin {
    pub fn new(service: Arc<ProjectService>) -> Self {
        Self { service }
    }
}

impl Plugin for ProjectPlugin {
    type Config = ();
    type Input = Arc<ProjectService>;
    type PrepareError = Infallible;
    type ApplyError = io::Error;

    fn prepare(&self, (): ()) -> Result<Self::Input, Self::PrepareError> {
        Ok(Arc::clone(&self.service))
    }

    async fn apply(&self, ctx: Context, service: &Self::Input) -> Result<(), Self::ApplyError> {
        let _published = ctx
            .provide(Arc::clone(service))
            .map_err(|e| io::Error::other(e.to_string()))?;
        register_create(&ctx, service)?;
        register_external_storage(&ctx, service)?;
        register_open(&ctx, service)?;
        register_replace(&ctx, service)?;
        register_save(&ctx, service)?;
        register_close(&ctx, service)?;
        Ok(())
    }
}

fn register_create(ctx: &Context, service: &Arc<ProjectService>) -> Result<(), io::Error> {
    let service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectCreate, _>(responder(move |ctx, args: ProjectCreateRequested| {
            let service = Arc::clone(&service);
            async move {
                let result = service.open(args.document, false);
                if let Ok(document) = &result {
                    notify::<ProjectOpenedEvent>(
                        &ctx,
                        ProjectOpened {
                            document: Arc::clone(document),
                            from_disk: false,
                        },
                    )
                    .await;
                }
                Ok::<_, Infallible>(Some(result))
            }
        }))
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

fn register_open(ctx: &Context, service: &Arc<ProjectService>) -> Result<(), io::Error> {
    let service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectOpen, _>(responder(move |ctx, args: ProjectOpenRequested| {
            let service = Arc::clone(&service);
            async move {
                let loaded = tokio::task::spawn_blocking(move || crate::file::load(&args.path))
                    .await
                    .map_err(|err| ProjectError::Io(io::Error::other(err.to_string())));
                let result = loaded
                    .and_then(|value| value)
                    .and_then(|doc| service.open(doc, true));
                if let Ok(document) = &result {
                    notify::<ProjectOpenedEvent>(
                        &ctx,
                        ProjectOpened {
                            document: Arc::clone(document),
                            from_disk: true,
                        },
                    )
                    .await;
                }
                Ok::<_, Infallible>(Some(result))
            }
        }))
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

fn register_replace(ctx: &Context, service: &Arc<ProjectService>) -> Result<(), io::Error> {
    let service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectReplace, _>(responder(move |ctx, args: ProjectReplaceRequested| {
            let service = Arc::clone(&service);
            async move {
                let result = service.replace(
                    &args.id,
                    args.namespace.clone(),
                    args.expected_revision,
                    args.state,
                );
                let result = match result {
                    Ok(Some(document)) => {
                        let value = document
                            .section(&args.namespace)
                            .expect("updated namespace must exist")
                            .clone();
                        let changed = ProjectStateChanged {
                            id: args.id,
                            namespace: args.namespace,
                            previous_revision: args.expected_revision,
                            revision: document.revision(),
                            state: Arc::new(value),
                        };
                        notify::<ProjectStateChangedEvent>(&ctx, changed).await;
                        Ok(document)
                    }
                    Ok(None) => service.snapshot(&args.id),
                    Err(error) => Err(error),
                };
                Ok::<_, Infallible>(Some(result))
            }
        }))
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

fn register_save(ctx: &Context, service: &Arc<ProjectService>) -> Result<(), io::Error> {
    let service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectSave, _>(responder(move |ctx, args: ProjectSaveRequested| {
            let service = Arc::clone(&service);
            async move {
                let id = args.id;
                let path = args.path;
                let saved = tokio::task::spawn_blocking({
                    let service = Arc::clone(&service);
                    let id = id.clone();
                    let path = path.clone();
                    move || service.save(&id, &path)
                })
                .await
                .map_err(|e| ProjectError::Io(io::Error::other(e.to_string())));
                let result = saved.and_then(|value| value);
                if let Ok(revision) = &result {
                    notify::<ProjectSavedEvent>(
                        &ctx,
                        ProjectSaved {
                            id,
                            path,
                            revision: *revision,
                        },
                    )
                    .await;
                }
                Ok::<_, Infallible>(Some(result))
            }
        }))
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

fn register_close(ctx: &Context, service: &Arc<ProjectService>) -> Result<(), io::Error> {
    let service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectClose, _>(responder(move |ctx, args: ProjectCloseRequested| {
            let service = Arc::clone(&service);
            async move {
                let id = args.id;
                let discard = args.discard_unsaved;
                let result = tokio::task::spawn_blocking({
                    let service = Arc::clone(&service);
                    let id = id.clone();
                    move || service.close(&id, discard)
                })
                .await
                .map_err(|e| ProjectError::Io(io::Error::other(e.to_string())))
                .and_then(|value| value);
                if result.is_ok() {
                    notify::<ProjectClosedEvent>(&ctx, ProjectClosed { id }).await;
                }
                Ok::<_, Infallible>(Some(result))
            }
        }))
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

async fn notify<E: cordis::Event<Output = ()>>(ctx: &Context, message: E::Args)
where
    E::Args: Clone,
{
    if let Err(error) = ctx.emit_parallel::<E>(Routing::Unscoped, message).await {
        eprintln!("Bellatrix Cordis observer failure: {error}");
    }
}

fn register_external_storage(
    ctx: &Context,
    service: &Arc<ProjectService>,
) -> Result<(), io::Error> {
    let import_service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectImport, _>(responder(move |ctx, args: ProjectImportRequested| {
            let service = Arc::clone(&import_service);
            async move {
                let result = service.open(args.document, args.is_saved);
                if let Ok(document) = &result {
                    notify::<ProjectOpenedEvent>(
                        &ctx,
                        ProjectOpened {
                            document: Arc::clone(document),
                            from_disk: args.is_saved,
                        },
                    )
                    .await;
                }
                Ok::<_, Infallible>(Some(result))
            }
        }))
        .map_err(|e| io::Error::other(e.to_string()))?;
    let prepare_service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectPrepareExternalSave, _>(responder(
            move |_, args: ProjectPrepareExternalSaveRequested| {
                let service = Arc::clone(&prepare_service);
                async move {
                    Ok::<_, Infallible>(Some(service.prepare_external_save(
                        &args.id,
                        args.expected_revision,
                        args.path,
                    )))
                }
            },
        ))
        .map_err(|e| io::Error::other(e.to_string()))?;
    let service = Arc::clone(service);
    let _listener = ctx
        .on::<ProjectAcknowledgeExternalSave, _>(responder(
            move |ctx, args: ProjectAcknowledgeExternalSaveRequested| {
                let service = Arc::clone(&service);
                async move {
                    let result =
                        service.acknowledge_external_save(&args.ticket, &args.written_bytes);
                    if let Ok(revision) = &result {
                        notify::<ProjectSavedEvent>(
                            &ctx,
                            ProjectSaved {
                                id: args.ticket.document().id().clone(),
                                path: args.ticket.path().to_path_buf(),
                                revision: *revision,
                            },
                        )
                        .await;
                    }
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}
