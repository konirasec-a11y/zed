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
        let document = document().with_section(MODELING_NAMESPACE, json!({
            "elements": [], "relations": [], "diagrams": [{ "id": "diagram-1", "name": "Classes", "uml_di_type": "UMLClassDiagram", "views": [] }]
        })).unwrap();
        let snapshot = from_document(&document).unwrap();
        assert_eq!(snapshot.name, "MDE test");
        assert_eq!(snapshot.project_id.as_str(), "mde-test");
        assert_eq!(snapshot.revision, 0);
        assert_eq!(snapshot.section.diagrams[0].name, "Classes");
    }
    #[test]
    fn old_namespaces_are_rejected_without_rewriting_or_creating_state() {
        for namespace in ["bellatrix.bellatrix-mde-modeling", "bellatrix.modeling"] {
            let document = document()
                .with_section(
                    namespace,
                    json!({"elements": [], "relations": [], "diagrams": []}),
                )
                .unwrap();
            assert!(
                matches!(from_document(&document), Err(error) if error.contains("not supported"))
            );
            assert!(document.section(namespace).is_some());
            assert!(document.section(MODELING_NAMESPACE).is_none());
        }
    }
    #[test]
    fn invalid_canonical_section_is_rejected() {
        let document = document()
            .with_section(MODELING_NAMESPACE, json!({"invalid": true}))
            .unwrap();
        assert!(from_document(&document).is_err());
    }
    #[test]
    fn no_section_does_not_create_an_empty_model() {
        assert!(from_document(&document()).is_err());
    }
}
