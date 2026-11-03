use crate::{Project, StructuralSeverity, validate_structural_model};
use std::collections::HashSet;

pub(super) fn new_relationships(before: &Project, after: &Project) -> Result<(), String> {
    let existing = before
        .relations
        .iter()
        .map(|relation| relation.id.as_str())
        .collect::<HashSet<_>>();
    let added = after
        .relations
        .iter()
        .filter(|relation| !existing.contains(relation.id.as_str()))
        .collect::<Vec<_>>();
    if added.is_empty() {
        return Ok(());
    }
    let identifiers = added
        .iter()
        .map(|relation| relation.id.as_str())
        .collect::<HashSet<_>>();
    // Draft diagnostics on unrelated, existing relationships must not block edits.
    for diagnostic in validate_structural_model(after) {
        if diagnostic.severity == StructuralSeverity::Error
            && identifiers.contains(diagnostic.subject_id.as_str())
        {
            return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
        }
    }
    let graph = crate::GraphIndex::new(after);
    // Use the shared graph query: a pre-existing unrelated cycle must not hide
    // a second cycle introduced by a new edge, nor block an unrelated edit.
    for relation in added
        .into_iter()
        .filter(|relation| relation.kind == "uml.generalization")
    {
        if graph
            .shortest_path(&relation.to, &relation.from, Some("uml.generalization"))
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Err(format!(
                "UML_GENERALIZATION_CYCLE: {} would create a generalization cycle",
                relation.id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ModelingRelationKind, Relation, UmlQuickCreate,
        commands::{Command, apply},
    };

    fn project(kinds: [&str; 4]) -> serde_json::Result<Project> {
        let identifiers = ["a", "b", "c", "d"];
        let elements = identifiers.iter().zip(kinds).map(|(identifier, kind)|
            serde_json::json!({"id": identifier, "name": identifier, "kind": kind})).collect::<Vec<_>>();
        let views = identifiers.iter().enumerate().map(|(index, identifier)| serde_json::json!({
            "id": format!("view-{identifier}"), "subject": identifier,
            "geometry": {"kind": "rect", "x": index as f64 * 220.0, "y": 100.0, "width": 180.0, "height": 100.0}
        })).collect::<Vec<_>>();
        serde_json::from_value(serde_json::json!({"id": "model", "name": "Draft",
            "elements": elements, "diagrams": [{"id": "diagram", "name": "Classes", "uml_di_type": "UMLClassDiagram", "views": views}]}))
    }

    fn edge(project: &mut Project, identifier: &str, from: &str, to: &str) {
        project.relations.push(Relation {
            id: identifier.into(),
            kind: "uml.generalization".into(),
            from: from.into(),
            to: to.into(),
            properties: Default::default(),
        });
    }

    fn create(from: &str, to: &str) -> Command {
        Command::CreateRelation {
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::Generalization,
            from: from.into(),
            to: to.into(),
        }
    }

    fn rejected_without_mutation(project: &Project, command: Command, code: &str) {
        let before = project.clone();
        let result = apply(project, &command, 0);
        assert!(
            matches!(result, Err(ref error) if error.contains(code)),
            "{result:?}"
        );
        assert_eq!(project, &before);
    }

    #[test]
    fn toolbox_rejects_incompatible_and_stereotype_generalizations() -> serde_json::Result<()> {
        for (specific, general, code) in [
            ("uml.class", "uml.interface", "UML_GENERALIZATION_TYPE"),
            ("uml.class", "uml.component", "UML_GENERALIZATION_TYPE"),
            (
                "uml.stereotype",
                "uml.class",
                "UML_STEREOTYPE_GENERALIZATION",
            ),
            (
                "uml.class",
                "uml.stereotype",
                "UML_STEREOTYPE_GENERALIZATION",
            ),
        ] {
            let model = project([specific, general, "uml.class", "uml.class"])?;
            rejected_without_mutation(&model, create("a", "b"), code);
        }
        Ok(())
    }

    #[test]
    fn structural_classifier_endpoints_use_the_shared_registry() -> serde_json::Result<()> {
        for (specific, general) in [
            ("uml.component", "uml.class"),
            ("uml.device", "uml.node"),
            ("uml.execution_environment", "uml.node"),
            ("uml.artifact", "uml.artifact"),
            ("uml.stereotype", "uml.stereotype"),
            ("uml.collaboration", "uml.collaboration"),
        ] {
            let model = project([specific, general, "uml.class", "uml.class"])?;
            assert!(
                apply(&model, &create("a", "b"), 0).is_ok(),
                "{specific} -> {general}"
            );
        }
        assert!(
            ModelingRelationKind::Generalization
                .validate_endpoints("uml.package", "uml.class")
                .is_err()
        );
        assert!(
            ModelingRelationKind::InterfaceRealization
                .validate_endpoints("uml.class", "uml.component")
                .is_err()
        );
        assert!(
            ModelingRelationKind::TemplateBinding
                .validate_endpoints("uml.class", "uml.component")
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn toolbox_and_quick_edit_reject_final_parents_atomically() -> serde_json::Result<()> {
        let mut model = project(["uml.class"; 4])?;
        for element in &mut model.elements {
            if element.id == "b" {
                element
                    .properties
                    .insert("isFinalSpecialization".into(), serde_json::json!(true));
            }
        }
        rejected_without_mutation(&model, create("a", "b"), "UML_GENERALIZATION_FINAL_PARENT");
        rejected_without_mutation(
            &model,
            Command::QuickEdit {
                diagram_id: "diagram".into(),
                element_id: "b".into(),
                action: UmlQuickCreate::Subtype,
            },
            "UML_GENERALIZATION_FINAL_PARENT",
        );
        assert!(
            apply(
                &model,
                &Command::QuickEdit {
                    diagram_id: "diagram".into(),
                    element_id: "b".into(),
                    action: UmlQuickCreate::Supertype
                },
                0
            )
            .is_ok()
        );
        Ok(())
    }

    #[test]
    fn toolbox_rejects_transitive_cycles_without_rejecting_multiple_inheritance()
    -> serde_json::Result<()> {
        let mut model = project(["uml.class"; 4])?;
        edge(&mut model, "a-b", "a", "b");
        edge(&mut model, "b-c", "b", "c");
        rejected_without_mutation(&model, create("c", "a"), "UML_GENERALIZATION_CYCLE");
        assert!(apply(&model, &create("a", "c"), 0).is_ok());
        Ok(())
    }

    #[test]
    fn new_cycles_are_rejected_even_when_draft_already_contains_another_cycle()
    -> serde_json::Result<()> {
        let mut model = project(["uml.class"; 4])?;
        edge(&mut model, "old-a-b", "a", "b");
        edge(&mut model, "old-b-a", "b", "a");
        edge(&mut model, "old-c-d", "c", "d");
        rejected_without_mutation(&model, create("d", "c"), "UML_GENERALIZATION_CYCLE");
        rejected_without_mutation(&model, create("c", "c"), "UML_GENERALIZATION_CYCLE");
        let result = apply(&model, &create("c", "a"), 0);
        assert!(result.is_ok(), "{result:?}");
        Ok(())
    }

    #[test]
    fn unrelated_draft_diagnostics_do_not_block_valid_new_edges_or_view_and_name_edits()
    -> serde_json::Result<()> {
        let mut model = project(["uml.class", "uml.interface", "uml.class", "uml.class"])?;
        edge(&mut model, "old-invalid-type", "a", "b");
        edge(&mut model, "old-cycle-return", "b", "a");
        for command in [
            create("c", "d"),
            Command::Rename {
                element_id: "a".into(),
                name: "Renamed".into(),
            },
            Command::MoveView {
                view_id: "view-a".into(),
                dx: 10.0,
                dy: 20.0,
            },
        ] {
            let result = apply(&model, &command, 0);
            assert!(result.is_ok(), "{command:?}: {result:?}");
        }
        Ok(())
    }
    #[test]
    fn new_interface_realizations_and_associations_enforce_endpoint_semantics()
    -> serde_json::Result<()> {
        for (kind, source, target, error) in [
            (
                ModelingRelationKind::InterfaceRealization,
                "uml.artifact",
                "uml.interface",
                Some("UML_INTERFACE_REALIZATION_SOURCE"),
            ),
            (
                ModelingRelationKind::InterfaceRealization,
                "uml.component",
                "uml.interface",
                None,
            ),
            (
                ModelingRelationKind::Association,
                "uml.package",
                "uml.class",
                Some("UML_ASSOCIATION_END_TYPE"),
            ),
            (
                ModelingRelationKind::Association,
                "uml.data_type",
                "uml.class",
                None,
            ),
        ] {
            let model = project([source, target, "uml.class", "uml.class"])?;
            let command = Command::CreateRelation {
                diagram_id: "diagram".into(),
                kind,
                from: "a".into(),
                to: "b".into(),
            };
            if let Some(code) = error {
                rejected_without_mutation(&model, command, code);
            } else {
                assert!(apply(&model, &command, 0).is_ok(), "{source} -> {target}");
            }
        }
        Ok(())
    }

    #[test]
    fn new_relationship_warnings_do_not_become_errors() -> serde_json::Result<()> {
        let model = project(["uml.template", "uml.interface", "uml.class", "uml.class"])?;
        let command = Command::CreateRelation {
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::InterfaceRealization,
            from: "a".into(),
            to: "b".into(),
        };
        let updated = apply(&model, &command, 0);
        assert!(updated.is_ok(), "{updated:?}");
        if let Ok(updated) = updated {
            assert!(validate_structural_model(&updated).iter().any(
                |diagnostic| diagnostic.severity == StructuralSeverity::Warning
                    && diagnostic.code == "MDE_RELATIONSHIP_ENDPOINT_UNSUPPORTED"
            ));
        }
        Ok(())
    }

    #[test]
    fn interface_realization_has_no_visibility_without_namespace() -> serde_json::Result<()> {
        let model = project(["uml.class", "uml.interface", "uml.class", "uml.class"])?;
        let command = Command::CreateRelation {
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::InterfaceRealization,
            from: "a".into(),
            to: "b".into(),
        };
        let result = apply(&model, &command, 0);
        assert!(result.is_ok(), "{result:?}");
        let mut updated = result.unwrap();
        let relation = updated.relations.iter_mut()
            .find(|relation| relation.kind == "uml.interface_realization").unwrap();
        assert_eq!(relation.properties.get("visibility"), Some(&serde_json::Value::Null));
        let relation_id = relation.id.clone();
        relation.properties.insert("visibility".into(), serde_json::json!("public"));
        assert!(crate::uml::root::Model::from_project(&updated).validate().iter().any(|issue|
            issue.code == "UML_VISIBILITY_NEEDS_OWNERSHIP"
                && issue.subject_id == relation_id
                && issue.severity == StructuralSeverity::Error));
        Ok(())
    }

    #[test]
    fn existing_invalid_association_does_not_block_unrelated_new_relationships_or_rename()
    -> serde_json::Result<()> {
        let mut model = project(["uml.package", "uml.class", "uml.class", "uml.interface"])?;
        model.relations.push(Relation {
            id: "old-invalid-association".into(),
            kind: "uml.association".into(),
            from: "a".into(),
            to: "b".into(),
            properties: Default::default(),
        });
        for command in [
            Command::CreateRelation {
                diagram_id: "diagram".into(),
                kind: ModelingRelationKind::InterfaceRealization,
                from: "c".into(),
                to: "d".into(),
            },
            Command::Rename {
                element_id: "a".into(),
                name: "Draft package".into(),
            },
        ] {
            let result = apply(&model, &command, 0);
            assert!(result.is_ok(), "{command:?}: {result:?}");
        }
        Ok(())
    }
}
