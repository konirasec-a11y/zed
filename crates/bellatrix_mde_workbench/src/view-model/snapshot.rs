//! Read-only projection of a Bellatrix project for a Zed Workspace item.
//! This does not own the model or modify ProjectService.
use bellatrix_mde::{MODELING_NAMESPACE, ModelingSection};
use bellatrix_project::{ProjectDocument, ProjectId};

#[derive(Clone, Debug)]
pub struct MdeSnapshot {
    pub project_id: ProjectId,
    pub name: String,
    pub revision: u64,
    pub section: ModelingSection,
}
pub fn from_document(document: &ProjectDocument) -> Result<MdeSnapshot, String> {
    let value = document.section(MODELING_NAMESPACE)
        .ok_or_else(|| format!("unsupported or missing MDE section in project {}: expected bellatrix.mde; beta documents in other namespaces are not supported", document.id().as_str()))?;
    let section = serde_json::from_value::<ModelingSection>(value.clone())
        .map_err(|error| format!("invalid MDE section: {error}"))?;
    bellatrix_mde::validate_model_hierarchy(&section.to_model(document.id().as_str(), document.name()))?;
    Ok(MdeSnapshot {
        project_id: document.id().clone(),
        name: document.name().to_owned(),
        revision: document.revision(),
        section,
    })
}
