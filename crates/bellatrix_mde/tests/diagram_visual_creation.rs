use bellatrix_mde::ModelingRelationKind;
use bellatrix_mde::commands::{Command, ModelingElementKind, apply};
use bellatrix_mde::{Format, Project, StructuralDiagramKind, decode, encode};

fn fixture() -> serde_json::Result<Project> {
    serde_json::from_str(include_str!("../fixtures/todo.json"))
}

fn create(
    project: &Project,
    kind: ModelingElementKind,
    owner: Option<&str>,
) -> Result<Project, String> {
    apply(
        project,
        &Command::CreateElement {
            diagram_id: "diagram-tasks".into(),
            owner_id: owner.map(str::to_owned),
            kind,
            name: None,
            x: 100.0,
            y: 100.0,
        },
        0,
    ).map_err(|error| format!("create {kind:?}: {error}"))
}

fn connect(
    project: &Project,
    kind: ModelingRelationKind,
    from: &str,
    to: &str,
) -> Result<Project, String> {
    apply(
        project,
        &Command::CreateRelation {
            diagram_id: "diagram-tasks".into(),
            kind,
            from: from.into(),
            to: to.into(),
        },
        0,
    ).map_err(|error| format!("connect {kind:?}: {error}"))
}

#[test]
fn fourteen_presentations_preserve_normative_diagram_identity_and_context()
-> Result<(), Box<dyn std::error::Error>> {
    for kind in StructuralDiagramKind::ALL_DIAGRAMS {
        let mut source = fixture()?;
        let context = if let Some(expected) = kind.context_metaclass() {
            let tool = match expected {
                "Activity" => ModelingElementKind::Activity,
                "StateMachine" => ModelingElementKind::StateMachine,
                "Interaction" => ModelingElementKind::Interaction,
                _ => return Err("unexpected context".into()),
            };
            source = create(&source, tool, None)?;
            vec![source.elements.last().ok_or("missing context")?.id.clone()]
        } else if kind == StructuralDiagramKind::CompositeStructure {
            vec!["type-list".into()]
        } else {
            Vec::new()
        };
        let result = apply(
            &source,
            &Command::CreateDiagram {
                kind,
                name: "Visual diagram".into(),
                model_elements: context.clone(),
            },
            0,
        )?;
        let diagram = result.diagrams.last().ok_or("missing diagram")?;
        assert_eq!(StructuralDiagramKind::from_diagram(diagram), Some(kind));
        assert_eq!(diagram.model_elements, context);
        if let Some(presentation) = kind.interaction_kind() {
            assert_eq!(diagram.uml_di_type, "UMLInteractionDiagram");
            assert_eq!(diagram.interaction_kind.as_deref(), Some(presentation));
        }
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(decode(&encode(&result, format)?, format)?, result);
        }
        if kind.context_metaclass().is_some() {
            let draft = apply(
                &source,
                &Command::CreateDiagram {
                    kind,
                    name: "Unverified".into(),
                    model_elements: Vec::new(),
                },
                1,
            )?;
            let diagram = draft.diagrams.last().ok_or("missing diagram")?;
            assert_eq!(diagram.uml_di_type, kind.uml_di_type());
            assert!(
                bellatrix_mde::validate_structural_diagram(&draft, &diagram.id, kind)
                    .iter()
                    .any(|diagnostic| diagnostic.code == "UMLDI_CONTEXT_MULTIPLICITY")
            );
        }
    }
    exercise_visual_drafts()?;
    Ok(())
}

