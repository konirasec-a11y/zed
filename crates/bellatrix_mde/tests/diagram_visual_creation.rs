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
    Ok(())
}
