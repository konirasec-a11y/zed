//! Read-only projection of a Bellatrix project for a Zed Workspace item.
//! This does not own the model or modify ProjectService.

use bellatrix_mde_model::ModelingSection;
use bellatrix_project::{ProjectDocument, ProjectId};

/// Current persisted namespace in the MDE integration. Do not silently rename it.
pub const MDE_NAMESPACE: &str = "bellatrix.bellatrix-mde-modeling";
/// Previous public namespace, kept readable while a versioned migration is designed.
pub const LEGACY_MODELING_NAMESPACE: &str = "bellatrix.modeling";

/// Immutable domain snapshot exposed to the GPUI adapter.
#[derive(Clone, Debug)]
pub struct MdeSnapshot {
    pub project_id: ProjectId,
    pub name: String,
    pub revision: u64,
    pub section: ModelingSection,
}

/// Decode the model without mutating the document or requiring a Zed Project.
pub fn from_document(document: &ProjectDocument) -> Result<MdeSnapshot, String> {
    let value = document
        .section(MDE_NAMESPACE)
        .or_else(|| document.section(LEGACY_MODELING_NAMESPACE))
        .ok_or_else(|| format!("no MDE section in project {}", document.id().as_str()))?;
    let section = serde_json::from_value::<ModelingSection>(value.clone())
        .map_err(|error| format!("invalid MDE section: {error}"))?;
    Ok(MdeSnapshot {
        project_id: document.id().clone(),
        name: document.name().to_owned(),
        revision: document.revision(),
        section,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document() -> ProjectDocument {
        ProjectDocument::new(ProjectId::new("mde-test").unwrap(), "MDE test").unwrap()
    }

    #[test]
    fn reads_current_namespace() {
        let document = document()
            .with_section(
                MDE_NAMESPACE,
                json!({
                    "elements": [],
                    "relations": [],
                    "diagrams": [{
                        "id": "diagram-1",
                        "name": "Classes",
                        "views": []
                    }]
                }),
            )
            .unwrap();
        let snapshot = from_document(&document).unwrap();
        assert_eq!(snapshot.name, "MDE test");
        assert_eq!(snapshot.project_id.as_str(), "mde-test");
        assert_eq!(snapshot.revision, 0);
        assert_eq!(snapshot.section.diagrams[0].name, "Classes");
    }

    #[test]
    fn reads_legacy_namespace_without_rewriting_it() {
        let document = document()
            .with_section(
                LEGACY_MODELING_NAMESPACE,
                json!({"elements": [], "relations": [], "diagrams": []}),
            )
            .unwrap();
        assert!(from_document(&document).is_ok());
        assert!(document.section(LEGACY_MODELING_NAMESPACE).is_some());
        assert!(document.section(MDE_NAMESPACE).is_none());
    }

    #[test]
    fn current_namespace_takes_precedence() {
        let document = document()
            .with_section(
                LEGACY_MODELING_NAMESPACE,
                json!({"elements": [], "relations": [], "diagrams": []}),
            )
            .unwrap()
            .with_section(MDE_NAMESPACE, json!({"invalid": true}))
            .unwrap();
        assert!(from_document(&document).is_err());
    }

    #[test]
    fn no_section_does_not_create_an_empty_model() {
        assert!(from_document(&document()).is_err());
    }
}