#[test]
fn behavior_nodes_use_canonical_composite_owners_and_transactional_commands()
-> Result<(), Box<dyn std::error::Error>> {
    let source = fixture()?;
    for (root, children) in [
        (
            ModelingElementKind::Activity,
            ModelingElementKind::ACTIVITY_NODES,
        ),
        (
            ModelingElementKind::Interaction,
            ModelingElementKind::INTERACTION_NODES,
        ),
    ] {
        let container = create(&source, root, None)?;
        let owner = container.elements.last().ok_or("missing owner")?.id.clone();
        for child in children {
            let result = create(&container, *child, Some(&owner))?;
            let element = result.elements.last().ok_or("missing child")?;
            assert_eq!(element.kind, child.model_kind());
            assert!(
                result
                    .relations
                    .iter()
                    .any(|relation| relation.kind == "owns"
                        && relation.from == owner
                        && relation.to == element.id)
            );
            assert!(create(&container, *child, None).is_err());
            assert!(create(&container, *child, Some("type-list")).is_err());
            assert_eq!(
                decode(&encode(&result, Format::Protobuf)?, Format::Protobuf)?,
                result
            );
        }
    }
    let namespace = create(&source, ModelingElementKind::Package, None)?;
    let namespace_id = namespace
        .elements
        .last()
        .ok_or("missing namespace")?
        .id
        .clone();
    let imported = connect(
        &namespace,
        ModelingRelationKind::ElementImport,
        &namespace_id,
        "type-list",
    )?;
    let import = imported
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.element_import")
        .ok_or("missing canonical ElementImport")?;
    let model = bellatrix_mde::uml::root::Model::from_project(&imported);
    assert_eq!(
        model
            .owner(model.element(&import.id).ok_or("missing import identity")?)?
            .map(|owner| owner.id().to_owned()),
        Some(namespace_id)
    );
    let machine = create(&source, ModelingElementKind::StateMachine, None)?;
    let machine_id = machine.elements.last().ok_or("missing machine")?.id.clone();
    let region = create(&machine, ModelingElementKind::Region, Some(&machine_id))?;
    let region_id = region.elements.last().ok_or("missing region")?.id.clone();
    for child in [
        ModelingElementKind::State,
        ModelingElementKind::Pseudostate,
        ModelingElementKind::FinalState,
    ] {
        let result = create(&region, child, Some(&region_id))?;
        let element = result.elements.last().ok_or("missing state")?;
        assert_eq!(element.kind, child.model_kind());
        if child == ModelingElementKind::Pseudostate {
            assert_eq!(
                element.properties.get("kind"),
                Some(&serde_json::json!("initial"))
            );
        }
        assert!(create(&region, child, Some(&machine_id)).is_err());
    }
    let mut graph = create(&source, ModelingElementKind::Activity, None)?;
    let activity_id = graph.elements.last().ok_or("missing activity")?.id.clone();
    graph = create(
        &graph,
        ModelingElementKind::OpaqueAction,
        Some(&activity_id),
    )?;
    let source_node = graph.elements.last().ok_or("missing action")?.id.clone();
    graph = create(
        &graph,
        ModelingElementKind::OpaqueAction,
        Some(&activity_id),
    )?;
    let target_node = graph.elements.last().ok_or("missing action")?.id.clone();
    for kind in [
        ModelingRelationKind::ControlFlow,
        ModelingRelationKind::ObjectFlow,
    ] {
        let linked = connect(&graph, kind, &source_node, &target_node)?;
        let edge = linked
            .relations
            .iter()
            .find(|edge| edge.kind == kind.model_kind())
            .ok_or("missing flow")?;
        assert!(
            linked.relations.iter().any(|owner| owner.kind == "owns"
                && owner.from == activity_id
                && owner.to == edge.id)
        );
        assert_eq!(
            bellatrix_mde::relationship_presentation(edge)
                .style
                .target_marker,
            bellatrix_mde::RelationshipMarker::OpenArrow
        );
        assert!(connect(&graph, kind, &source_node, "type-list").is_err());
    }
    for (left, right, kinds, owner_is_target) in [
        (
            ModelingElementKind::UseCase,
            ModelingElementKind::UseCase,
            &[ModelingRelationKind::Include, ModelingRelationKind::Extend][..],
            false,
        ),
        (
            ModelingElementKind::Package,
            ModelingElementKind::Package,
            &[
                ModelingRelationKind::PackageImport,
                ModelingRelationKind::PackageMerge,
            ][..],
            false,
        ),
        (
            ModelingElementKind::Artifact,
            ModelingElementKind::Node,
            &[ModelingRelationKind::Deployment][..],
            true,
        ),
        (
            ModelingElementKind::Artifact,
            ModelingElementKind::Class,
            &[ModelingRelationKind::Manifestation][..],
            false,
        ),
    ] {
        let mut graph = create(&source, ModelingElementKind::Package, None)?;
        let package_id = graph.elements.last().ok_or("missing package")?.id.clone();
        graph = create(&graph, left, Some(&package_id))?;
        let left_id = graph.elements.last().ok_or("missing source")?.id.clone();
        graph = create(&graph, right, Some(&package_id))?;
        let right_id = graph.elements.last().ok_or("missing target")?.id.clone();
        for kind in kinds {
            let linked = connect(&graph, *kind, &left_id, &right_id)?;
            let edge = linked
                .relations
                .iter()
                .find(|edge| edge.kind == kind.model_kind())
                .ok_or("missing relationship")?;
            let owner_id = if owner_is_target { &right_id } else { &left_id };
            let semantic_model = bellatrix_mde::uml::root::Model::from_project(&linked);
            let relationship = semantic_model
                .element(&edge.id)
                .ok_or("missing semantic relationship")?;
            assert_eq!(
                semantic_model.owner(relationship)?.map(|owner| owner.id()),
                Some(owner_id.as_str())
            );
            if matches!(kind, ModelingRelationKind::Deployment | ModelingRelationKind::Manifestation) {
                assert_eq!(edge.properties.get("visibility"), Some(&serde_json::Value::Null));
                let named = relationship.named().ok_or("missing NamedElement")?;
                assert!(semantic_model.namespace(named)?.is_none());
            }
            let presentation = bellatrix_mde::relationship_presentation(edge);
            assert_eq!(presentation.style.line_mode, bellatrix_mde::LineMode::Dashed);
            assert!(
                presentation
                    .name
                    .as_deref()
                    .is_some_and(|name| name.starts_with('«'))
            );
            assert_eq!(
                decode(&encode(&linked, Format::Protobuf)?, Format::Protobuf)?,
                linked
            );
        }
    }
    for pin in [
        ModelingElementKind::InputPin,
        ModelingElementKind::OutputPin,
        ModelingElementKind::ValuePin,
    ] {
        let pinned = create(&graph, pin, Some(&source_node))?;
        let pin_id = pinned.elements.last().ok_or("missing pin")?.id.clone();
        let pin_view = pinned
            .diagrams
            .iter()
            .flat_map(|diagram| &diagram.views)
            .find(|view| view.subject.as_deref() == Some(&pin_id))
            .ok_or("missing pin view")?;
        let before = pin_view.geometry.clone();
        let action_view = pinned
            .diagrams
            .iter()
            .flat_map(|diagram| &diagram.views)
            .find(|view| view.subject.as_deref() == Some(&source_node))
            .ok_or("missing action view")?
            .id
            .clone();
        let moved = apply(
            &pinned,
            &Command::MoveView {
                view_id: action_view,
                dx: 50.0,
                dy: 30.0,
            },
            0,
        )?;
        let after = &moved
            .diagrams
            .iter()
            .flat_map(|diagram| &diagram.views)
            .find(|view| view.id == pin_view.id)
            .ok_or("missing moved pin")?
            .geometry;
        let mut expected = before;
        expected.translate(50.0, 30.0);
        assert_eq!(after, &expected);
        assert!(create(&graph, pin, Some(&activity_id)).is_err());
    }
    let interaction = create(&source, ModelingElementKind::Interaction, None)?;
    let interaction_id = interaction
        .elements
        .last()
        .ok_or("missing interaction")?
        .id
        .clone();
    let fragment = create(
        &interaction,
        ModelingElementKind::CombinedFragment,
        Some(&interaction_id),
    )?;
    let fragment_id = fragment
        .elements
        .last()
        .ok_or("missing fragment")?
        .id
        .clone();
    let one = create(
        &fragment,
        ModelingElementKind::InteractionOperand,
        Some(&fragment_id),
    )?;
    let one_id = one.elements.last().ok_or("missing operand")?.id.clone();
    let two = create(
        &one,
        ModelingElementKind::InteractionOperand,
        Some(&fragment_id),
    )?;
    let two_id = two.elements.last().ok_or("missing operand")?.id.clone();
    let views = two
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .filter(|view| {
            view.subject
                .as_deref()
                .is_some_and(|id| id == one_id || id == two_id)
        })
        .collect::<Vec<_>>();
    assert_eq!(views.len(), 2);
    assert_ne!(views[0].geometry, views[1].geometry);
    assert!(
        create(
            &interaction,
            ModelingElementKind::InteractionOperand,
            Some(&interaction_id)
        )
        .is_err()
    );
    assert_eq!(
        decode(&encode(&two, Format::Protobuf)?, Format::Protobuf)?,
        two
    );
    Ok(())
}

