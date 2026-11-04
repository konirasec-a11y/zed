use crate::project_integration::from_document;
use bellatrix_mde::{MODELING_NAMESPACE, Project, commands::Command, mdj_conversion};
use bellatrix_project::{ProjectDocument, ProjectPrepareEncodedSaveRequested, ProjectService};
use cordis::{Context, Event, event::responder};
use std::{convert::Infallible, io, sync::Arc};

#[derive(Clone)]
pub struct ModelingApplyMdjEditRequested {
    pub document: Arc<ProjectDocument>,
    pub command: Command,
}

pub struct ModelingApplyMdjEdit;
impl Event for ModelingApplyMdjEdit {
    const NAME: &'static str = "bellatrix.mde.apply-mdj-edit.v1";
    type Args = ModelingApplyMdjEditRequested;
    type Output = Result<u64, String>;
}

// A revision alone cannot identify a reopened session. The native bridge needs
// the exact committed Arc without looking up an identity that may have changed.
pub(crate) struct ModelingApplyMdjEditSnapshot;
impl Event for ModelingApplyMdjEditSnapshot {
    const NAME: &'static str = "bellatrix.mde.apply-mdj-edit-snapshot.v1";
    type Args = ModelingApplyMdjEditRequested;
    type Output = Result<Arc<ProjectDocument>, String>;
}

#[derive(Clone)]
pub struct ModelingExportMdjRequested {
    pub document: Arc<ProjectDocument>,
}

pub struct ModelingExportMdj;
impl Event for ModelingExportMdj {
    const NAME: &'static str = "bellatrix.mde.export-mdj.v1";
    type Args = ModelingExportMdjRequested;
    type Output = Result<MdjExportReceipt, String>;
}

/// Verified bytes for one immutable snapshot, not a write reservation or a save acknowledgement.
/// Native adapters must still guard source/destination changes and acknowledge actual I/O.
#[derive(Clone, Debug)]
pub struct MdjExportReceipt {
    document: Arc<ProjectDocument>,
    bytes: Arc<[u8]>,
}

impl MdjExportReceipt {
    pub fn document(&self) -> &Arc<ProjectDocument> {
        &self.document
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn matches_snapshot(&self, snapshot: &Arc<ProjectDocument>) -> bool {
        Arc::ptr_eq(&self.document, snapshot)
    }

    pub fn prepare_save(&self, path: std::path::PathBuf) -> ProjectPrepareEncodedSaveRequested {
        ProjectPrepareEncodedSaveRequested {
            document: Arc::clone(&self.document),
            path,
            bytes: Arc::clone(&self.bytes),
        }
    }
}

pub(crate) fn ensure_mdj_document(
    document: &ProjectDocument,
    model: &Project,
) -> Result<(), String> {
    if model.mdj_provenance.is_none() {
        return Err("MDJ interchange requires an imported StarUML document".into());
    }
    if document
        .sections()
        .keys()
        .any(|namespace| namespace != MODELING_NAMESPACE)
    {
        return Err("MDJ interchange cannot represent other platform sections; save the complete native project instead".into());
    }
    Ok(())
}

async fn export_snapshot(
    projects: &ProjectService,
    request: ModelingExportMdjRequested,
) -> Result<MdjExportReceipt, String> {
    let document = projects
        .snapshot(request.document.id())
        .map_err(|error| error.to_string())?;
    if !Arc::ptr_eq(&document, &request.document) {
        return Err("stale MDJ export; the requested snapshot is no longer open".into());
    }
    let model = from_document(&document)?.to_model(document.id().as_str(), document.name());
    ensure_mdj_document(&document, &model)?;
    let bytes = tokio::task::spawn_blocking(move || mdj_conversion::export(&model))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let current = projects
        .snapshot(document.id())
        .map_err(|error| error.to_string())?;
    if !Arc::ptr_eq(&document, &current) {
        return Err("document changed while preparing MDJ bytes; no save was acknowledged".into());
    }
    Ok(MdjExportReceipt {
        document,
        bytes: bytes.into(),
    })
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let edits = Arc::clone(&projects);
    let _edit = context
        .on::<ModelingApplyMdjEdit, _>(responder(
            move |context: Context, request: ModelingApplyMdjEditRequested| {
                let projects = Arc::clone(&edits);
                async move {
                    Ok::<_, Infallible>(Some(
                        crate::mutation::execute_mdj(
                            &context,
                            &projects,
                            request.document,
                            request.command,
                        )
                        .await
                        .map(|document| document.revision()),
                    ))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let snapshots = Arc::clone(&projects);
    let _snapshot_edit = context
        .on::<ModelingApplyMdjEditSnapshot, _>(responder(
            move |context: Context, request: ModelingApplyMdjEditRequested| {
                let projects = Arc::clone(&snapshots);
                async move {
                    Ok::<_, Infallible>(Some(
                        crate::mutation::execute_mdj(
                            &context,
                            &projects,
                            request.document,
                            request.command,
                        )
                        .await,
                    ))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let _export = context
        .on::<ModelingExportMdj, _>(responder(
            move |_: Context, request: ModelingExportMdjRequested| {
                let projects = Arc::clone(&projects);
                async move { Ok::<_, Infallible>(Some(export_snapshot(&projects, request).await)) }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}
