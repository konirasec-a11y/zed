use bellatrix_mde::commands::{self, Command, ModelingMemberKind};
use bellatrix_mde::{
    ModelingSection, Project, UmlCapability, UmlQuickCreate, UmlType, inspect_element,
    quick_edit_for,
};

fn project(kind: &str) -> Project {
    let mut project: Project =
        serde_json::from_str(include_str!("../fixtures/todo.json")).expect("fixture");
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "type-task")
        .expect("subject")
        .kind = kind.into();
    project
}

#[test]
fn inherited_members_use_the_same_contract_in_commands_and_quick_edit() {
    for (kind, attributes, operations, receptions, ports, parts) in [
        ("uml.component", true, true, true, true, true),
        ("uml.artifact", true, true, false, false, false),
        ("uml.signal", true, false, false, false, false),
        ("uml.collaboration", true, false, false, false, true),
        ("uml.node", true, true, true, true, true),
        ("uml.device", true, true, true, true, true),
        ("uml.execution_environment", true, true, true, true, true),
        ("uml.stereotype", true, true, true, true, true),
        ("uml.nary_association", false, false, false, false, false),
        ("uml.package", false, false, false, false, false),
        ("uml.model", false, false, false, false, false),
    ] {
        let original = project(kind);
        let before = original.clone();
        let semantic_type = UmlType::from_kind(kind).expect("type");
        for (member, capability, action, supported) in [
            (
                ModelingMemberKind::Attribute,
                UmlCapability::Attribute,
                "uml.add-attribute",
                attributes,
            ),
            (
                ModelingMemberKind::Operation,
                UmlCapability::Operation,
                "uml.add-operation",
                operations,
            ),
            (
                ModelingMemberKind::Reception,
                UmlCapability::Reception,
                "uml.add-reception",
                receptions,
            ),
        ] {
            assert_eq!(
                semantic_type.supports(capability),
                supported,
                "{kind} {capability:?}"
            );
            assert_eq!(
                quick_edit_for(kind).iter().any(|entry| entry.id == action),
                supported,
                "{kind} {action}"
            );
            let result = commands::apply(
                &original,
                &Command::AddMember {
                    owner_id: "type-task".into(),
                    kind: member,
                },
                0,
            );
            assert_eq!(result.is_ok(), supported, "{kind} {member:?}: {result:?}");
            if let Ok(updated) = result {
                let member = updated.elements.last().expect("new member");
                assert!(
                    updated
                        .relations
                        .iter()
                        .any(|relation| relation.kind == "owns"
                            && relation.from == "type-task"
                            && relation.to == member.id)
                );
            }
        }
        for (action, supported) in [(UmlQuickCreate::Port, ports), (UmlQuickCreate::Part, parts)] {
            let action_id = match action {
                UmlQuickCreate::Port => "uml.add-port",
                UmlQuickCreate::Part => "uml.add-part",
                _ => unreachable!(),
            };
            assert_eq!(
                quick_edit_for(kind)
                    .iter()
                    .any(|entry| entry.id == action_id),
                supported
            );
            let result = commands::apply(
                &original,
                &Command::QuickEdit {
                    diagram_id: "diagram-tasks".into(),
                    element_id: "type-task".into(),
                    action,
                },
                0,
            );
            assert_eq!(result.is_ok(), supported, "{kind} {action:?}: {result:?}");
        }
        assert_eq!(original, before);
    }
}

#[test]
fn inspector_respects_inherited_metaclass_properties() {
    for (kind, classifier, active) in [
        ("uml.component", true, true),
        ("uml.artifact", true, false),
        ("uml.interface", true, false),
        ("uml.collaboration", true, false),
        ("uml.package", false, false),
        ("uml.model", false, false),
    ] {
        let section = ModelingSection::from_model(&project(kind));
        let inspector = inspect_element(&section, "type-task").expect("inspector");
        for (field, expected) in [
            ("isAbstract", classifier),
            ("isFinalSpecialization", classifier),
            ("isActive", active),
        ] {
            assert_eq!(
                inspector
                    .properties
                    .iter()
                    .any(|property| property.path == format!("properties.{field}")),
                expected,
                "{kind}: {field}"
            );
        }
    }
}

#[test]
fn legacy_template_schema_does_not_claim_a_normative_metaclass() {
    assert_eq!(UmlType::Template.metaclass(), None);
    let section = ModelingSection::from_model(&project("uml.template"));
    let inspector = inspect_element(&section, "type-task").expect("legacy inspector");
    for field in ["stereotype", "visibility", "isAbstract", "isLeaf"] {
        assert!(
            inspector
                .properties
                .iter()
                .any(|property| property.path == format!("properties.{field}"))
        );
    }
}
