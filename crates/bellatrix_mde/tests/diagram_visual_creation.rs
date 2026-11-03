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
    )
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
    )
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
            assert!(
                apply(
                    &source,
                    &Command::CreateDiagram {
                        kind,
                        name: "Invalid".into(),
                        model_elements: vec!["type-list".into()],
                    },
                    0
                )
                .is_err()
            );
        }
    }
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
            assert!(linked.relations.iter().any(|owner| owner.kind == "owns"
                && &owner.from == owner_id
                && owner.to == edge.id));
            let presentation = bellatrix_mde::relationship_presentation(edge);
            assert!(presentation.style.dashed);
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
    for pin in [ModelingElementKind::InputPin, ModelingElementKind::OutputPin, ModelingElementKind::ValuePin] {
        let pinned = create(&graph, pin, Some(&source_node))?;
        let pin_id = pinned.elements.last().ok_or("missing pin")?.id.clone();
        let pin_view = pinned.diagrams.iter().flat_map(|diagram| &diagram.views)
            .find(|view| view.subject.as_deref() == Some(&pin_id)).ok_or("missing pin view")?;
        let before = pin_view.geometry.clone();
        let action_view = pinned.diagrams.iter().flat_map(|diagram| &diagram.views)
            .find(|view| view.subject.as_deref() == Some(&source_node)).ok_or("missing action view")?.id.clone();
        let moved = apply(&pinned, &Command::MoveView { view_id: action_view, dx: 50.0, dy: 30.0 }, 0)?;
        let after = &moved.diagrams.iter().flat_map(|diagram| &diagram.views).find(|view| view.id == pin_view.id).ok_or("missing moved pin")?.geometry;
        let mut expected = before;
        expected.translate(50.0, 30.0);
        assert_eq!(after, &expected);
        assert!(create(&graph, pin, Some(&activity_id)).is_err());
    }
    let interaction = create(&source, ModelingElementKind::Interaction, None)?;
    let interaction_id = interaction.elements.last().ok_or("missing interaction")?.id.clone();
    let fragment = create(&interaction, ModelingElementKind::CombinedFragment, Some(&interaction_id))?;
    let fragment_id = fragment.elements.last().ok_or("missing fragment")?.id.clone();
    let one = create(&fragment, ModelingElementKind::InteractionOperand, Some(&fragment_id))?;
    let one_id = one.elements.last().ok_or("missing operand")?.id.clone();
    let two = create(&one, ModelingElementKind::InteractionOperand, Some(&fragment_id))?;
    let two_id = two.elements.last().ok_or("missing operand")?.id.clone();
    let views = two.diagrams.iter().flat_map(|diagram| &diagram.views).filter(|view| view.subject.as_deref().is_some_and(|id| id == one_id || id == two_id)).collect::<Vec<_>>();
    assert_eq!(views.len(), 2);
    assert_ne!(views[0].geometry, views[1].geometry);
    assert!(create(&interaction, ModelingElementKind::InteractionOperand, Some(&interaction_id)).is_err());
    assert_eq!(decode(&encode(&two, Format::Protobuf)?, Format::Protobuf)?, two);
    Ok(())
}
