use super::ModelingElementKind;
use crate::{
    Element, Geometry, ModelingSection, Relation, Style, UmlQuickCreate, UmlType, View,
    quick_relation_rule, reroute_relationships,
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

pub(super) fn apply(
    section: &mut ModelingSection,
    diagram_id: &str,
    element_id: &str,
    view_id: Option<&str>,
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
    let subject_type = UmlType::from_kind(&kind);
    if action == UmlQuickCreate::TemplateParameter {
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
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let view = if let Some(view_id) = view_id {
        let view = diagram
            .all_views()
            .find(|view| view.id == view_id)
            .ok_or("Quick Edit View not found in the active diagram")?;
        if view.subject.as_deref() != Some(element_id) {
            return Err("Quick Edit View no longer represents the selected subject".into());
        }
        view
    } else {
        let mut occurrences = diagram
            .all_views()
            .filter(|view| view.subject.as_deref() == Some(element_id));
        let view = occurrences
            .next()
            .ok_or("Quick Edit subject has no View in the active diagram")?;
        if occurrences.next().is_some() {
            return Err("Quick Edit requires a View ID when the subject has multiple occurrences in the active diagram".into());
        }
        view
    };
    let (left, top, width, height) = match &view.geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => (*x, *y, *width, *height),
        _ => return Err("Quick Edit requires a node View".into()),
    };
    if matches!(action, UmlQuickCreate::Port | UmlQuickCreate::Part) {
        if diagram
            .all_views()
            .filter(|view| view.subject.as_deref() == Some(element_id))
            .count()
            > 1
        {
            return Err("Quick Edit Port and Part placement is not supported for multiple occurrences of the owner in the active diagram".into());
        }
        return super::features::insert_feature(
            section, diagram_id, element_id, action, revision, project_id,
        );
    }
    let subject_view_id = view.id.clone();
    let (child_kind, prefix, offset, child_size) = match action {
        UmlQuickCreate::Note => (
            "uml.note",
            ModelingElementKind::Comment.default_name(),
            (width + 48.0, -16.0),
            (136.0, 78.0),
        ),
        UmlQuickCreate::Constraint => (
            "uml.constraint",
            "Constraint",
            (-180.0, -32.0),
            (134.0, 76.0),
        ),
        UmlQuickCreate::Subtype | UmlQuickCreate::Supertype => {
            let interface = subject_type == Some(UmlType::Interface);
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
                match subject_type {
                    Some(UmlType::Class) => ModelingElementKind::Class.default_name(),
                    Some(UmlType::Interface) => ModelingElementKind::Interface.default_name(),
                    _ => subject_type
                        .and_then(UmlType::metaclass)
                        .unwrap_or("Element"),
                },
                offset,
                size,
            )
        }
        UmlQuickCreate::RealizingClass => (
            "rust.struct",
            ModelingElementKind::Class.default_name(),
            (width + 65.0, height + 50.0),
            (185.0, 112.0),
        ),
        UmlQuickCreate::ProvidedInterface => (
            "rust.trait",
            ModelingElementKind::Interface.default_name(),
            (width + 55.0, -75.0),
            (120.0, 76.0),
        ),
        UmlQuickCreate::RequiredInterface => (
            "rust.trait",
            ModelingElementKind::Interface.default_name(),
            (width + 55.0, height + 10.0),
            (120.0, 76.0),
        ),
        UmlQuickCreate::AssociatedClass
        | UmlQuickCreate::AggregatedClass
        | UmlQuickCreate::CompositedClass => (
            "rust.struct",
            ModelingElementKind::Class.default_name(),
            (width + 80.0, 0.0),
            (185.0, 112.0),
        ),
        UmlQuickCreate::TemplateParameter | UmlQuickCreate::Port | UmlQuickCreate::Part => {
            return Err("contained features must use Class specialization".into());
        }
        UmlQuickCreate::Reception => return Err("reception must use member command".into()),
    };
    let canonical_kind = match child_kind {
        "rust.struct" | "uml.class" => Some(ModelingElementKind::Class),
        "rust.trait" | "uml.interface" => Some(ModelingElementKind::Interface),
        _ => None,
    };
    let child_size = canonical_kind
        .map(|kind| kind.initial_size_in_diagram(diagram))
        .unwrap_or(child_size);
    let offset = if action == UmlQuickCreate::Supertype {
        (offset.0, -child_size.1 - 90.0)
    } else {
        offset
    };
    let rule = quick_relation_rule(action)
        .ok_or("Quick Edit action does not create a graph relationship")?;
    let (semantic_kind, aggregation) = crate::relation_kind_for_creation(rule.kind);
    let new_id = fresh_id(section, "node", revision, project_id)?;
    let new_view_id = fresh_id(section, "view", revision, project_id)?;
    let annotation_link = match action {
        UmlQuickCreate::Note => Some(
            crate::contract_projection::annotation_link("Comment", &new_id, element_id)
                .ok_or("Comment has no annotation link projection")?,
        ),
        UmlQuickCreate::Constraint => Some(
            crate::contract_projection::annotation_link("Constraint", &new_id, element_id)
                .ok_or("Constraint has no annotation link projection")?,
        ),
        _ => None,
    };
    let relation_id = match &annotation_link {
        Some(link) => link.id.clone(),
        None => fresh_id(section, "relation", revision, project_id)?,
    };
    let relation_view_id = fresh_id(section, "relation-view", revision, project_id)?;
    let owner_id = if matches!(action, UmlQuickCreate::Note | UmlQuickCreate::Constraint) {
        Some(element_id.to_owned())
    } else {
        let model = crate::uml::root::Model::from_section(section);
        let subject = model
            .element(element_id)
            .ok_or("Quick Edit subject not found")?;
        let namespace = model
            .owner(subject)?
            .filter(|owner| owner.conforms_to("Namespace"));
        super::resolve_element_owner(section, child_kind, namespace.map(|owner| owner.id()))?
    };
    let name = super::element_name(section, owner_id.as_deref(), prefix, None)?;
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
    let (new_id, new_view_id) = if let Some(kind) = canonical_kind {
        let new_id = super::create_element(
            section,
            &super::Command::CreateElement {
                diagram_id: diagram_id.into(),
                owner_id: owner_id.clone(),
                kind,
                name: None,
                x: child_x,
                y: child_y,
            },
            revision,
            project_id,
        )?;
        let new_view_id = section
            .diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
            .and_then(|diagram| {
                diagram
                    .all_views()
                    .find(|view| view.subject.as_deref() == Some(&new_id))
            })
            .ok_or("created classifier has no View")?
            .id
            .clone();
        (new_id, new_view_id)
    } else {
        let mut child_properties = std::collections::BTreeMap::new();
        if action == UmlQuickCreate::Note {
            child_properties.insert("body".into(), serde_json::Value::String(name.clone()));
            child_properties.insert(
                "annotatedElement".into(),
                serde_json::json!([{ "$ref": element_id }]),
            );
        } else if action == UmlQuickCreate::Constraint {
            let specification_id = fresh_id(section, "specification", revision, project_id)?;
            let ownership_id = fresh_id(section, "specification-owner", revision, project_id)?;
            child_properties.insert(
                "constrainedElement".into(),
                serde_json::json!([{ "$ref": element_id }]),
            );
            child_properties.insert(
                "specification".into(),
                serde_json::json!({ "$ref": specification_id }),
            );
            // An empty authored expression records no invented predicate or language.
            section.elements.push(Element {
                id: specification_id.clone(),
                kind: "uml.opaque_expression".into(),
                name: String::new(),
                label: Some("Constraint specification".into()),
                description: None,
                properties: std::collections::BTreeMap::from([
                    ("body".into(), serde_json::json!([])),
                    ("language".into(), serde_json::json!([])),
                ]),
                annotations: Vec::new(),
                implementations: Vec::new(),
                content: None,
            });
            section.relations.push(Relation {
                id: ownership_id,
                kind: "owns".into(),
                from: new_id.clone(),
                to: specification_id,
                properties: std::collections::BTreeMap::from([(
                    "role".into(),
                    serde_json::json!("specification"),
                )]),
            });
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
                children: Vec::new(),
                presentation: Default::default(),
                endpoints: None,
                timing: None,
                draft: None,
                id: new_view_id.clone(),
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
        (new_id, new_view_id)
    };
    let (source, target) = if rule.child_is_source {
        (new_id.as_str(), element_id)
    } else {
        (element_id, new_id.as_str())
    };
    let (source_view_id, target_view_id) = if rule.child_is_source {
        (new_view_id, subject_view_id)
    } else {
        (subject_view_id, new_view_id)
    };
    let endpoints = crate::ViewEndpoints {
        source_view_id: Some(source_view_id),
        target_view_id: Some(target_view_id),
    };
    if !rule.kind.is_visual_only() {
        if canonical_kind.is_none()
            && let Some(owner_id) = &owner_id
        {
            crate::uml::root::set_owner(section, project_id, &new_id, owner_id)?;
        }
        let relation_owner = if matches!(semantic_kind, "uml.association" | "uml.usage") {
            owner_id.as_deref()
        } else {
            None
        };
        return super::create_relation(
            section,
            diagram_id,
            rule.kind,
            source,
            target,
            relation_owner,
            Some(&endpoints),
            revision,
            project_id,
        );
    }
    let geometry =
        crate::relationships::route_at_views(section, diagram_id, source, target, &endpoints)?;
    let mut properties = std::collections::BTreeMap::new();
    if semantic_kind == "uml.interface_realization" {
        // This composite role subsets ownedElement, not Namespace::ownedMember.
        properties.insert("visibility".into(), serde_json::Value::Null);
    }
    if let Some(aggregation) = aggregation {
        properties.insert("end1Aggregation".into(), serde_json::json!(aggregation));
    }
    section
        .relations
        .push(annotation_link.unwrap_or_else(|| Relation {
            id: relation_id.clone(),
            kind: semantic_kind.into(),
            from: source.into(),
            to: target.into(),
            properties,
        }));
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .push(View {
            children: Vec::new(),
            presentation: Default::default(),
            endpoints: Some(endpoints),
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
    if let Some(owner_id) = owner_id {
        crate::uml::root::set_owner(section, project_id, &new_id, &owner_id)?;
        if matches!(semantic_kind, "uml.association" | "uml.usage") {
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
            view_id: None,
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
                    original
                        .elements
                        .iter_mut()
                        .find(|element| element.id == "type-task")
                        .unwrap()
                        .properties
                        .insert("isActive".into(), true.into());
                }
                let result = crate::commands::apply(&original, &command(action), 0);
                assert!(result.is_ok(), "{action:?}: {result:?}");
                let project = result.unwrap();
                assert_eq!(
                    project.elements.len(),
                    original.elements.len()
                        + 1
                        + usize::from(
                            project.root_model.is_some() && original.root_model.is_none()
                        )
                );
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
        let result =
            crate::commands::apply(&original, &command(UmlQuickCreate::RequiredInterface), 0);
        assert!(
            matches!(result, Err(ref error) if error.contains("UML_NAMESPACE_MEMBERS_DISTINGUISHABLE")),
            "{result:?}"
        );
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
            original
                .relations
                .iter_mut()
                .find(|relation| relation.id == "ref-task")
                .unwrap()
                .properties
                .insert("name".into(), serde_json::json!("Task reference"));
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
                    }
                    + if relation_kind == Some("uml.association") {
                        2
                    } else {
                        0
                    }
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
                    let section = ModelingSection::from_model(&project);
                    assert_eq!(
                        crate::relationship_notation::association_diamonds(&section, edge)
                            .expect("canonical association diamonds")
                            .first()
                            .copied(),
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
