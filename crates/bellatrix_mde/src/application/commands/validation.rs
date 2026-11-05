#[cfg(test)]
mod tests {
    use crate::{
        ModelingRelationKind, Project, Relation,
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
            owner_id: None,
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::Generalization,
            from: from.into(),
            to: to.into(),
        }
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
    fn interface_realization_has_no_visibility_without_namespace() -> serde_json::Result<()> {
        let model = project(["uml.class", "uml.interface", "uml.class", "uml.class"])?;
        let command = Command::CreateRelation {
            owner_id: None,
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::InterfaceRealization,
            from: "a".into(),
            to: "b".into(),
        };
        let result = apply(&model, &command, 0);
        assert!(result.is_ok(), "{result:?}");
        let updated = result.unwrap();
        let relation = updated
            .relations
            .iter()
            .find(|relation| relation.kind == "uml.interface_realization")
            .unwrap();
        assert_eq!(
            relation.properties.get("visibility"),
            Some(&serde_json::Value::Null)
        );
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
                owner_id: None,
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
