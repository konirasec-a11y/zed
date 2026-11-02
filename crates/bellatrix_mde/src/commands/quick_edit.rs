use crate::{Element, Geometry, ModelingSection, Relation, Style, UmlQuickCommand, UmlType, View, quick_relation_rule, reroute_relationships, route_relationship};

fn fresh_id(section: &ModelingSection, purpose: &str, revision: u64) -> Result<String, String> {
    let prefix = format!("mde-quick-{purpose}");
    let [id] = super::fresh_ids(section, [prefix.as_str()], revision)?;
    Ok(id)
}
fn unique_name(section: &ModelingSection, prefix: &str) -> Result<String, String> {
    let mut sequence = 1_u64;
    loop {
        let name = format!("{prefix}{sequence}");
        if !section.elements.iter().any(|element| element.name == name) { return Ok(name); }
        sequence = sequence.checked_add(1).ok_or("name sequence overflow")?;
    }
}

pub(super) fn apply(section: &mut ModelingSection, diagram_id: &str, element_id: &str,
    action: UmlQuickCommand, revision: u64) -> Result<(), String> {
    let element = section.elements.iter().find(|element| element.id == element_id).ok_or("Quick Edit subject not found")?;
    let kind = element.kind.clone();
    let subject_type = UmlType::from_kind(&kind).ok_or("Quick Edit subject is not a supported UML type")?;
    if !action.accepts(&kind) { return Err(format!("{kind} does not support {action:?}")); }
    if matches!(action, UmlQuickCommand::AddTemplateParameter | UmlQuickCommand::AddPort | UmlQuickCommand::AddPart) {
        return super::features::insert_feature(section, diagram_id, element_id, action, revision);
    }
    if action == UmlQuickCommand::AddReception {
        super::members::append_member(section, element_id, super::ModelingMemberKind::Reception, revision)?;
        return Ok(());
    }
    let diagram = section.diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let view = diagram.views.iter().find(|view| view.subject.as_deref() == Some(element_id)).ok_or("Quick Edit subject has no View in the active diagram")?;
    let (left, top, width, height) = match &view.geometry {
        Geometry::Rect { x, y, width, height } => (*x, *y, *width, *height),
        _ => return Err("Quick Edit requires a node View".into()),
    };
    let (child_kind, prefix, offset, child_size) = match action {
        UmlQuickCommand::AddNote => ("uml.note", "Note", (width + 48.0, -16.0), (136.0, 78.0)),
        UmlQuickCommand::AddConstraint => ("uml.constraint", "Constraint", (-180.0, -32.0), (134.0, 76.0)),
        UmlQuickCommand::AddSubtype | UmlQuickCommand::AddSupertype => {
            let interface = subject_type == UmlType::Interface;
            let size = if interface { (120.0, 76.0) } else { (185.0, 112.0) };
            let offset = if action == UmlQuickCommand::AddSubtype { (0.0, height + 90.0) } else { (0.0, -size.1 - 90.0) };
            (kind.as_str(), if interface { "Interface" } else { "Class" }, offset, size)
        }
        UmlQuickCommand::AddRealizingClass => ("rust.struct", "Class", (width + 65.0, height + 50.0), (185.0, 112.0)),
        UmlQuickCommand::AddProvidedInterface => ("rust.trait", "Interface", (width + 55.0, -75.0), (120.0, 76.0)),
        UmlQuickCommand::AddRequiredInterface => ("rust.trait", "Interface", (width + 55.0, height + 10.0), (120.0, 76.0)),
        UmlQuickCommand::AddAssociatedClass | UmlQuickCommand::AddAggregatedClass | UmlQuickCommand::AddCompositedClass =>
            ("rust.struct", "Class", (width + 80.0, 0.0), (185.0, 112.0)),
        UmlQuickCommand::AddTemplateParameter | UmlQuickCommand::AddPort | UmlQuickCommand::AddPart =>
            return Err("contained features must use Class specialization".into()),
        UmlQuickCommand::AddReception => return Err("reception must use member command".into()),
    };
    let rule = quick_relation_rule(action).ok_or("Quick Edit action does not create a graph relationship")?;
    let new_id = fresh_id(section, "node", revision)?;
    let new_view_id = fresh_id(section, "view", revision)?;
    let relation_id = fresh_id(section, "relation", revision)?;
    let relation_view_id = fresh_id(section, "relation-view", revision)?;
    let owner_id = fresh_id(section, "owner", revision)?;
    let name = unique_name(section, prefix)?;
    let z_index = diagram.views.iter().map(|view| view.z_index).max().unwrap_or(0).checked_add(1).ok_or("z-order overflow")?;
    let edge_z_index = z_index.checked_add(1).ok_or("z-order overflow")?;
    let child_x = left + offset.0;
    let mut child_y = top + offset.1;
    for _ in 0..=diagram.views.len() {
        let overlaps = diagram.views.iter().any(|view| match &view.geometry {
            Geometry::Rect { x, y, width, height } => child_x < x + width + 8.0 && child_x + child_size.0 + 8.0 > *x
                && child_y < y + height + 8.0 && child_y + child_size.1 + 8.0 > *y,
            _ => false,
        });
        if !overlaps { break; }
        child_y += child_size.1 + 24.0;
    }
    section.elements.push(Element { id: new_id.clone(), kind: child_kind.into(), name,
        label: None, description: None, properties: Default::default(), annotations: Vec::new(), implementations: Vec::new(), content: None });
    section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?.views.push(View {
        id: new_view_id, subject: Some(new_id.clone()), label: None,
        geometry: Geometry::Rect { x: child_x, y: child_y, width: child_size.0, height: child_size.1 }, style: Style::default(), z_index,
    });
    let (source, target) = if rule.child_is_source { (new_id.as_str(), element_id) } else { (element_id, new_id.as_str()) };
    let geometry = route_relationship(section, diagram_id, source, target)?;
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("routing".into(), serde_json::json!("automatic"));
    if let Some(aggregation) = rule.aggregation { properties.insert("end1Aggregation".into(), serde_json::json!(aggregation)); }
    section.relations.push(Relation { id: relation_id.clone(), kind: rule.kind.into(), from: source.into(), to: target.into(), properties });
    section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?.views.push(View {
        id: relation_view_id, subject: Some(relation_id), label: None, geometry, style: Style::default(), z_index: edge_z_index,
    });
    if action == UmlQuickCommand::AddConstraint {
        section.relations.push(Relation { id: owner_id, kind: "owns".into(), from: element_id.into(), to: new_id, properties: Default::default() });
    }
    reroute_relationships(section, diagram_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Graph, Project, commands::Command};
    fn fixture() -> Project { serde_json::from_str(include_str!("../../fixtures/todo.json")).unwrap() }
    fn command(action: UmlQuickCommand) -> Command {
        Command::QuickEdit { diagram_id: "diagram-tasks".into(), element_id: "type-task".into(), action }
    }
    #[test]
    fn shared_commands_work_for_class_and_interface() {
        for subject_kind in ["rust.struct", "rust.trait", "uml.class", "uml.interface"] {
            for action in [UmlQuickCommand::AddNote, UmlQuickCommand::AddConstraint, UmlQuickCommand::AddReception,
                UmlQuickCommand::AddSubtype, UmlQuickCommand::AddSupertype] {
                let mut original = fixture();
                original.elements.iter_mut().find(|element| element.id == "type-task").unwrap().kind = subject_kind.into();
                let project = crate::commands::apply(&original, &command(action), 0).unwrap();
                assert_eq!(project.elements.len(), original.elements.len() + 1);
                Graph::new(&project).validate().unwrap();
                if matches!(action, UmlQuickCommand::AddSubtype | UmlQuickCommand::AddSupertype) {
                    let child = project.elements.last().unwrap();
                    assert_eq!(child.kind, subject_kind);
                    let edge = project.relations.iter().find(|edge| edge.kind == "uml.generalization").unwrap();
                    if action == UmlQuickCommand::AddSubtype { assert_eq!(edge.from, child.id); assert_eq!(edge.to, "type-task"); }
                    else { assert_eq!(edge.from, "type-task"); assert_eq!(edge.to, child.id); }
                }
            }
        }
    }
    #[test]
    fn every_class_specialization_creates_a_semantic_result() {
        for (action, expected_kind, relation_kind) in [
            (UmlQuickCommand::AddTemplateParameter, "uml.template_parameter", None),
            (UmlQuickCommand::AddProvidedInterface, "rust.trait", Some("uml.interface_realization")),
            (UmlQuickCommand::AddRequiredInterface, "rust.trait", Some("uml.dependency")),
            (UmlQuickCommand::AddAssociatedClass, "rust.struct", Some("uml.association")),
            (UmlQuickCommand::AddAggregatedClass, "rust.struct", Some("uml.association")),
            (UmlQuickCommand::AddCompositedClass, "rust.struct", Some("uml.association")),
            (UmlQuickCommand::AddPort, "uml.port", None), (UmlQuickCommand::AddPart, "uml.part", None),
        ] {
            let original = fixture();
            let project = crate::commands::apply(&original, &command(action), 0).unwrap();
            assert_eq!(project.elements.len(), original.elements.len() + 1, "{action:?}");
            let child = project.elements.iter().find(|element| element.kind == expected_kind && element.id.starts_with("mde-")).unwrap();
            if let Some(kind) = relation_kind {
                let edge = project.relations.iter().find(|relation| relation.kind == kind).unwrap();
                assert!(edge.from == child.id || edge.to == child.id);
                let aggregation = match action { UmlQuickCommand::AddAggregatedClass => Some("shared"), UmlQuickCommand::AddCompositedClass => Some("composite"), _ => None };
                if let Some(value) = aggregation { assert_eq!(edge.properties.get("end1Aggregation").and_then(|value| value.as_str()), Some(value)); }
                assert!(project.diagrams.iter().flat_map(|diagram| &diagram.views).any(|view| view.subject.as_deref() == Some(edge.id.as_str())));
            } else {
                assert!(project.relations.iter().any(|relation| relation.kind == "owns" && relation.from == "type-task" && relation.to == child.id));
                assert_eq!(project.diagrams.iter().flat_map(|diagram| &diagram.views).any(|view| view.subject.as_deref() == Some(child.id.as_str())), action != UmlQuickCommand::AddTemplateParameter);
            }
        }
    }
    #[test]
    fn class_capability_is_not_accidentally_inherited_by_interface() {
        let mut original = fixture();
        original.elements.iter_mut().find(|element| element.id == "type-task").unwrap().kind = "rust.trait".into();
        let before = original.clone();
        for action in [UmlQuickCommand::AddTemplateParameter, UmlQuickCommand::AddProvidedInterface, UmlQuickCommand::AddRequiredInterface,
            UmlQuickCommand::AddAssociatedClass, UmlQuickCommand::AddAggregatedClass, UmlQuickCommand::AddCompositedClass,
            UmlQuickCommand::AddPort, UmlQuickCommand::AddPart] {
            assert!(crate::commands::apply(&original, &command(action), 0).is_err());
            assert_eq!(original, before, "{action:?}");
        }
    }
    #[test]
    fn a_package_can_be_annotated_but_cannot_receive_a_reception() {
        let mut original = fixture();
        original.elements.iter_mut().find(|element| element.id == "type-task").unwrap().kind = "uml.package".into();
        assert!(crate::commands::apply(&original, &command(UmlQuickCommand::AddReception), 0).is_err());
        let project = crate::commands::apply(&original, &command(UmlQuickCommand::AddNote), 0).unwrap();
        assert!(project.elements.iter().any(|element| element.kind == "uml.note"));
    }
}
