use super::{Command, ModelingElementKind, apply};
use crate::{Geometry, Project, UmlNotation, uml_header, uml_notation};

fn fixture() -> serde_json::Result<Project> {
    serde_json::from_value(serde_json::json!({
        "id": "project", "name": "Structural creation",
        "elements": [
            {"id": "package", "kind": "uml.package", "name": "Package"},
            {"id": "model", "kind": "uml.model", "name": "Model"},
            {"id": "class", "kind": "uml.class", "name": "Class"}
        ],
        "diagrams": [{"id": "diagram", "name": "Composite", "uml_di_type": "UMLCompositeStructureDiagram", "model_elements": ["class"]}]
    }))
}

fn create(kind: ModelingElementKind, owner: Option<&str>) -> Command {
    Command::CreateElement {
        diagram_id: "diagram".into(),
        owner_id: owner.map(str::to_owned),
        kind,
        name: None,
        x: 20.0,
        y: 40.0,
    }
}

#[test]
fn component_and_artifact_are_package_owned_semantic_objects_with_existing_notation()
-> Result<(), Box<dyn std::error::Error>> {
    let source = fixture()?;
    for (kind, keyword) in [
        (ModelingElementKind::Component, "«component»"),
        (ModelingElementKind::Artifact, "«artifact»"),
    ] {
        assert!(kind.requires_owner());
        for owner in ["package", "model"] {
            let updated = apply(&source, &create(kind, Some(owner)), 0)?;
            let element = updated
                .elements
                .iter()
                .find(|element| element.kind == kind.model_kind())
                .ok_or("created element missing")?;
            assert_eq!(element.name, kind.default_name());
            assert_eq!(
                element.properties.get("visibility"),
                Some(&serde_json::json!("public"))
            );
            assert_eq!(
                element.properties.get("isAbstract"),
                Some(&serde_json::json!(false))
            );
            assert_eq!(
                element.properties.get("isFinalSpecialization"),
                Some(&serde_json::json!(false))
            );
            if kind == ModelingElementKind::Component {
                assert_eq!(
                    element.properties.get("isIndirectlyInstantiated"),
                    Some(&serde_json::json!(true))
                );
                assert_eq!(
                    element.properties.get("isActive"),
                    Some(&serde_json::json!(false))
                );
            } else {
                assert!(!element.properties.contains_key("isIndirectlyInstantiated"));
                assert!(!element.properties.contains_key("isActive"));
            }
            assert_eq!(updated.elements.len(), source.elements.len() + 1);
            let ownership = updated
                .relations
                .iter()
                .find(|relation| relation.to == element.id)
                .ok_or("ownership missing")?;
            assert_eq!(ownership.kind, "owns");
            assert_eq!(ownership.from, owner);
            assert_eq!(updated.relations.len(), 1);
            let diagram = updated.diagrams.first().ok_or("diagram missing")?;
            assert_eq!(diagram.model_elements, vec!["class"]);
            assert_eq!(diagram.views.len(), 1);
            let view = diagram.views.first().ok_or("view missing")?;
            assert_eq!(view.subject.as_deref(), Some(element.id.as_str()));
            assert!(matches!(
                view.geometry,
                Geometry::Rect {
                    x: 20.0,
                    y: 40.0,
                    ..
                }
            ));
            assert_eq!(
                uml_notation(&element.kind),
                UmlNotation::ClassifierRectangle
            );
            assert_eq!(uml_header(&element.kind, None).keyword, Some(keyword));
            assert!(crate::validate_structural_model(&updated).is_empty());
            let repeated = apply(&updated, &create(kind, Some(owner)), 0)?;
            assert_eq!(repeated.relations.len(), 2);
            let identifiers = repeated
                .elements
                .iter()
                .map(|element| &element.id)
                .chain(repeated.relations.iter().map(|relation| &relation.id))
                .chain(
                    repeated
                        .diagrams
                        .iter()
                        .flat_map(|diagram| diagram.views.iter().map(|view| &view.id)),
                )
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(
                identifiers.len(),
                repeated.elements.len() + repeated.relations.len() + 2
            );
        }
    }
    Ok(())
}

#[test]
fn required_owner_is_explicit_resolved_and_supported_not_inferred_from_diagram_context()
-> serde_json::Result<()> {
    let source = fixture()?;
    let before = source.clone();
    for kind in ModelingElementKind::STRUCTURAL.iter().copied() {
        for owner in [None, Some("missing"), Some("class"), Some("diagram")] {
            assert!(apply(&source, &create(kind, owner), 0).is_err());
            assert_eq!(source, before);
        }
    }
    Ok(())
}

#[test]
fn implicit_creation_uses_an_actual_model_without_using_diagram_context() -> Result<(), Box<dyn std::error::Error>>
{
    let source = fixture()?;
    let updated = apply(&source, &create(ModelingElementKind::Class, None), 0)?;
    assert!(!ModelingElementKind::Class.requires_owner());
    assert_eq!(updated.root_model.as_deref(), Some("model"));
    let semantic = crate::uml::root::Model::from_project(&updated);
    let created = updated.elements.last().ok_or("missing class")?;
    assert_eq!(semantic.owner(semantic.element(&created.id).ok_or("missing identity")?)?.map(|owner| owner.id()), Some("model"));
    assert_eq!(
        updated
            .diagrams
            .first()
            .map(|diagram| &diagram.model_elements),
        source
            .diagrams
            .first()
            .map(|diagram| &diagram.model_elements)
    );
    Ok(())
}

