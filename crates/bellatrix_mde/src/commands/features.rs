use crate::{Element, Geometry, ModelingSection, Relation, Style, UmlQuickCommand, View};
use std::collections::HashSet;

fn fresh(section: &ModelingSection, prefix: &str, revision: u64) -> Result<String, String> {
    let prefix = format!("mde-class-{prefix}");
    let [id] = super::fresh_ids(section, [prefix.as_str()], revision)?;
    Ok(id)
}

fn name(section: &ModelingSection, prefix: &str) -> Result<String, String> {
    for sequence in 1_u64.. {
        let candidate = format!("{prefix}{sequence}");
        if !section.elements.iter().any(|element| element.name == candidate) { return Ok(candidate); }
    }
    Err("member name overflow".into())
}

fn contained_ids(section: &ModelingSection, owner_id: &str) -> HashSet<String> {
    section.relations.iter().filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter_map(|relation| section.elements.iter().find(|element| element.id == relation.to))
        .filter(|element| matches!(element.kind.as_str(), "uml.port" | "uml.part"))
        .map(|element| element.id.clone()).collect()
}

pub(super) fn translate_contained_views(section: &mut ModelingSection, diagram_id: &str,
    owner_id: &str, dx: f64, dy: f64) {
    let children = contained_ids(section, owner_id);
    if let Some(diagram) = section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id) {
        for view in &mut diagram.views {
            if view.subject.as_ref().is_some_and(|id| children.contains(id)) { view.geometry.translate(dx, dy); }
        }
    }
}

pub(super) fn reposition_ports(section: &mut ModelingSection, diagram_id: &str, owner_id: &str) {
    let port_ids: HashSet<_> = section.relations.iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter_map(|relation| section.elements.iter().find(|element| element.id == relation.to))
        .filter(|element| element.kind == "uml.port").map(|element| element.id.clone()).collect();
    if let Some(diagram) = section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id) {
        let bounds = diagram.views.iter().find(|view| view.subject.as_deref() == Some(owner_id))
            .and_then(|view| match &view.geometry {
                Geometry::Rect { x, y, width, height } => Some((*x, *y, *width, *height)), _ => None,
            });
        if let Some((x, y, width, height)) = bounds {
            for view in &mut diagram.views {
                if !view.subject.as_ref().is_some_and(|id| port_ids.contains(id)) { continue; }
                if let Geometry::Rect { x: child_x, y: child_y, width: child_width, height: child_height } = &mut view.geometry {
                    *child_x = x + width - *child_width / 2.0;
                    *child_y = (*child_y).clamp(y - *child_height / 2.0, y + height - *child_height / 2.0);
                }
            }
        }
    }
}

pub(super) fn class_minimum_bounds(section: &ModelingSection, diagram_id: &str, owner_id: &str) -> Option<(f64, f64)> {
    let diagram = section.diagrams.iter().find(|diagram| diagram.id == diagram_id)?;
    let (x, y) = diagram.views.iter().find(|view| view.subject.as_deref() == Some(owner_id))
        .and_then(|view| match &view.geometry { Geometry::Rect { x, y, .. } => Some((*x, *y)), _ => None })?;
    let part_ids: HashSet<_> = section.relations.iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter_map(|relation| section.elements.iter().find(|element| element.id == relation.to))
        .filter(|element| element.kind == "uml.part").map(|element| element.id.clone()).collect();
    let mut minimum_width: f64 = 80.0;
    let mut minimum_height: f64 = 48.0;
    for view in &diagram.views {
        if !view.subject.as_ref().is_some_and(|id| part_ids.contains(id)) { continue; }
        if let Geometry::Rect { x: child_x, y: child_y, width, height } = &view.geometry {
            minimum_width = minimum_width.max(child_x + width - x + 12.0);
            minimum_height = minimum_height.max(child_y + height - y + 12.0);
        }
    }
    Some((minimum_width, minimum_height))
}