fn exercise_visual_drafts() -> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::{Geometry, StructuralSeverity, validate_structural_diagram};
    let before = fixture()?;
    // Exercise the same shared creation paths used by the Class and Association quick tools.
    let class = apply(&before, &Command::CreateElementOrVisualDraft {
        diagram_id: "diagram-tasks".into(), owner_id: None, kind: ModelingElementKind::Class,
        name: Some("Quick class".into()), x: 80.0, y: 90.0,
    }, 30)?;
    let quick_view = class.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last()).ok_or("quick class View missing")?;
    assert!(bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&class))
        .iter().any(|component| component.diagram_id == "diagram-tasks" && component.view_id == quick_view.id));
    let association = apply(&class, &Command::CreateRelationOrVisualDraft {
        diagram_id: "diagram-tasks".into(), kind: bellatrix_mde::ModelingRelationKind::Association,
        from: "type-list".into(), to: "type-task".into(),
    }, 31)?;
    let quick_edge = association.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last()).ok_or("quick association View missing")?;
    assert!(matches!(&quick_edge.geometry, Geometry::Path { segments, .. } if !segments.is_empty()));
    assert!(bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&association))
        .iter().any(|component| component.diagram_id == "diagram-tasks" && component.view_id == quick_edge.id));
    let fallback = apply(
        &before,
        &Command::CreateElementOrVisualDraft {
            diagram_id: "diagram-tasks".into(),
            owner_id: None,
            kind: ModelingElementKind::State,
            name: Some("Waiting".into()),
            x: 10.0,
            y: 20.0,
        },
        0,
    )?;
    assert_eq!(fallback.elements, before.elements);
    assert!(
        fallback
            .diagrams
            .iter()
            .flat_map(|diagram| &diagram.views)
            .any(|view| view.draft.as_ref().is_some_and(|draft| draft
                .properties
                .contains_key("semantic_creation_diagnostic")))
    );
    let fallback_inventory = bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&fallback));
    assert!(fallback_inventory.iter().any(|view| view.diagram_id == "diagram-tasks" && view.subject_id.is_none() && view.label == "Waiting"));
    let after = apply(
        &before,
        &Command::CreateVisualDraft {
            diagram_id: "diagram-tasks".into(),
            notation_kind: "State".into(),
            label: Some("Waiting".into()),
            geometry: Geometry::Rect {
                x: 10.0,
                y: 20.0,
                width: 120.0,
                height: 80.0,
            },
            properties: Default::default(),
            parent_view_id: None,
            source_view_id: None,
            target_view_id: None,
        },
        1,
    )?;
    assert_eq!(after.elements, before.elements);
    let diagram = after
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "diagram-tasks")
        .ok_or("missing diagram")?;
    let draft = diagram.views.last().ok_or("missing draft")?;
    assert!(draft.subject.is_none());
    assert_eq!(
        draft
            .draft
            .as_ref()
            .map(|metadata| metadata.notation_kind.as_str()),
        Some("State")
    );
    let kind = StructuralDiagramKind::from_diagram(diagram).ok_or("missing diagram type")?;
    assert!(
        validate_structural_diagram(&after, &diagram.id, kind)
            .iter()
            .any(|issue| issue.code == "VISUAL_DRAFT_UNCHECKED"
                && issue.subject_id == draft.id
                && issue.severity == StructuralSeverity::Warning)
    );
    let draft_id = draft.id.clone();
    let inventory = bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&after));
    assert!(inventory.iter().any(|view| view.diagram_id == "diagram-tasks" && view.view_id == draft_id && view.subject_id.is_none() && view.label == "Waiting"));
    assert!(inventory.iter().any(|view| view.diagram_id == "diagram-tasks" && view.view_id == "view-task" && view.subject_id.as_deref() == Some("type-task")));
    assert_eq!(inventory.len(), after.diagrams.iter().map(|diagram| diagram.views.len()).sum::<usize>());
    let duplicate = apply(&after, &Command::PlaceElementView {
        diagram_id: "diagram-tasks".into(), element_id: "type-task".into(), x: 350.0, y: 150.0,
    }, 2)?;
    let inventory = bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&duplicate));
    assert_eq!(inventory.iter().filter(|view| view.subject_id.as_deref() == Some("type-task") && view.diagram_id == "diagram-tasks").count(), 2);
    // The clicked View identities, not the first View of each subject, drive attachments.
    let repeated_id = duplicate.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last()).ok_or("repeated View missing")?.id.clone();
    let make_link = |source: &str, target: &str, from: Option<&str>, to: Option<&str>| Command::CreateRelationAtViewsOrVisualDraft {
        diagram_id: "diagram-tasks".into(), kind: bellatrix_mde::ModelingRelationKind::Association,
        source_view_id: source.into(), target_view_id: target.into(), from: from.map(str::to_owned), to: to.map(str::to_owned),
    };
    let mixed = apply(&duplicate, &make_link(&repeated_id, &draft_id, Some("type-task"), None), 40)?;
    assert_eq!(mixed.relations, duplicate.relations);
    let mixed_edge = mixed.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last()).ok_or("mixed edge missing")?;
    let mixed_edge_id = mixed_edge.id.clone();
    let ends = mixed_edge.endpoints.as_ref().ok_or("edge endpoints missing")?;
    assert_eq!(ends.source_view_id.as_deref(), Some(repeated_id.as_str()));
    assert_eq!(ends.target_view_id.as_deref(), Some(draft_id.as_str()));
    let reverse = apply(&mixed, &make_link(&draft_id, &repeated_id, None, Some("type-task")), 41)?;
    let reverse_edge = reverse.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last()).ok_or("reverse edge missing")?;
    assert_eq!(reverse_edge.endpoints.as_ref().and_then(|ends| ends.source_view_id.as_deref()), Some(draft_id.as_str()));
    assert_eq!(reverse_edge.endpoints.as_ref().and_then(|ends| ends.target_view_id.as_deref()), Some(repeated_id.as_str()));
    assert!(apply(&mixed, &make_link("missing", &draft_id, Some("type-task"), None), 42).is_err());
    assert!(apply(&mixed, &make_link(&repeated_id, &draft_id, Some("type-list"), None), 42).is_err());
    let moved_other = apply(&mixed, &Command::MoveView { view_id: "view-task".into(), dx: 17.0, dy: 5.0 }, 42)?;
    let edge = |project: &Project| project.diagrams.iter().flat_map(|diagram| &diagram.views)
        .find(|view| view.id == mixed_edge_id).map(|view| view.geometry.clone());
    assert_eq!(edge(&mixed), edge(&moved_other));
    let moved_selected = apply(&mixed, &Command::MoveView { view_id: repeated_id.clone(), dx: 17.0, dy: 5.0 }, 43)?;
    assert_ne!(edge(&mixed), edge(&moved_selected));
    let retargeted = apply(&mixed, &Command::SetViewEndpoints {
        diagram_id: "diagram-tasks".into(), view_id: mixed_edge_id.clone(),
        endpoints: bellatrix_mde::ViewEndpoints { source_view_id: Some("view-task".into()), target_view_id: Some(draft_id.clone()) },
    }, 44)?;
    assert_eq!(retargeted.diagrams.iter().flat_map(|diagram| &diagram.views).find(|view| view.id == mixed_edge_id)
        .and_then(|view| view.endpoints.as_ref()).and_then(|ends| ends.source_view_id.as_deref()), Some("view-task"));
    for format in [Format::Json, Format::Protobuf] {
        assert_eq!(decode(&encode(&retargeted, format)?, format)?, retargeted);
    }
    let canonical = apply(&class, &Command::CreateElement {
        diagram_id: "diagram-tasks".into(), owner_id: None, kind: ModelingElementKind::Class,
        name: Some("Other class".into()), x: 450.0, y: 90.0,
    }, 50)?;
    let source_subject = quick_view.subject.as_deref().ok_or("canonical source subject missing")?;
    let target_subject = canonical.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last()).and_then(|view| view.subject.as_deref()).ok_or("canonical target subject missing")?.to_owned();
    let canonical = apply(&canonical, &Command::PlaceElementView { diagram_id: "diagram-tasks".into(), element_id: source_subject.into(), x: 100.0, y: 450.0 }, 51)?;
    let source_copy = canonical.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks").and_then(|diagram| diagram.views.last()).ok_or("source copy missing")?.id.clone();
    let canonical = apply(&canonical, &Command::PlaceElementView { diagram_id: "diagram-tasks".into(), element_id: target_subject.clone(), x: 550.0, y: 450.0 }, 52)?;
    let target_copy = canonical.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks").and_then(|diagram| diagram.views.last()).ok_or("target copy missing")?.id.clone();
    let initial_associations = canonical.relations.iter().filter(|relation| relation.kind == "uml.association").count();
    let canonical = apply(&canonical, &make_link(&source_copy, &target_copy, Some(source_subject), Some(&target_subject)), 53)?;
    let canonical_edge = canonical.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks").and_then(|diagram| diagram.views.last()).ok_or("canonical edge missing")?;
    assert!(canonical_edge.draft.is_none());
    let canonical_edge_id = canonical_edge.id.clone();
    let relation_id = canonical_edge.subject.clone().ok_or("canonical relation missing")?;
    let relation = canonical.relations.iter().find(|relation| relation.id == relation_id).ok_or("relation missing")?;
    assert_eq!((&relation.from, &relation.to), (&source_subject.to_owned(), &target_subject));
    assert_eq!(canonical.relations.iter().filter(|relation| relation.kind == "uml.association").count(), initial_associations + 1);
    assert_eq!(canonical_edge.endpoints.as_ref(), Some(&bellatrix_mde::ViewEndpoints { source_view_id: Some(source_copy.clone()), target_view_id: Some(target_copy.clone()) }));
    let canonical_path = |project: &Project| {
        let section = bellatrix_mde::ModelingSection::from_model(project);
        bellatrix_mde::RelationshipGraph::new(&section, "diagram-tasks").view_path(&canonical_edge_id)
    };
    assert_eq!(canonical_path(&canonical), Some(canonical_edge.geometry.clone()));
    let unselected_moved = apply(&canonical, &Command::MoveView { view_id: quick_view.id.clone(), dx: 70.0, dy: 20.0 }, 54)?;
    assert_eq!(canonical_path(&canonical), canonical_path(&unselected_moved));
    let selected_moved = apply(&canonical, &Command::MoveView { view_id: source_copy.clone(), dx: 70.0, dy: 20.0 }, 55)?;
    assert_ne!(canonical_path(&canonical), canonical_path(&selected_moved));
    // Two unnamed Associations collide in their owning Namespace. Preserve the
    // fallback check, then distinguish the first relation for the endpoint test.
    let conflicting = apply(&canonical, &make_link(&target_copy, &source_copy, Some(&target_subject), Some(source_subject)), 56)?;
    let conflict = conflicting.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last()).and_then(|view| view.draft.as_ref()).ok_or("expected conflicting relation draft")?;
    assert!(conflict.properties.get("semantic_creation_diagnostic").and_then(serde_json::Value::as_str)
        .is_some_and(|message| message.starts_with("UML_NAMESPACE_MEMBERS_DISTINGUISHABLE:")));
    assert_eq!(conflicting.relations, canonical.relations);
    let canonical = apply(&canonical, &Command::SetProperty {
        element_id: relation_id.clone(), path: "properties.name".into(), value: serde_json::json!("Forward association"),
    }, 56)?;
    let canonical_reverse = apply(&canonical, &make_link(&target_copy, &source_copy, Some(&target_subject), Some(source_subject)), 56)?;
    let reverse_view = canonical_reverse.diagrams.iter().find(|diagram| diagram.id == "diagram-tasks").and_then(|diagram| diagram.views.last()).ok_or("reverse canonical edge missing")?;
    assert!(reverse_view.draft.is_none());
    assert_eq!(reverse_view.endpoints.as_ref(), Some(&bellatrix_mde::ViewEndpoints { source_view_id: Some(target_copy.clone()), target_view_id: Some(source_copy.clone()) }));
    let retargeted_canonical = apply(&canonical, &Command::SetViewEndpoints {
        diagram_id: "diagram-tasks".into(), view_id: canonical_edge_id.clone(),
        endpoints: bellatrix_mde::ViewEndpoints { source_view_id: Some(quick_view.id.clone()), target_view_id: Some(target_copy.clone()) },
    }, 57)?;
    assert_eq!(retargeted_canonical.relations, canonical.relations);
    assert_ne!(canonical_path(&retargeted_canonical), canonical_path(&canonical));
    let mut missing_selected = canonical.clone();
    for diagram in &mut missing_selected.diagrams { diagram.views.retain(|view| view.id != target_copy); }
    assert!(canonical_path(&missing_selected).is_none());
    assert!(apply(&missing_selected, &make_link(&source_copy, &target_copy, Some(source_subject), Some(&target_subject)), 58).is_err());
    for format in [Format::Json, Format::Protobuf] {
        assert_eq!(decode(&encode(&retargeted_canonical, format)?, format)?, retargeted_canonical);
    }
    let legacy: bellatrix_mde::View = serde_json::from_value(serde_json::json!({
        "id": "legacy-edge", "subject": null,
        "geometry": {"kind":"path", "start":{"x":0.0,"y":0.0}, "segments":[]},
        "draft": {"notation_kind":"uml.association", "source_view_id":"old-source", "target_view_id":"old-target"}
    }))?;
    assert_eq!(legacy.endpoints.as_ref().and_then(|ends| ends.source_view_id.as_deref()), Some("old-source"));
    let normalized = serde_json::to_value(&legacy)?;
    assert!(normalized["draft"].get("source_view_id").is_none());
    assert_eq!(normalized["endpoints"]["target_view_id"], "old-target");
    // This authored tag-7 draft fixture predates the tag-9 View endpoint field.
    let old_wire = decode(include_bytes!("../fixtures/draft-endpoints-v1.blx"), Format::Protobuf)?;
    let migrated = old_wire.diagrams.first().and_then(|diagram| diagram.views.first()).ok_or("old wire View missing")?;
    assert_eq!(migrated, &legacy);
    for format in [Format::Json, Format::Protobuf] {
        assert_eq!(decode(&encode(&old_wire, format)?, format)?, old_wire);
    }
    let other_diagram = apply(&duplicate, &Command::PlaceElementView {
        diagram_id: "diagram-overview".into(), element_id: "type-task".into(), x: 20.0, y: 30.0,
    }, 3)?;
    let inventory = bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&other_diagram));
    assert!(inventory.iter().any(|view| view.diagram_id == "diagram-overview" && view.subject_id.as_deref() == Some("type-task")));
    let nested = apply(&after, &Command::CreateVisualDraft {
        diagram_id: "diagram-tasks".into(), notation_kind: "Annotation".into(), label: Some("Note".into()),
        geometry: Geometry::Rect { x: 20.0, y: 30.0, width: 40.0, height: 30.0 },
        properties: Default::default(), parent_view_id: Some(draft_id.clone()), source_view_id: None, target_view_id: None,
    }, 4)?;
    assert!(bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&nested))
        .iter().any(|view| view.label == "Note" && view.parent_view_id.as_deref() == Some(draft_id.as_str())));
    let labeled = apply(
        &after,
        &Command::SetViewLabel {
            view_id: draft_id.clone(),
            label: Some("Revised".into()),
        },
        2,
    )?;
    assert_eq!(labeled.elements, after.elements);
    assert!(bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&labeled))
        .iter().any(|view| view.view_id == draft_id && view.label == "Revised"));
    assert_eq!(
        labeled
            .diagrams
            .iter()
            .flat_map(|diagram| &diagram.views)
            .find(|view| view.id == draft_id)
            .and_then(|view| view.label.as_deref()),
        Some("Revised")
    );
    let canonical_label = apply(
        &labeled,
        &Command::SetViewLabel {
            view_id: "view-task".into(),
            label: Some("Display only".into()),
        },
        3,
    )?;
    assert_eq!(canonical_label.elements, labeled.elements);
    let connected = apply(
        &canonical_label,
        &Command::CreateVisualDraft {
            diagram_id: "diagram-tasks".into(),
            notation_kind: "uml.transition".into(),
            label: None,
            geometry: Geometry::Path {
                start: bellatrix_mde::Point { x: 0.0, y: 0.0 },
                segments: Vec::new(),
            },
            properties: Default::default(),
            parent_view_id: None,
            source_view_id: Some(draft_id.clone()),
            target_view_id: Some("view-task".into()),
        },
        2,
    )?;
    let message = apply(
        &connected,
        &Command::CreateVisualDraft {
            diagram_id: "diagram-tasks".into(),
            notation_kind: "uml.message".into(),
            label: Some("1: retry()".into()),
            geometry: Geometry::Path {
                start: bellatrix_mde::Point { x: 70.0, y: 40.0 },
                segments: vec![
                    bellatrix_mde::PathSegment::Line {
                        to: bellatrix_mde::Point { x: 118.0, y: 40.0 },
                    },
                    bellatrix_mde::PathSegment::Line {
                        to: bellatrix_mde::Point { x: 118.0, y: 80.0 },
                    },
                    bellatrix_mde::PathSegment::Line {
                        to: bellatrix_mde::Point { x: 70.0, y: 80.0 },
                    },
                ],
            },
            properties: serde_json::from_value(
                serde_json::json!({"messageSort":"synchCall", "self_message":true,
            "source_anchor":{"x":0.5,"y":0.25}, "target_anchor":{"x":0.5,"y":0.75}}),
            )?,
            parent_view_id: None,
            source_view_id: Some(draft_id.clone()),
            target_view_id: Some(draft_id.clone()),
        },
        3,
    )?;
    let message_id = message
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "diagram-tasks")
        .and_then(|diagram| diagram.views.last())
        .ok_or("missing visual message")?
        .id
        .clone();
    let moved = apply(
        &message,
        &Command::MoveView {
            view_id: draft_id.clone(),
            dx: 20.0,
            dy: 10.0,
        },
        4,
    )?;
    let connected = apply(
        &moved,
        &Command::MoveView {
            view_id: message_id.clone(),
            dx: 0.0,
            dy: 8.0,
        },
        5,
    )?;
    let message = connected
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.id == message_id)
        .ok_or("missing moved message")?;
    let Geometry::Path { start, segments } = &message.geometry else {
        return Err("missing message path".into());
    };
    assert_eq!(*start, bellatrix_mde::Point { x: 90.0, y: 58.0 });
    assert!(
        matches!(segments.as_slice(), [bellatrix_mde::PathSegment::Line { to: first },
        bellatrix_mde::PathSegment::Line { to: second }, bellatrix_mde::PathSegment::Line { to: end }]
        if first.x == 138.0 && first.y == 58.0 && second.x == 138.0 && second.y == 98.0 && end.x == 90.0 && end.y == 98.0)
    );
    assert_eq!(connected.elements, before.elements);
    let ordering_tool = ModelingRelationKind::GeneralOrdering;
    assert!(ordering_tool.is_visual_only());
    let connected = apply(&connected, &Command::CreateVisualDraft {
        diagram_id: "diagram-tasks".into(), notation_kind: ordering_tool.model_kind().into(),
        label: None, geometry: Geometry::Path {
            start: bellatrix_mde::Point { x: 0.0, y: 0.0 }, segments: Vec::new(),
        },
        properties: serde_json::from_value(serde_json::json!({
            "source_anchor":{"x":0.5,"y":0.25}, "target_anchor":{"x":0.5,"y":0.75}
        }))?,
        parent_view_id: None, source_view_id: Some(draft_id.clone()), target_view_id: Some("view-task".into()),
    }, 6)?;
    let ordering = connected.diagrams.iter().flat_map(|diagram| &diagram.views)
        .find(|view| view.draft.as_ref().is_some_and(|draft| draft.notation_kind == ordering_tool.model_kind()))
        .ok_or("missing visual ordering")?;
    let draft = ordering.draft.as_ref().ok_or("missing ordering presentation")?;
    let notation = bellatrix_mde::resolve_relationship_style(&draft.notation_kind, &draft.properties);
    assert_eq!(notation.line_mode, bellatrix_mde::LineMode::Dotted);
    assert_eq!(notation.target_marker, bellatrix_mde::RelationshipMarker::None);
    assert_eq!(notation.middle_marker, bellatrix_mde::RelationshipMarker::FilledTriangle);
    assert_eq!(connected.elements, before.elements);
    assert_eq!(connected.relations, before.relations);
    let mut connected = connected;
    for tool in bellatrix_mde::commands::ModelingTool::INTERFACE_FORMS {
        let presentation = tool.resolve_interface_presentation().ok_or("missing Interface role")?;
        connected = apply(&connected, &Command::CreateVisualDraft {
            diagram_id: "diagram-tasks".into(), notation_kind: "uml.interface".into(),
            label: Some(tool.label().into()), geometry: Geometry::Rect { x: 420.0, y: 220.0, width: 112.0, height: 68.0 },
            properties: serde_json::from_value(serde_json::json!({"interface_presentation":presentation}))?,
            parent_view_id: None, source_view_id: None, target_view_id: None,
        }, 7)?;
        let placed = connected.diagrams.iter().flat_map(|diagram| &diagram.views)
            .find(|view| view.draft.as_ref().is_some_and(|draft| draft.properties.get("interface_presentation").and_then(serde_json::Value::as_str) == Some(presentation)))
            .ok_or("Interface tool did not materialize a View")?;
        assert!(placed.subject.is_none());
        assert!(bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&connected)).iter()
            .any(|view| view.view_id == placed.id));
    }
    assert_eq!(connected.elements, before.elements);
    assert_eq!(connected.relations, before.relations);
    for tool in [ModelingRelationKind::MessageFound, ModelingRelationKind::MessageLost] {
        let found = tool == ModelingRelationKind::MessageFound;
        let free = bellatrix_mde::Point { x: 700.0, y: 120.0 };
        connected = apply(&connected, &Command::CreateVisualDraft {
            diagram_id: "diagram-tasks".into(), notation_kind: tool.model_kind().into(), label: Some(tool.label().into()),
            geometry: Geometry::Path { start: if found { free } else { bellatrix_mde::Point { x: 200.0, y: 120.0 } },
                segments: vec![bellatrix_mde::PathSegment::Line { to: if found { bellatrix_mde::Point { x: 200.0, y: 120.0 } } else { free } }] },
            properties: serde_json::from_value(serde_json::json!({"messageKind":tool.resolve_message_kind(),"messageSort":"synchCall"}))?,
            parent_view_id: None, source_view_id: (!found).then(|| "view-task".into()), target_view_id: found.then(|| "view-task".into()),
        }, 8)?;
        let message = connected.diagrams.iter().flat_map(|diagram| &diagram.views)
            .find(|view| view.label.as_deref() == Some(tool.label())).ok_or("missing partial message View")?;
        let message_id = message.id.clone();
        let before_geometry = message.geometry.clone();
        let draft = message.draft.as_ref().ok_or("missing message metadata")?;
        let style = bellatrix_mde::resolve_relationship_style(&draft.notation_kind, &draft.properties);
        assert_eq!(if found { style.source_marker } else { style.target_marker }, if found { bellatrix_mde::RelationshipMarker::FilledCircle } else { bellatrix_mde::RelationshipMarker::FilledTriangleAndCircle });
        connected = apply(&connected, &Command::MoveView { view_id: "view-task".into(), dx: 30.0, dy: 15.0 }, 9)?;
        let message = connected.diagrams.iter().flat_map(|diagram| &diagram.views)
            .find(|view| view.id == message_id).ok_or("missing moved partial message")?;
        let Geometry::Path { start, segments } = &message.geometry else { return Err("missing partial message path".into()); };
        let Some(bellatrix_mde::PathSegment::Line { to }) = segments.last() else { return Err("missing free endpoint".into()); };
        assert_eq!(if found { *start } else { *to }, free);
        assert_ne!(message.geometry, before_geometry);
    }
    let removed = apply(
        &connected,
        &Command::RemoveView {
            diagram_id: "diagram-tasks".into(),
            view_id: draft_id.clone(),
        },
        3,
    )?;
    let diagram = removed
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "diagram-tasks")
        .ok_or("missing diagram")?;
    assert!(diagram.views.iter().any(|view| {
        view.endpoints
            .as_ref()
            .is_some_and(|endpoints| endpoints.source_view_id.as_deref() == Some(&draft_id))
    }));
    assert!(
        validate_structural_diagram(&removed, &diagram.id, kind)
            .iter()
            .any(|issue| issue.code == "VISUAL_DRAFT_ORPHANED_LINK")
    );
    assert!(!bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&removed))
        .iter().any(|view| view.view_id == draft_id));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&removed, format)?, format)?;
        assert_eq!(reopened, removed);
        assert_eq!(bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&reopened)),
            bellatrix_mde::visual_components(&bellatrix_mde::ModelingSection::from_model(&removed)));
    }
    Ok(())
}