#[test]
fn package_and_model_ownership_and_defaults_survive_roundtrip_and_cascade_deletion()
-> Result<(), Box<dyn std::error::Error>> {
    for owner in ["package", "model"] {
        let source = fixture()?;
        let with_component = apply(
            &source,
            &create(ModelingElementKind::Component, Some(owner)),
            0,
        )?;
        let complete = apply(
            &with_component,
            &create(ModelingElementKind::Artifact, Some(owner)),
            1,
        )?;
        for format in [crate::Format::Json, crate::Format::Protobuf] {
            let decoded = crate::decode(&crate::encode(&complete, format)?, format)?;
            assert_eq!(decoded, complete);
            let children = decoded
                .elements
                .iter()
                .filter(|element| matches!(element.kind.as_str(), "uml.component" | "uml.artifact"))
                .map(|element| element.id.clone())
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(children.len(), 2);
            assert_eq!(
                decoded
                    .relations
                    .iter()
                    .filter(|relation| relation.kind == "owns"
                        && relation.from == owner
                        && children.contains(&relation.to))
                    .count(),
                2
            );
            let deleted = apply(
                &decoded,
                &Command::DeleteElement {
                    element_id: owner.into(),
                },
                2,
            )?;
            assert!(deleted.element(owner).is_none());
            assert!(
                deleted
                    .elements
                    .iter()
                    .all(|element| !children.contains(&element.id))
            );
            assert!(
                deleted
                    .relations
                    .iter()
                    .all(|relation| relation.from != owner
                        && !children.contains(&relation.from)
                        && !children.contains(&relation.to))
            );
            assert!(
                deleted
                    .diagrams
                    .iter()
                    .flat_map(|diagram| &diagram.views)
                    .all(|view| view
                        .subject
                        .as_ref()
                        .is_none_or(|subject| !children.contains(subject)))
            );
            let mut expected = source.clone();
            expected.elements.retain(|element| element.id != owner);
            assert_eq!(deleted, expected);
            crate::Graph::new(&deleted).validate()?;
        }
    }
    Ok(())
}


#[test]
fn direct_port_creation_and_semantic_dispatch_materialize_composite_aggregation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut source = fixture()?;
    source.diagrams.first_mut().ok_or("diagram missing")?.views.push(
        serde_json::from_value(serde_json::json!({
            "id": "class-view", "subject": "class",
            "geometry": {"kind":"rect", "x":0.0, "y":0.0, "width":180.0, "height":120.0}
        }))?
    );
    let before = source.clone();
    for command in [
        create(ModelingElementKind::Port, Some("class")),
        Command::CreateElementOrVisualDraft {
            diagram_id: "diagram".into(), owner_id: Some("class".into()),
            kind: ModelingElementKind::Port, name: None, x: 20.0, y: 40.0,
        },
    ] {
        let updated = apply(&source, &command, 0)?;
        let port = updated.elements.iter().find(|element| element.kind == "uml.port")
            .ok_or("semantic Port missing; valid creation must not silently become a visual draft")?;
        assert_eq!(port.properties.get("aggregation"), Some(&serde_json::json!("composite")));
        assert!(updated.relations.iter().any(|relation|
            relation.kind == "owns" && relation.from == "class" && relation.to == port.id));
        let view = updated.diagrams.iter().flat_map(|diagram| &diagram.views)
            .find(|view| view.subject.as_deref() == Some(port.id.as_str())).ok_or("Port View missing")?;
        assert!(view.draft.is_none());
        assert!(!crate::validate_structural_model(&updated).iter().any(|issue|
            issue.code == "UML_PORT_AGGREGATION"));
        assert_eq!(source, before);
    }
    Ok(())
}

#[test]
fn port_visual_fallback_preserves_intent_without_claiming_semantic_creation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut source = fixture()?;
    source.diagrams.first_mut().ok_or("diagram missing")?.views.push(
        serde_json::from_value(serde_json::json!({
            "id": "package-view", "subject": "package",
            "geometry": {"kind":"rect", "x":0.0, "y":0.0, "width":180.0, "height":120.0}
        }))?
    );
    let direct = apply(&source, &create(ModelingElementKind::Port, Some("package")), 0);
    assert!(matches!(direct, Err(ref error) if error.contains("no supported UML composite ownership property")), "{direct:?}");
    let updated = apply(&source, &Command::CreateElementOrVisualDraft {
        diagram_id: "diagram".into(), owner_id: Some("package".into()),
        kind: ModelingElementKind::Port, name: None, x: 20.0, y: 40.0,
    }, 0)?;
    assert_eq!(updated.elements, source.elements);
    assert_eq!(updated.relations, source.relations);
    let view = updated.diagrams.iter().flat_map(|diagram| &diagram.views)
        .find(|view| view.draft.is_some()).ok_or("visual fallback missing")?;
    assert!(view.subject.is_none());
    let draft = view.draft.as_ref().ok_or("draft missing")?;
    assert_eq!(draft.notation_kind, "uml.port");
    assert_eq!(draft.properties.get("aggregation"), Some(&serde_json::json!("composite")));
    assert!(draft.properties.get("semantic_creation_diagnostic").and_then(serde_json::Value::as_str)
        .is_some_and(|reason| reason.contains("no supported UML composite ownership property")));
    Ok(())
}
