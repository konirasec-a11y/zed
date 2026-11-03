use crate::{Element, Geometry, ModelingSection, Relation, Style, UmlQuickCreate, View};
use std::collections::HashSet;

fn fresh(
    section: &ModelingSection,
    prefix: &str,
    revision: u64,
    project_id: &str,
) -> Result<String, String> {
    let prefix = format!("mde-class-{prefix}");
    let [id] = super::fresh_ids(section, [prefix.as_str()], revision, project_id)?;
    Ok(id)
}

fn name(section: &ModelingSection, prefix: &str) -> Result<String, String> {
    for sequence in 1_u64.. {
        let candidate = format!("{prefix}{sequence}");
        if !section
            .elements
            .iter()
            .any(|element| element.name == candidate)
        {
            return Ok(candidate);
        }
    }
    Err("member name overflow".into())
}

fn contained_ids(section: &ModelingSection, owner_id: &str) -> HashSet<String> {
    section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter_map(|relation| {
            section
                .elements
                .iter()
                .find(|element| element.id == relation.to)
        })
        .filter(|element| matches!(element.kind.as_str(), "uml.port" | "uml.part"))
        .map(|element| element.id.clone())
        .collect()
}

pub(super) fn translate_contained_views(
    section: &mut ModelingSection,
    diagram_id: &str,
    owner_id: &str,
    dx: f64,
    dy: f64,
) {
    let children = contained_ids(section, owner_id);
    if let Some(diagram) = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
    {
        for view in &mut diagram.views {
            if view
                .subject
                .as_ref()
                .is_some_and(|id| children.contains(id))
            {
                view.geometry.translate(dx, dy);
            }
        }
    }
}

pub(super) fn reposition_ports(section: &mut ModelingSection, diagram_id: &str, owner_id: &str) {
    let port_ids: HashSet<_> = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter_map(|relation| {
            section
                .elements
                .iter()
                .find(|element| element.id == relation.to)
        })
        .filter(|element| element.kind == "uml.port")
        .map(|element| element.id.clone())
        .collect();
    if let Some(diagram) = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
    {
        let bounds = diagram
            .views
            .iter()
            .find(|view| view.subject.as_deref() == Some(owner_id))
            .and_then(|view| match &view.geometry {
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => Some((*x, *y, *width, *height)),
                _ => None,
            });
        if let Some((x, y, width, height)) = bounds {
            for view in &mut diagram.views {
                if !view
                    .subject
                    .as_ref()
                    .is_some_and(|id| port_ids.contains(id))
                {
                    continue;
                }
                if let Geometry::Rect {
                    x: child_x,
                    y: child_y,
                    width: child_width,
                    height: child_height,
                } = &mut view.geometry
                {
                    *child_x = x + width - *child_width / 2.0;
                    *child_y =
                        (*child_y).clamp(y - *child_height / 2.0, y + height - *child_height / 2.0);
                }
            }
        }
    }
}

pub(super) fn class_minimum_bounds(
    section: &ModelingSection,
    diagram_id: &str,
    owner_id: &str,
) -> Option<(f64, f64)> {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)?;
    let (x, y) = diagram
        .views
        .iter()
        .find(|view| view.subject.as_deref() == Some(owner_id))
        .and_then(|view| match &view.geometry {
            Geometry::Rect { x, y, .. } => Some((*x, *y)),
            _ => None,
        })?;
    let part_ids: HashSet<_> = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter_map(|relation| {
            section
                .elements
                .iter()
                .find(|element| element.id == relation.to)
        })
        .filter(|element| element.kind == "uml.part")
        .map(|element| element.id.clone())
        .collect();
    let mut minimum_width: f64 = 80.0;
    let mut minimum_height: f64 = 48.0;
    for view in &diagram.views {
        if !view
            .subject
            .as_ref()
            .is_some_and(|id| part_ids.contains(id))
        {
            continue;
        }
        if let Geometry::Rect {
            x: child_x,
            y: child_y,
            width,
            height,
        } = &view.geometry
        {
            minimum_width = minimum_width.max(child_x + width - x + 12.0);
            minimum_height = minimum_height.max(child_y + height - y + 12.0);
        }
    }
    Some((minimum_width, minimum_height))
}