pub(super) fn insert_feature(section: &mut ModelingSection, diagram_id: &str,
    owner_id: &str, action: UmlQuickCommand, revision: u64) -> Result<(), String> {
    let owner = section.elements.iter().find(|element| element.id == owner_id).ok_or("Class missing")?;
    if !matches!(owner.kind.as_str(), "rust.struct" | "uml.class") { return Err("Class specialization requires a Class".into()); }
    let diagram = section.diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("diagram missing")?;
    let owner_view = diagram.views.iter().find(|view| view.subject.as_deref() == Some(owner_id)).ok_or("Class has no View in diagram")?;
    let z_index = diagram.views.iter().map(|view| view.z_index).max().unwrap_or(0).checked_add(1).ok_or("z-index overflow")?;
    let (x, y, width, height) = match &owner_view.geometry {
        Geometry::Rect { x, y, width, height } => (*x, *y, *width, *height),
        _ => return Err("Class geometry must be Rect".into()),
    };
    let (kind, prefix, shape) = match action {
        UmlQuickCommand::AddTemplateParameter => ("uml.template_parameter", "TemplateParameter", None),
        UmlQuickCommand::AddPort => ("uml.port", "Port", Some((16.0, 16.0))),
        UmlQuickCommand::AddPart => ("uml.part", "Part", Some((86.0, 48.0))),
        _ => return Err("not a contained Class specialization".into()),
    };
    let previous = section.relations.iter().filter(|relation| relation.kind == "owns" && relation.from == owner_id)
        .filter(|relation| section.elements.iter().any(|element| element.id == relation.to && element.kind == kind)).count();
    let node_id = fresh(section, "feature", revision)?;
    let owner_relation_id = fresh(section, "owns", revision)?;
    let label = name(section, prefix)?;
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("visibility".into(), serde_json::json!("public"));
    if action == UmlQuickCommand::AddTemplateParameter { properties.insert("parameterType".into(), serde_json::json!("UMLClassifier")); }
    if action == UmlQuickCommand::AddPort { properties.insert("isService".into(), serde_json::json!(false)); }
    if action == UmlQuickCommand::AddPart { properties.insert("type".into(), serde_json::json!("")); }
    section.elements.push(Element { id: node_id.clone(), kind: kind.into(), name: label, label: None,
        description: None, properties, annotations: Vec::new(), implementations: Vec::new(), content: None });
    section.relations.push(Relation { id: owner_relation_id, kind: "owns".into(), from: owner_id.into(),
        to: node_id.clone(), properties: Default::default() });
    if let Some((element_width, element_height)) = shape {
        let view_id = fresh(section, "feature-view", revision)?;
        let (position_x, position_y) = if action == UmlQuickCommand::AddPort {
            (x + width - element_width / 2.0, y + height * 0.3 + previous as f64 * 24.0)
        } else {
            (x + 12.0 + (previous % 2) as f64 * 94.0, y + 38.0 + (previous / 2) as f64 * 52.0)
        };
        section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("diagram missing")?.views.push(View {
            id: view_id, subject: Some(node_id), label: None,
            geometry: Geometry::Rect { x: position_x, y: position_y, width: element_width, height: element_height },
            style: Style::default(), z_index,
        });
    }
    if action == UmlQuickCommand::AddTemplateParameter { super::members::grow_classifier_views(section, owner_id); }
    if action == UmlQuickCommand::AddPart {
        if let Some((minimum_width, minimum_height)) = class_minimum_bounds(section, diagram_id, owner_id) {
            if let Some(view) = section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id)
                .and_then(|diagram| diagram.views.iter_mut().find(|view| view.subject.as_deref() == Some(owner_id))) {
                if let Geometry::Rect { width, height, .. } = &mut view.geometry {
                    *width = (*width).max(minimum_width);
                    *height = (*height).max(minimum_height);
                }
            }
        }
    }
    if matches!(action, UmlQuickCommand::AddPort | UmlQuickCommand::AddPart) {
        reposition_ports(section, diagram_id, owner_id);
        if action == UmlQuickCommand::AddPart { crate::reroute_relationships(section, diagram_id); }
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
        insert_feature(&mut section, &diagram_id, "type-task", UmlQuickCommand::AddPort, 0).unwrap();
        insert_feature(&mut section, &diagram_id, "type-task", UmlQuickCommand::AddPart, 1).unwrap();
        let children = contained_ids(&section, "type-task");
        assert_eq!(children.len(), 2);
        let before = section.diagrams[0].views.iter().filter(|view| view.subject.as_ref().is_some_and(|id| children.contains(id)))
            .map(|view| (view.id.clone(), view.geometry.clone())).collect::<Vec<_>>();
        for view in &mut section.diagrams[0].views {
            if view.subject.as_deref() == Some("type-task") { view.geometry.translate(35.0, 20.0); }
        }
        translate_contained_views(&mut section, &diagram_id, "type-task", 35.0, 20.0);
        for (id, old) in before {
            let actual = &section.diagrams[0].views.iter().find(|view| view.id == id).unwrap().geometry;
            let (Geometry::Rect { x: old_x, y: old_y, .. }, Geometry::Rect { x, y, .. }) = (old, actual) else { panic!("contained feature must remain a rectangle") };
            assert!((*x - old_x - 35.0).abs() < 1e-9);
            assert!((*y - old_y - 20.0).abs() < 1e-9);
        }
        let bounds = class_minimum_bounds(&section, &diagram_id, "type-task").unwrap();
        assert!(bounds.0 > 80.0 && bounds.1 > 48.0);
        for view in &mut section.diagrams[0].views {
            if view.subject.as_deref() == Some("type-task") {
                if let Geometry::Rect { width, .. } = &mut view.geometry { *width += 90.0; }
            }
        }
        reposition_ports(&mut section, &diagram_id, "type-task");
        let class_right = section.diagrams[0].views.iter().find(|view| view.subject.as_deref() == Some("type-task"))
            .and_then(|view| match &view.geometry { Geometry::Rect { x, width, .. } => Some(x + width), _ => None }).unwrap();
        let port_id = section.elements.iter().find(|element| element.kind == "uml.port").unwrap().id.as_str();
        let port_center = section.diagrams[0].views.iter().find(|view| view.subject.as_deref() == Some(port_id))
            .and_then(|view| match &view.geometry { Geometry::Rect { x, width, .. } => Some(x + width / 2.0), _ => None }).unwrap();
        assert!((port_center - class_right).abs() < 1e-9);
    }
    #[test]
    fn template_port_part_are_owned_and_visually_distinct() {
        let mut section = fixture();
        for (index, action) in [UmlQuickCommand::AddTemplateParameter, UmlQuickCommand::AddPort, UmlQuickCommand::AddPart].into_iter().enumerate() {
            let diagram_id = section.diagrams[0].id.clone();
            insert_feature(&mut section, &diagram_id, "type-task", action, index as u64).unwrap();
        }
        for kind in ["uml.template_parameter", "uml.port", "uml.part"] {
            let element = section.elements.iter().find(|element| element.kind == kind).unwrap();
            assert!(section.relations.iter().any(|relation| relation.kind == "owns" && relation.from == "type-task" && relation.to == element.id));
            let has_view = section.diagrams.iter().flat_map(|diagram| &diagram.views).any(|view| view.subject.as_deref() == Some(element.id.as_str()));
            assert_eq!(has_view, kind != "uml.template_parameter");
        }
    }
}
