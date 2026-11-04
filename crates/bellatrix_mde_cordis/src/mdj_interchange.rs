use crate::MdjEditCommand;
use crate::project_integration::section_json;
use crate::MODELING_NAMESPACE;
use bellatrix_project::{ProjectDocument, ProjectPrepareEncodedSaveRequested, ProjectService};
use cordis::{Context, Event, event::responder};
use std::{convert::Infallible, io, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub enum ModelingDocumentFormat {
    ProjectJson,
    ModelJson,
    ModelProtobuf,
    StarUml,
}

#[derive(Clone, Debug)]
pub struct ModelingPrepareDocumentRequested {
    pub bytes: Vec<u8>,
    pub format: ModelingDocumentFormat,
}

#[derive(Clone, Debug)]
pub struct PreparedModelingDocument {
    pub document: ProjectDocument,
    pub diagnostics: Vec<String>,
}

pub struct ModelingPrepareDocument;
impl Event for ModelingPrepareDocument {
    const NAME: &'static str = "bellatrix.mde.prepare-document.v1";
    type Args = ModelingPrepareDocumentRequested;
    type Output = Result<PreparedModelingDocument, String>;
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelDocumentResponse {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    #[serde(default)]
    diagnostics: Vec<MdjImportDiagnostic>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MdjImportDiagnostic {
    path: String,
    message: String,
}

fn api_error_message(error: String) -> String {
    serde_json::from_str::<serde_json::Value>(&error).ok()
        .and_then(|response| response.get("error")?.get("message")?.as_str().map(str::to_owned))
        .unwrap_or(error)
}

fn document_from_response(response: &str) -> Result<PreparedModelingDocument, String> {
    let imported: ModelDocumentResponse = serde_json::from_str(response)
        .map_err(|error| format!("Invalid model document response: {error}"))?;
    if imported.api_version != 1 {
        return Err("Unsupported model document API version".into());
    }
    #[derive(serde::Serialize)]
    struct ModelRequest<'a> { api_version: u32, model: &'a serde_json::value::RawValue }
    let request = serde_json::to_string(&ModelRequest { api_version: imported.api_version, model: &imported.model })
        .map_err(|error| error.to_string())?;
    let parts = bellatrix_mde::prepare_model_document(&request).map_err(api_error_message)?;
    let id = bellatrix_project::ProjectId::new(parts.id).map_err(|error| error.to_string())?;
    let state = serde_json::from_str(&parts.section_json).map_err(|error| error.to_string())?;
    let document = ProjectDocument::new(id, parts.name).map_err(|error| error.to_string())?
        .with_section(MODELING_NAMESPACE, state).map_err(|error| error.to_string())?;
    let diagnostics = imported.diagnostics.into_iter().map(|diagnostic| {
        let path = if diagnostic.path.is_empty() { "/" } else { &diagnostic.path };
        format!("{path}: {}", diagnostic.message)
    }).collect();
    Ok(PreparedModelingDocument { document, diagnostics })
}

#[derive(serde::Deserialize)]
struct RawProjectDocument {
    id: bellatrix_project::ProjectId,
    name: String,
    #[serde(default, deserialize_with = "deserialize_raw_sections")]
    sections: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>>,
}

fn deserialize_raw_sections<'de, D>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, Box<serde_json::value::RawValue>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct SectionsVisitor;
    impl<'de> serde::de::Visitor<'de> for SectionsVisitor {
        type Value = std::collections::BTreeMap<String, Box<serde_json::value::RawValue>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("project sections with unique namespaces")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut sections = std::collections::BTreeMap::new();
            while let Some(namespace) = access.next_key::<String>()? {
                if sections.contains_key(&namespace) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate project namespace: {namespace}"
                    )));
                }
                sections.insert(namespace, access.next_value::<Box<serde_json::value::RawValue>>()?);
            }
            Ok(sections)
        }
    }
    deserializer.deserialize_map(SectionsVisitor)
}

fn prepare_document(request: ModelingPrepareDocumentRequested) -> Result<PreparedModelingDocument, String> {
    match request.format {
        ModelingDocumentFormat::StarUml => {
            let response = bellatrix_mde::import_mdj_document(&request.bytes)
                .map_err(|error| format!("Invalid StarUML MDJ document: {}", api_error_message(error)))?;
            document_from_response(&response)
        }
        ModelingDocumentFormat::ModelJson | ModelingDocumentFormat::ModelProtobuf => {
            let format = match request.format {
                ModelingDocumentFormat::ModelProtobuf => "protobuf",
                _ => "json",
            };
            let response = bellatrix_mde::decode_model_document(&request.bytes, format)
                .map_err(api_error_message)?;
            document_from_response(&response)
        }
        ModelingDocumentFormat::ProjectJson => {
            // Validate the original section before Value can overwrite duplicate
            // authoring, hierarchy or composition keys rejected by the engine.
            let raw: RawProjectDocument = serde_json::from_slice(&request.bytes)
                .map_err(|error| format!("Invalid Bellatrix project document: {error}"))?;
            let section = raw.sections.get(MODELING_NAMESPACE)
                .ok_or("unsupported or missing MDE section: expected bellatrix.mde; beta documents in other namespaces are not supported")?;
            bellatrix_mde::validate_model_section(raw.id.as_str(), &raw.name, section.get())?;
            let document: ProjectDocument = serde_json::from_slice(&request.bytes)
                .map_err(|error| format!("Invalid Bellatrix project document: {error}"))?;
            Ok(PreparedModelingDocument { document, diagnostics: Vec::new() })
        }
    }
}

#[derive(Clone)]
pub struct ModelingApplyMdjEditRequested {
    pub document: Arc<ProjectDocument>,
    pub command: MdjEditCommand,
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

fn ensure_mdj_sections(document: &ProjectDocument) -> Result<(), String> {
    if document.sections().keys().any(|namespace| namespace != MODELING_NAMESPACE) {
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
    ensure_mdj_sections(&document)?;
    let section = section_json(&document)?;
    let id = document.id().as_str().to_owned();
    let name = document.name().to_owned();
    let bytes = tokio::task::spawn_blocking(move || bellatrix_mde::export_mdj_section(&id, &name, &section).map_err(api_error_message))
        .await
        .map_err(|error| error.to_string())??;
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
    let _prepare = context
        .on::<ModelingPrepareDocument, _>(responder(
            move |_: Context, request: ModelingPrepareDocumentRequested| async move {
                let result = tokio::task::spawn_blocking(move || prepare_document(request))
                    .await.map_err(|error| error.to_string()).and_then(|result| result);
                Ok::<_, Infallible>(Some(result))
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
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