fn insert_template_parameter(
    section: &mut ModelingSection,
    owner_id: &str,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    let owner = section
        .elements
        .iter()
        .find(|element| element.id == owner_id)
        .ok_or("template owner missing")?;
    let owner_type = crate::UmlType::from_kind(&owner.kind).ok_or("unsupported template owner")?;
    let signature_kind = if owner_type.is_classifier() {
        "uml.redefinable_template_signature"
    } else {
        "uml.template_signature"
    };
    let signatures = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter_map(|relation| {
            section
                .elements
                .iter()
                .find(|element| element.id == relation.to)
        })
        .filter(|element| crate::templates::is_signature(&element.kind))
        .collect::<Vec<_>>();
    if signatures.len() > 1 {
        return Err(
            "template owner has multiple signatures; repair the model before adding parameters"
                .into(),
        );
    }
    if signatures
        .first()
        .is_some_and(|signature| signature.kind != signature_kind)
    {
        return Err("existing template signature has an incompatible UML metaclass".into());
    }
    // Extending inherited signatures needs parameter substitution and ordering, not a local copy.
    let model = section.to_model(project_id, "Template validation");
    let graph = crate::GraphIndex::new(&model);
    for parent in section
        .elements
        .iter()
        .filter(|candidate| candidate.id != owner_id)
    {
        if graph
            .shortest_path(owner_id, &parent.id, Some("uml.generalization"))
            .map_err(|error| error.to_string())?
            .is_some()
            && section.relations.iter().any(|relation| {
                relation.kind == "owns"
                    && relation.from == parent.id
                    && section.elements.iter().any(|element| {
                        element.id == relation.to && crate::templates::is_signature(&element.kind)
                    })
            })
        {
            return Err("inherited template signature extension is not implemented; refusing an incomplete signature".into());
        }
    }
    let existing_signature = signatures.first().map(|signature| signature.id.clone());
    let owner_name = owner.name.clone();
    let signature_id = existing_signature
        .clone()
        .map(Ok)
        .unwrap_or_else(|| fresh(section, "template-signature", revision, project_id))?;
    let signature_ownership = fresh(section, "template-signature-owns", revision, project_id)?;
    let parameter_id = fresh(section, "template-parameter", revision, project_id)?;
    let parameter_ownership = fresh(section, "template-parameter-owns", revision, project_id)?;
    let exposed_id = fresh(section, "parametered-classifier", revision, project_id)?;
    let exposed_ownership = fresh(section, "parametered-classifier-owns", revision, project_id)?;
    let reference_id = fresh(section, "parametered-element", revision, project_id)?;
    let parameter_name = name(section, "T")?;
    let make_element = |id: String, kind: &str, name: String, properties| Element {
        id,
        kind: kind.into(),
        name,
        label: None,
        description: None,
        properties,
        annotations: Vec::new(),
        implementations: Vec::new(),
        content: None,
    };
    if existing_signature.is_none() {
        section.elements.push(make_element(
            signature_id.clone(),
            signature_kind,
            format!("{owner_name} template signature"),
            std::collections::BTreeMap::from([(
                crate::templates::GENERATED_SIGNATURE.into(),
                serde_json::json!({"name": format!("{owner_name} template signature")}),
            )]),
        ));
        section.relations.push(Relation {
            id: signature_ownership,
            kind: "owns".into(),
            from: owner_id.into(),
            to: signature_id.clone(),
            properties: Default::default(),
        });
    }
    section.elements.push(make_element(
        parameter_id.clone(),
        "uml.classifier_template_parameter",
        parameter_name.clone(),
        std::collections::BTreeMap::from([("allowSubstitutable".into(), serde_json::json!(false))]),
    ));
    section.elements.push(make_element(
        exposed_id.clone(),
        "uml.class",
        parameter_name,
        Default::default(),
    ));
    for (id, kind, from, to) in [
        (
            parameter_ownership,
            "owns",
            signature_id,
            parameter_id.clone(),
        ),
        (
            exposed_ownership,
            "owns",
            parameter_id.clone(),
            exposed_id.clone(),
        ),
        (
            reference_id,
            "uml.parametered_element",
            parameter_id,
            exposed_id,
        ),
    ] {
        section.relations.push(Relation {
            id,
            kind: kind.into(),
            from,
            to,
            properties: Default::default(),
        });
    }
    super::members::grow_classifier_views(section, owner_id);
    Ok(())
}

