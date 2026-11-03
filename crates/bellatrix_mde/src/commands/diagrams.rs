use crate::{
    Diagram, ModelingSection, Project, StructuralDiagramKind, StructuralSeverity,
    validate_structural_diagram,
};

pub(super) fn create(
    section: &mut ModelingSection,
    project: &Project,
    kind: StructuralDiagramKind,
    name: &str,
    model_elements: &[String],
    revision: u64,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("diagram name must not be blank".into());
    }
    let [identifier] = super::fresh_ids(section, ["mde-diagram"], revision, &project.id)?;
    section.diagrams.push(Diagram {
        id: identifier.clone(),
        name: name.into(),
        width: 1280.0,
        height: 720.0,
        uml_di_type: kind.uml_di_type().into(),
        interaction_kind: kind.interaction_kind().map(str::to_owned),
        model_elements: model_elements.to_vec(),
        views: Vec::new(),
    });
    let candidate = section.to_model(&project.id, &project.name);
    // Whole-model semantic errors may predate this empty diagram. Only its
    // UMLDI metadata/context errors constrain this command; warnings allow drafts.
    for diagnostic in validate_structural_diagram(&candidate, &identifier, kind) {
        if diagnostic.severity == StructuralSeverity::Error && diagnostic.code.starts_with("UMLDI_")
        {
            return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
        }
    }
    Ok(())
}

pub(super) fn rename(
    section: &mut ModelingSection,
    identifier: &str,
    name: &str,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("diagram name must not be blank".into());
    }
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == identifier)
        .ok_or("diagram not found")?
        .name = name.into();
    Ok(())
}

pub(super) fn delete(section: &mut ModelingSection, identifier: &str) -> Result<(), String> {
    if !section
        .diagrams
        .iter()
        .any(|diagram| diagram.id == identifier)
    {
        return Err("diagram not found".into());
    }
    section.diagrams.retain(|diagram| diagram.id != identifier);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, apply};
    fn fixture() -> serde_json::Result<Project> {
        serde_json::from_str(include_str!("../../fixtures/todo.json"))
    }

    #[test]
    fn all_seven_diagram_categories_can_be_created_with_valid_context()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = fixture()?;
        for kind in StructuralDiagramKind::ALL {
            let context = if kind == StructuralDiagramKind::CompositeStructure {
                vec!["type-list".into()]
            } else {
                Vec::new()
            };
            let updated = apply(
                &source,
                &Command::CreateDiagram {
                    kind,
                    name: "New diagram".into(),
                    model_elements: context.clone(),
                },
                0,
            )?;
            let diagram = updated.diagrams.last().ok_or("created diagram missing")?;
            assert_eq!(diagram.uml_di_type.as_str(), kind.uml_di_type());
            assert_eq!(diagram.model_elements, context);
            assert!(diagram.views.is_empty());
            assert_eq!((diagram.width, diagram.height), (1280.0, 720.0));
            assert_eq!(updated.elements, source.elements);
            assert_eq!(updated.relations, source.relations);
            assert_eq!(updated.id, source.id);
            let second = apply(
                &updated,
                &Command::CreateDiagram {
                    kind,
                    name: "Another".into(),
                    model_elements: diagram.model_elements.clone(),
                },
                0,
            )?;
            assert_ne!(
                second.diagrams.last().ok_or("second diagram missing")?.id,
                diagram.id
            );
        }
        Ok(())
    }

    #[test]
    fn invalid_creation_and_missing_diagram_edits_leave_source_unchanged() -> serde_json::Result<()>
    {
        let source = fixture()?;
        let before = source.clone();
        for command in [
            Command::CreateDiagram {
                kind: StructuralDiagramKind::Class,
                name: " ".into(),
                model_elements: Vec::new(),
            },
            Command::CreateDiagram {
                kind: StructuralDiagramKind::Class,
                name: "Invalid context".into(),
                model_elements: vec!["type-list".into()],
            },
            Command::CreateDiagram {
                kind: StructuralDiagramKind::CompositeStructure,
                name: "Missing context".into(),
                model_elements: Vec::new(),
            },
            Command::CreateDiagram {
                kind: StructuralDiagramKind::CompositeStructure,
                name: "Unknown context".into(),
                model_elements: vec!["missing".into()],
            },
            Command::RenameDiagram {
                diagram_id: "missing".into(),
                name: "Name".into(),
            },
            Command::RenameDiagram {
                diagram_id: "diagram-tasks".into(),
                name: " ".into(),
            },
            Command::DeleteDiagram {
                diagram_id: "missing".into(),
            },
        ] {
            assert!(apply(&source, &command, 0).is_err());
            assert_eq!(source, before);
        }
        Ok(())
    }

    #[test]
    fn rename_and_delete_change_only_the_selected_diagram() -> Result<(), Box<dyn std::error::Error>>
    {
        let source = fixture()?;
        let renamed = apply(
            &source,
            &Command::RenameDiagram {
                diagram_id: "diagram-tasks".into(),
                name: "Renamed".into(),
            },
            0,
        )?;
        let mut expected = source.clone();
        for diagram in &mut expected.diagrams {
            if diagram.id == "diagram-tasks" {
                diagram.name = "Renamed".into();
            }
        }
        assert_eq!(renamed, expected);
        let deleted = apply(
            &renamed,
            &Command::DeleteDiagram {
                diagram_id: "diagram-tasks".into(),
            },
            1,
        )?;
        expected
            .diagrams
            .retain(|diagram| diagram.id != "diagram-tasks");
        assert_eq!(deleted, expected);
        Ok(())
    }

    #[test]
    fn unrelated_draft_generalization_errors_do_not_block_creation()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut source = fixture()?;
        source.relations.push(crate::Relation {
            id: "draft-self".into(),
            kind: "uml.generalization".into(),
            from: "type-list".into(),
            to: "type-list".into(),
            properties: Default::default(),
        });
        assert!(
            apply(
                &source,
                &Command::CreateDiagram {
                    kind: StructuralDiagramKind::Class,
                    name: "New draft".into(),
                    model_elements: Vec::new()
                },
                0
            )
            .is_ok()
        );
        Ok(())
    }
}
