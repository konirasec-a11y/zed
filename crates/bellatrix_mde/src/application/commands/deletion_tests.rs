use super::{Command, apply};
use crate::Project;

fn fixture() -> serde_json::Result<Project> {
    serde_json::from_value(serde_json::json!({
        "id": "model", "name": "Context ownership",
        "elements": [
            {"id": "owner", "kind": "uml.package", "name": "Owner"},
            {"id": "context", "kind": "uml.class", "name": "Context"},
            {"id": "unrelated", "kind": "uml.class", "name": "Unrelated"}
        ],
        "relations": [{"id": "owns-context", "kind": "owns", "from": "owner", "to": "context"}],
        "diagrams": [{"id": "composite", "name": "Composite", "uml_di_type": "UMLCompositeStructureDiagram",
            "model_elements": ["context"]}]
    }))
}

#[test]
fn deleting_context_or_its_owner_is_rejected_without_mutation() -> serde_json::Result<()> {
    let source = fixture()?;
    let before = source.clone();
    for identifier in ["context", "owner"] {
        let result = apply(
            &source,
            &Command::DeleteElement {
                element_id: identifier.into(),
            },
            0,
        );
        assert!(
            matches!(result, Err(ref error) if error.contains("UMLDI_CONTEXT_IN_USE")
            && error.contains("composite") && error.contains("context")),
            "{result:?}"
        );
        assert_eq!(source, before);
    }
    Ok(())
}

#[test]
fn removing_diagram_first_allows_context_and_ownership_closure_deletion()
-> Result<(), Box<dyn std::error::Error>> {
    let source = fixture()?;
    for identifier in ["context", "owner"] {
        let without_diagram = apply(
            &source,
            &Command::DeleteDiagram {
                diagram_id: "composite".into(),
            },
            0,
        )?;
        assert_eq!(without_diagram.elements, source.elements);
        assert_eq!(without_diagram.relations, source.relations);
        let deleted = apply(
            &without_diagram,
            &Command::DeleteElement {
                element_id: identifier.into(),
            },
            1,
        )?;
        assert!(deleted.element("context").is_none());
        assert!(deleted.element("unrelated").is_some());
        assert!(deleted.relations.is_empty());
        assert!(deleted.diagrams.is_empty());
        assert_eq!(deleted.element("owner").is_some(), identifier != "owner");
    }
    Ok(())
}

#[test]
fn unrelated_deletion_preserves_composite_context() -> Result<(), Box<dyn std::error::Error>> {
    let source = fixture()?;
    let deleted = apply(
        &source,
        &Command::DeleteElement {
            element_id: "unrelated".into(),
        },
        0,
    )?;
    assert!(deleted.element("unrelated").is_none());
    assert!(deleted.element("context").is_some());
    assert!(deleted.element("owner").is_some());
    assert_eq!(deleted.relations, source.relations);
    assert_eq!(deleted.diagrams, source.diagrams);
    Ok(())
}