pub(super) fn insert_feature(
    section: &mut ModelingSection,
    diagram_id: &str,
    owner_id: &str,
    action: UmlQuickCreate,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    let owner = section
        .elements
        .iter()
        .find(|element| element.id == owner_id)
        .ok_or("feature owner missing")?;
    if !action.accepts(&owner.kind) {
        return Err("element type cannot own this feature kind".into());
    }
    if action == UmlQuickCreate::TemplateParameter {
        if !section
            .diagrams
            .iter()
            .any(|diagram| diagram.id == diagram_id)
        {
            return Err("diagram missing".into());
        }
        return insert_template_parameter(section, owner_id, revision, project_id);
    }
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram missing")?;
    let owner_view = diagram
        .views
        .iter()
        .find(|view| view.subject.as_deref() == Some(owner_id))
        .ok_or("feature owner has no View in diagram")?;
    let z_index = diagram
        .views
        .iter()
        .map(|view| view.z_index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("z-index overflow")?;
    let (x, y, width, height) = match &owner_view.geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => (*x, *y, *width, *height),
        _ => return Err("feature owner geometry must be Rect".into()),
    };
    let (kind, prefix, shape) = match action {
        UmlQuickCreate::TemplateParameter => {
            return Err("template parameters require a signature".into());
        }
        UmlQuickCreate::Port => ("uml.port", "Port", Some((16.0, 16.0))),
        UmlQuickCreate::Part => ("uml.part", "Part", Some((86.0, 48.0))),
        _ => return Err("not a contained feature action".into()),
    };
    let previous = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter(|relation| {
            section
                .elements
                .iter()
                .any(|element| element.id == relation.to && element.kind == kind)
        })
        .count();
    let node_id = fresh(section, "feature", revision, project_id)?;
    let owner_relation_id = fresh(section, "owns", revision, project_id)?;
    let label = name(section, prefix)?;
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("visibility".into(), serde_json::json!("public"));
    if action == UmlQuickCreate::Port {
        properties.insert("isService".into(), serde_json::json!(false));
    }
    if action == UmlQuickCreate::Part {
        properties.insert("type".into(), serde_json::json!(""));
    }
    section.elements.push(Element {
        id: node_id.clone(),
        kind: kind.into(),
        name: label,
        label: None,
        description: None,
        properties,
        annotations: Vec::new(),
        implementations: Vec::new(),
        content: None,
    });
    section.relations.push(Relation {
        id: owner_relation_id,
        kind: "owns".into(),
        from: owner_id.into(),
        to: node_id.clone(),
        properties: Default::default(),
    });
    if let Some((element_width, element_height)) = shape {
        let view_id = fresh(section, "feature-view", revision, project_id)?;
        let (position_x, position_y) = if action == UmlQuickCreate::Port {
            (
                x + width - element_width / 2.0,
                y + height * 0.3 + previous as f64 * 24.0,
            )
        } else {
            (
                x + 12.0 + (previous % 2) as f64 * 94.0,
                y + 38.0 + (previous / 2) as f64 * 52.0,
            )
        };
        section
            .diagrams
            .iter_mut()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("diagram missing")?
            .views
            .push(View {
                id: view_id,
                subject: Some(node_id),
                label: None,
                geometry: Geometry::Rect {
                    x: position_x,
                    y: position_y,
                    width: element_width,
                    height: element_height,
                },
                style: Style::default(),
                z_index,
            });
    }
    if action == UmlQuickCreate::Part
        && let Some((minimum_width, minimum_height)) =
            class_minimum_bounds(section, diagram_id, owner_id)
        && let Some(view) = section
            .diagrams
            .iter_mut()
            .find(|diagram| diagram.id == diagram_id)
            .and_then(|diagram| {
                diagram
                    .views
                    .iter_mut()
                    .find(|view| view.subject.as_deref() == Some(owner_id))
            })
        && let Geometry::Rect { width, height, .. } = &mut view.geometry
    {
        *width = (*width).max(minimum_width);
        *height = (*height).max(minimum_height);
    }
    if matches!(action, UmlQuickCreate::Port | UmlQuickCreate::Part) {
        reposition_ports(section, diagram_id, owner_id);
        if action == UmlQuickCreate::Part {
            crate::reroute_relationships(section, diagram_id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ModelingSection {
        let project = serde_json::from_str(include_str!("../../fixtures/todo.json")).unwrap();
        ModelingSection::from_model(&project)
    }
    #[test]
    fn contained_views_follow_class_move_and_port_reanchors_on_resize() {
        let mut section = fixture();
        let diagram_id = section.diagrams[0].id.clone();
        insert_feature(
            &mut section,
            &diagram_id,
            "type-task",
            UmlQuickCreate::Port,
            0,
            "test-project",
        )
        .unwrap();
        insert_feature(
            &mut section,
            &diagram_id,
            "type-task",
            UmlQuickCreate::Part,
            1,
            "test-project",
        )
        .unwrap();
        let children = contained_ids(&section, "type-task");
        assert_eq!(children.len(), 2);
        let before = section.diagrams[0]
            .views
            .iter()
            .filter(|view| {
                view.subject
                    .as_ref()
                    .is_some_and(|id| children.contains(id))
            })
            .map(|view| (view.id.clone(), view.geometry.clone()))
            .collect::<Vec<_>>();
        for view in &mut section.diagrams[0].views {
            if view.subject.as_deref() == Some("type-task") {
                view.geometry.translate(35.0, 20.0);
            }
        }
        translate_contained_views(&mut section, &diagram_id, "type-task", 35.0, 20.0);
        for (id, old) in before {
            let actual = &section.diagrams[0]
                .views
                .iter()
                .find(|view| view.id == id)
                .unwrap()
                .geometry;
            let (
                Geometry::Rect {
                    x: old_x, y: old_y, ..
                },
                Geometry::Rect { x, y, .. },
            ) = (old, actual)
            else {
                panic!("contained feature must remain a rectangle")
            };
            assert!((*x - old_x - 35.0).abs() < 1e-9);
            assert!((*y - old_y - 20.0).abs() < 1e-9);
        }
        let bounds = class_minimum_bounds(&section, &diagram_id, "type-task").unwrap();
        assert!(bounds.0 > 80.0 && bounds.1 > 48.0);
        for view in &mut section.diagrams[0].views {
            if view.subject.as_deref() == Some("type-task")
                && let Geometry::Rect { width, .. } = &mut view.geometry
            {
                *width += 90.0;
            }
        }
        reposition_ports(&mut section, &diagram_id, "type-task");
        let class_right = section.diagrams[0]
            .views
            .iter()
            .find(|view| view.subject.as_deref() == Some("type-task"))
            .and_then(|view| match &view.geometry {
                Geometry::Rect { x, width, .. } => Some(x + width),
                _ => None,
            })
            .unwrap();
        let port_id = section
            .elements
            .iter()
            .find(|element| element.kind == "uml.port")
            .unwrap()
            .id
            .as_str();
        let port_center = section.diagrams[0]
            .views
            .iter()
            .find(|view| view.subject.as_deref() == Some(port_id))
            .and_then(|view| match &view.geometry {
                Geometry::Rect { x, width, .. } => Some(x + width / 2.0),
                _ => None,
            })
            .unwrap();
        assert!((port_center - class_right).abs() < 1e-9);
    }
    #[test]
    fn template_port_part_are_owned_and_visually_distinct() {
        let mut section = fixture();
        for (index, action) in [
            UmlQuickCreate::TemplateParameter,
            UmlQuickCreate::Port,
            UmlQuickCreate::Part,
        ]
        .into_iter()
        .enumerate()
        {
            let diagram_id = section.diagrams[0].id.clone();
            insert_feature(
                &mut section,
                &diagram_id,
                "type-task",
                action,
                index as u64,
                "test-project",
            )
            .unwrap();
        }
        for kind in ["uml.classifier_template_parameter", "uml.port", "uml.part"] {
            let element = section
                .elements
                .iter()
                .find(|element| element.kind == kind)
                .unwrap();
            assert!(section.relations.iter().any(|relation| {
                relation.kind == "owns"
                    && (relation.from == "type-task"
                        || (kind == "uml.classifier_template_parameter"
                            && section.elements.iter().any(|owner| {
                                owner.id == relation.from
                                    && owner.kind == "uml.redefinable_template_signature"
                            })))
                    && relation.to == element.id
            }));
            let has_view = section
                .diagrams
                .iter()
                .flat_map(|diagram| &diagram.views)
                .any(|view| view.subject.as_deref() == Some(element.id.as_str()));
            assert_eq!(has_view, kind != "uml.classifier_template_parameter");
        }
    }
}
