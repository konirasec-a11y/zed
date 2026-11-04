use bellatrix_mde::commands::{self, Command};
use bellatrix_mde::{Project, UmlCapability, UmlQuickCreate, UmlType, quick_edit_for};

fn project(kind: &str) -> Project {
    serde_json::from_value(serde_json::json!({"id":"inheritance", "name":"Inheritance", "elements":[{"id":"owner", "name":"Owner", "kind":kind}], "diagrams":[{"id":"diagram", "name":"D", "uml_di_type":"UMLClassDiagram", "views":[{"id":"view", "subject":"owner", "geometry":{"kind":"rect", "x":0.0,"y":0.0,"width":180.0,"height":120.0}}]}]})).expect("fixture")
}

#[test]
fn inherited_interface_creation_uses_behaviored_classifier_contract() {
    for (kind, expected) in [
        ("uml.component", true),
        ("uml.node", true),
        ("uml.collaboration", true),
        ("uml.artifact", false),
        ("uml.signal", false),
        ("uml.package", false),
        ("uml.interface", false),
    ] {
        for (action, id, relation_kind) in [
            (
                UmlQuickCreate::ProvidedInterface,
                "uml.add-provided-interface",
                "uml.interface_realization",
            ),
            (
                UmlQuickCreate::RequiredInterface,
                "uml.add-required-interface",
                "uml.usage",
            ),
        ] {
            assert_eq!(
                UmlType::from_kind(kind)
                    .expect("kind")
                    .supports(UmlCapability::ClassInterfaces),
                expected,
                "{kind}"
            );
            assert_eq!(
                quick_edit_for(kind).iter().any(|entry| entry.id == id),
                expected,
                "{kind}: {id}"
            );
            let original = project(kind);
            let result = commands::apply(
                &original,
                &Command::QuickEdit {
                    diagram_id: "diagram".into(),
                    element_id: "owner".into(),
                    action,
                },
                0,
            );
            assert_eq!(result.is_ok(), expected, "{kind} {action:?}: {result:?}");
            if let Ok(updated) = result {
                let relation = updated
                    .relations
                    .iter()
                    .find(|relation| relation.kind == relation_kind)
                    .expect("typed relationship");
                assert_eq!(relation.from, "owner");
                if action == UmlQuickCreate::RequiredInterface {
                    let presentation = bellatrix_mde::relationship_presentation(
                        &bellatrix_mde::ModelingSection::from_model(&updated),
                        relation,
                    );
                    assert_eq!(presentation.name.as_deref(), Some("«use»"));
                    assert_eq!(presentation.source_role, "client");
                    assert_eq!(presentation.target_role, "supplier");
                    assert_eq!(
                        presentation.style.line_mode,
                        bellatrix_mde::LineMode::Dashed
                    );
                    assert_eq!(
                        presentation.style.target_marker,
                        bellatrix_mde::RelationshipMarker::OpenArrow
                    );
                    assert_eq!(
                        bellatrix_mde::relationship_root_kind(relation),
                        Some(bellatrix_mde::RootMetaclass::DirectedRelationship)
                    );
                    let selected = updated
                        .elements
                        .iter()
                        .map(|element| element.id.clone())
                        .collect();
                    let generation = bellatrix_mde::analyze_generation_graph(&updated, &selected)
                        .expect("usage ordering");
                    let owner = generation
                        .units
                        .iter()
                        .find(|unit| unit.id == "owner")
                        .expect("owner generation unit");
                    assert!(
                        owner
                            .requires
                            .iter()
                            .any(|dependency| dependency.id == relation.to)
                    );
                }
                assert!(
                    updated
                        .elements
                        .iter()
                        .any(|element| element.id == relation.to && element.kind == "rust.trait")
                );
            }
            assert_eq!(original, project(kind));
        }
    }
}

#[test]
fn specialization_preserves_subject_metaclass_and_respects_final_supertype() {
    for (kind, expected) in [
        ("uml.component", true),
        ("uml.artifact", true),
        ("uml.signal", true),
        ("uml.collaboration", true),
        ("uml.package", false),
        ("uml.model", false),
    ] {
        for action in [UmlQuickCreate::Subtype, UmlQuickCreate::Supertype] {
            let original = project(kind);
            let result = commands::apply(
                &original,
                &Command::QuickEdit {
                    diagram_id: "diagram".into(),
                    element_id: "owner".into(),
                    action,
                },
                0,
            );
            assert_eq!(result.is_ok(), expected, "{kind} {action:?}: {result:?}");
            if let Ok(updated) = result {
                let child = updated
                    .elements
                    .iter()
                    .find(|element| element.id != "owner" && element.kind == kind)
                    .expect("child");
                assert_eq!(child.kind, kind);
                let relation = updated
                    .relations
                    .iter()
                    .find(|relation| relation.kind == "uml.generalization")
                    .expect("generalization");
                assert_eq!(
                    relation.from,
                    if action == UmlQuickCreate::Subtype {
                        child.id.as_str()
                    } else {
                        "owner"
                    }
                );
                assert_eq!(
                    relation.to,
                    if action == UmlQuickCreate::Subtype {
                        "owner"
                    } else {
                        child.id.as_str()
                    }
                );
            }
        }
    }
    let mut final_component = project("uml.component");
    final_component
        .elements
        .first_mut()
        .expect("owner")
        .properties
        .insert("isFinalSpecialization".into(), serde_json::json!(true));
    assert!(
        commands::apply(
            &final_component,
            &Command::QuickEdit {
                diagram_id: "diagram".into(),
                element_id: "owner".into(),
                action: UmlQuickCreate::Subtype
            },
            0
        )
        .is_err()
    );
}

#[test]
fn association_quick_create_uses_typed_member_ends_not_class_whitelist() {
    for (kind, expected) in [
        ("uml.component", true),
        ("uml.artifact", true),
        ("uml.signal", true),
        ("uml.interface", true),
        ("uml.package", false),
        ("uml.model", false),
    ] {
        for (action, aggregation) in [
            (UmlQuickCreate::AssociatedClass, None),
            (UmlQuickCreate::AggregatedClass, Some("shared")),
            (UmlQuickCreate::CompositedClass, Some("composite")),
        ] {
            let original = project(kind);
            let result = commands::apply(
                &original,
                &Command::QuickEdit {
                    diagram_id: "diagram".into(),
                    element_id: "owner".into(),
                    action,
                },
                0,
            );
            assert_eq!(result.is_ok(), expected, "{kind} {action:?}: {result:?}");
            if let Ok(updated) = result {
                let relation = updated
                    .relations
                    .iter()
                    .find(|relation| relation.kind == "uml.association")
                    .expect("association");
                assert_eq!(relation.from, "owner");
                let references = bellatrix_mde::uml::references::ReferenceIndex::new(
                    bellatrix_mde::uml::root::Model::from_project(&updated),
                );
                let ends = bellatrix_mde::uml::associations::member_ends(&references, &relation.id)
                    .expect("canonical ends");
                assert_eq!(
                    ends.get(1)
                        .and_then(|end| end.properties.get("aggregation"))
                        .and_then(|value| value.as_str()),
                    aggregation
                );
            }
        }
    }
}
