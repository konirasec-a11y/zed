use crate::{
    Element, Geometry, ModelingSection, Relation, Style, UmlQuickCreate, UmlType, View,
    quick_relation_rule, reroute_relationships, route_relationship,
};

fn fresh_id(
    section: &ModelingSection,
    purpose: &str,
    revision: u64,
    project_id: &str,
) -> Result<String, String> {
    let prefix = format!("mde-quick-{purpose}");
    let [id] = super::fresh_ids(section, [prefix.as_str()], revision, project_id)?;
    Ok(id)
}
fn unique_name(section: &ModelingSection, prefix: &str) -> Result<String, String> {
    let mut sequence = 1_u64;
    loop {
        let name = format!("{prefix}{sequence}");
        if !section.elements.iter().any(|element| element.name == name) {
            return Ok(name);
        }
        sequence = sequence.checked_add(1).ok_or("name sequence overflow")?;
    }
}

pub(super) fn apply(
    section: &mut ModelingSection,
    diagram_id: &str,
    element_id: &str,
    action: UmlQuickCreate,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    let element = section
        .elements
        .iter()
        .find(|element| element.id == element_id)
        .ok_or("Quick Edit subject not found")?;
    let kind = element.kind.clone();
    let subject_type =
        UmlType::from_kind(&kind).ok_or("Quick Edit subject is not a supported UML type")?;
    if !action.accepts(&kind) {
        return Err(format!("{kind} does not support {action:?}"));
    }
    if matches!(
        action,
        UmlQuickCreate::TemplateParameter | UmlQuickCreate::Port | UmlQuickCreate::Part
    ) {
        return super::features::insert_feature(
            section, diagram_id, element_id, action, revision, project_id,
        );
    }
    if action == UmlQuickCreate::Reception {
        super::members::append_member(
            section,
            element_id,
            super::ModelingMemberKind::Reception,
            revision,
            project_id,
        )?;
        return Ok(());
    }
    if !matches!(action, UmlQuickCreate::Note | UmlQuickCreate::Constraint) {
        let model = crate::uml::root::Model::from_section(section);
        let subject = model.element(element_id).ok_or("Quick Edit subject not found")?;
        if model.owner(subject)?.is_none() { crate::uml::root::adopt_root_model(section, project_id, "Model")?; }
    }
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let view = diagram
        .views
        .iter()
        .find(|view| view.subject.as_deref() == Some(element_id))
        .ok_or("Quick Edit subject has no View in the active diagram")?;
    let (left, top, width, height) = match &view.geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => (*x, *y, *width, *height),
        _ => return Err("Quick Edit requires a node View".into()),
    };
    let (child_kind, prefix, offset, child_size) = match action {
        UmlQuickCreate::Note => ("uml.note", "Note", (width + 48.0, -16.0), (136.0, 78.0)),
        UmlQuickCreate::Constraint => (
            "uml.constraint",
            "Constraint",
            (-180.0, -32.0),
            (134.0, 76.0),
        ),
        UmlQuickCreate::Subtype | UmlQuickCreate::Supertype => {
            let interface = subject_type == UmlType::Interface;
            let size = if interface {
                (120.0, 76.0)
            } else {
                (185.0, 112.0)
            };
            let offset = if action == UmlQuickCreate::Subtype {
                (0.0, height + 90.0)
            } else {
                (0.0, -size.1 - 90.0)
            };
            (
                kind.as_str(),
                subject_type
                    .metaclass()
                    .ok_or("Quick Edit subject has no UML metaclass")?,
                offset,
                size,
            )
        }
        UmlQuickCreate::RealizingClass => (
            "rust.struct",
            "Class",
            (width + 65.0, height + 50.0),
            (185.0, 112.0),
        ),
        UmlQuickCreate::ProvidedInterface => (
            "rust.trait",
            "Interface",
            (width + 55.0, -75.0),
            (120.0, 76.0),
        ),
        UmlQuickCreate::RequiredInterface => (
            "rust.trait",
            "Interface",
            (width + 55.0, height + 10.0),
            (120.0, 76.0),
        ),
        UmlQuickCreate::AssociatedClass
        | UmlQuickCreate::AggregatedClass
        | UmlQuickCreate::CompositedClass => {
            ("rust.struct", "Class", (width + 80.0, 0.0), (185.0, 112.0))
        }
        UmlQuickCreate::TemplateParameter | UmlQuickCreate::Port | UmlQuickCreate::Part => {
            return Err("contained features must use Class specialization".into());
        }
        UmlQuickCreate::Reception => return Err("reception must use member command".into()),
    };
    let rule = quick_relation_rule(action)
        .ok_or("Quick Edit action does not create a graph relationship")?;
    let new_id = fresh_id(section, "node", revision, project_id)?;
    let new_view_id = fresh_id(section, "view", revision, project_id)?;
    let relation_id = fresh_id(section, "relation", revision, project_id)?;
    let relation_view_id = fresh_id(section, "relation-view", revision, project_id)?;
    let name = unique_name(section, prefix)?;
    let z_index = diagram
        .views
        .iter()
        .map(|view| view.z_index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("z-order overflow")?;
    let edge_z_index = z_index.checked_add(1).ok_or("z-order overflow")?;
    let child_x = left + offset.0;
    let mut child_y = top + offset.1;
    for _ in 0..=diagram.views.len() {
        let overlaps = diagram.views.iter().any(|view| match &view.geometry {
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => {
                child_x < x + width + 8.0
                    && child_x + child_size.0 + 8.0 > *x
                    && child_y < y + height + 8.0
                    && child_y + child_size.1 + 8.0 > *y
            }
            _ => false,
        });
        if !overlaps {
            break;
        }
        child_y += child_size.1 + 24.0;
    }
    let mut child_properties = std::collections::BTreeMap::new();
    if action == UmlQuickCreate::Note {
        child_properties.insert("body".into(), serde_json::Value::String(name.clone()));
    }
    section.elements.push(Element {
        id: new_id.clone(),
        kind: child_kind.into(),
        name,
        label: None,
        description: None,
        properties: child_properties,
        annotations: Vec::new(),
        implementations: Vec::new(),
        content: None,
    });
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .push(View {
            endpoints: None,
            timing: None,
            draft: None,
            id: new_view_id,
            subject: Some(new_id.clone()),
            label: None,
            geometry: Geometry::Rect {
                x: child_x,
                y: child_y,
                width: child_size.0,
                height: child_size.1,
            },
            style: Style::default(),
            z_index,
        });
    let (source, target) = if rule.child_is_source {
        (new_id.as_str(), element_id)
    } else {
        (element_id, new_id.as_str())
    };
    let geometry = route_relationship(section, diagram_id, source, target)?;
    let mut properties = std::collections::BTreeMap::new();
    if rule.kind == "uml.interface_realization" {
        // This composite role subsets ownedElement, not Namespace::ownedMember.
        properties.insert("visibility".into(), serde_json::Value::Null);
    }
    if let Some(aggregation) = rule.aggregation {
        properties.insert("end1Aggregation".into(), serde_json::json!(aggregation));
    }
    section.relations.push(Relation {
        id: relation_id.clone(),
        kind: rule.kind.into(),
        from: source.into(),
        to: target.into(),
        properties,
    });
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .push(View {
            endpoints: None,
            timing: None,
            draft: None,
            id: relation_view_id,
            subject: Some(relation_id.clone()),
            label: None,
            geometry,
            style: Style {
                routing: Some(crate::RelationshipRouting::Automatic),
                ..Style::default()
            },
            z_index: edge_z_index,
        });
    if matches!(action, UmlQuickCreate::Note | UmlQuickCreate::Constraint) {
        crate::uml::root::set_owner(section, project_id, &new_id, element_id)?;
    } else {
        let model = crate::uml::root::Model::from_section(section);
        let subject = model.element(element_id).ok_or("Quick Edit subject not found")?;
        let namespace = model.owner(subject)?.filter(|owner| owner.conforms_to("Namespace")).map(|owner| owner.id().to_owned());
        let owner_id = match namespace { Some(id) => id, None => crate::uml::root::adopt_root_model(section, project_id, "Model")? };
        crate::uml::root::set_owner(section, project_id, &new_id, &owner_id)?;
        if matches!(rule.kind, "uml.association" | "uml.usage") {
            crate::uml::root::set_owner(section, project_id, &relation_id, &owner_id)?;
        }
    }
    reroute_relationships(section, diagram_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Graph, Project, commands::Command};
    fn fixture() -> Project {
        serde_json::from_str(include_str!("../../../fixtures/todo.json")).unwrap()
    }
    fn command(action: UmlQuickCreate) -> Command {
        Command::QuickEdit {
            diagram_id: "diagram-tasks".into(),
            element_id: "type-task".into(),
            action,
        }
    }
    #[test]
    fn shared_commands_work_for_class_and_interface() {
        for subject_kind in ["rust.struct", "rust.trait", "uml.class", "uml.interface"] {
            for action in [
                UmlQuickCreate::Note,
                UmlQuickCreate::Constraint,
                UmlQuickCreate::Reception,
                UmlQuickCreate::Subtype,
                UmlQuickCreate::Supertype,
            ] {
                let mut original = fixture();
                original
                    .elements
                    .iter_mut()
                    .find(|element| element.id == "type-task")
                    .unwrap()
                    .kind = subject_kind.into();
                if matches!(subject_kind, "rust.struct" | "uml.class") {
                    original.elements.iter_mut().find(|element| element.id == "type-task")
                        .unwrap().properties.insert("isActive".into(), true.into());
                }
                let result = crate::commands::apply(&original, &command(action), 0);
                assert!(result.is_ok(), "{action:?}: {result:?}");
                let project = result.unwrap();
                assert_eq!(project.elements.len(), original.elements.len() + 1 + usize::from(project.root_model.is_some() && original.root_model.is_none()));
                Graph::new(&project).validate().unwrap();
                if matches!(action, UmlQuickCreate::Subtype | UmlQuickCreate::Supertype) {
                    let child = project.elements.last().unwrap();
                    assert_eq!(child.kind, subject_kind);
                    let edge = project
                        .relations
                        .iter()
                        .find(|edge| edge.kind == "uml.generalization")
                        .unwrap();
                    if action == UmlQuickCreate::Subtype {
                        assert_eq!(edge.from, child.id);
                        assert_eq!(edge.to, "type-task");
                    } else {
                        assert_eq!(edge.from, "type-task");
                        assert_eq!(edge.to, child.id);
                    }
                }
            }
        }
    }
    #[test]
    fn required_interface_rejects_anonymous_dependency_name_collision_atomically() {
        let original = fixture();
        let before = original.clone();
        let result = crate::commands::apply(&original, &command(UmlQuickCreate::RequiredInterface), 0);
        assert!(matches!(result, Err(ref error) if error.contains("UML_NAMESPACE_MEMBERS_DISTINGUISHABLE")), "{result:?}");
        assert_eq!(original, before);
    }

    #[test]
    fn every_class_specialization_creates_a_semantic_result() {
        for (action, expected_kind, relation_kind) in [
            (
                UmlQuickCreate::TemplateParameter,
                "uml.classifier_template_parameter",
                None,
            ),
            (
                UmlQuickCreate::ProvidedInterface,
                "rust.trait",
                Some("uml.interface_realization"),
            ),
            (
                UmlQuickCreate::RequiredInterface,
                "rust.trait",
                Some("uml.usage"),
            ),
            (
                UmlQuickCreate::AssociatedClass,
                "rust.struct",
                Some("uml.association"),
            ),
            (
                UmlQuickCreate::AggregatedClass,
                "rust.struct",
                Some("uml.association"),
            ),
            (
                UmlQuickCreate::CompositedClass,
                "rust.struct",
                Some("uml.association"),
            ),
            (UmlQuickCreate::Port, "uml.port", None),
            (UmlQuickCreate::Part, "uml.part", None),
        ] {
            let mut original = fixture();
            // The positive fixture must not create an anonymous Usage/Dependency
            // name collision in the Model adopted by this command.
            original.relations.iter_mut().find(|relation| relation.id == "ref-task")
                .unwrap().properties.insert("name".into(), serde_json::json!("Task reference"));
            let result = crate::commands::apply(&original, &command(action), 0);
            assert!(result.is_ok(), "{action:?}: {result:?}");
            let project = result.unwrap();
            assert_eq!(
                project.elements.len(),
                original.elements.len()
                    + if action == UmlQuickCreate::TemplateParameter {
                        3
                    } else {
                        1
                    } + if relation_kind == Some("uml.association") { 2 } else { 0 }
                    + usize::from(project.root_model.is_some() && original.root_model.is_none()),
                "{action:?}"
            );
            let child = project
                .elements
                .iter()
                .find(|element| element.kind == expected_kind && element.id.starts_with("mde-"))
                .unwrap();
            if let Some(kind) = relation_kind {
                let edge = project
                    .relations
                    .iter()
                    .find(|relation| relation.kind == kind)
                    .unwrap();
                assert!(edge.from == child.id || edge.to == child.id);
                let aggregation = match action {
                    UmlQuickCreate::AggregatedClass => Some("shared"),
                    UmlQuickCreate::CompositedClass => Some("composite"),
                    _ => None,
                };
                if let Some(value) = aggregation {
                    assert_eq!(
                        edge.properties
                            .get("end1Aggregation")
                            .and_then(|value| value.as_str()),
                        Some(value)
                    );
                }
                assert!(
                    project
                        .diagrams
                        .iter()
                        .flat_map(|diagram| &diagram.views)
                        .any(|view| view.subject.as_deref() == Some(edge.id.as_str()))
                );
            } else {
                assert!(project.relations.iter().any(|relation| {
                    relation.kind == "owns"
                        && (relation.from == "type-task"
                            || (action == UmlQuickCreate::TemplateParameter
                                && project.elements.iter().any(|owner| {
                                    owner.id == relation.from
                                        && owner.kind == "uml.redefinable_template_signature"
                                })))
                        && relation.to == child.id
                }));
                assert_eq!(
                    project
                        .diagrams
                        .iter()
                        .flat_map(|diagram| &diagram.views)
                        .any(|view| view.subject.as_deref() == Some(child.id.as_str())),
                    action != UmlQuickCreate::TemplateParameter
                );
            }
        }
    }
    #[test]
    fn class_capability_is_not_accidentally_inherited_by_interface() {
        let mut original = fixture();
        original
            .elements
            .iter_mut()
            .find(|element| element.id == "type-task")
            .unwrap()
            .kind = "rust.trait".into();
        let before = original.clone();
        for action in [
            UmlQuickCreate::ProvidedInterface,
            UmlQuickCreate::RequiredInterface,
            UmlQuickCreate::Port,
            UmlQuickCreate::Part,
        ] {
            assert!(crate::commands::apply(&original, &command(action), 0).is_err());
            assert_eq!(original, before, "{action:?}");
        }
    }
    #[test]
    fn a_package_can_be_annotated_but_cannot_receive_a_reception() {
        let mut original = fixture();
        original
            .elements
            .iter_mut()
            .find(|element| element.id == "type-task")
            .unwrap()
            .kind = "uml.package".into();
        assert!(crate::commands::apply(&original, &command(UmlQuickCreate::Reception), 0).is_err());
        let project = crate::commands::apply(&original, &command(UmlQuickCreate::Note), 0).unwrap();
        assert!(
            project
                .elements
                .iter()
                .any(|element| element.kind == "uml.note")
        );
    }
}
