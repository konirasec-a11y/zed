//! Pure model commands. The caller owns concurrency; this module owns mutations.
mod kinds;
mod members;
mod features;
mod quick_edit;
mod name_expression;

pub use kinds::{ModelingElementKind, ModelingTool};
pub use members::ModelingMemberKind;
pub use name_expression::format as name_expression_text;

use crate::{Element, Geometry, Graph, ModelingRelationKind, ModelingSection, Project,
    Relation, RelationshipGraph, Style, UmlQuickCommand, UmlType, View,
    classifier_contents, relation_kind_for_creation, reroute_relationships, route_relationship};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum Command {
    CreateElement { diagram_id: String, kind: ModelingElementKind, name: Option<String>, x: f64, y: f64 },
    CreateRelation { diagram_id: String, kind: ModelingRelationKind, from: String, to: String },
    AddMember { owner_id: String, kind: ModelingMemberKind },
    QuickEdit { diagram_id: String, element_id: String, action: UmlQuickCommand },
    MoveView { view_id: String, dx: f64, dy: f64 },
    ResizeView { view_id: String, width: f64, height: f64 },
    DeleteElement { element_id: String },
    SetProperty { element_id: String, path: String, value: Value },
    SetNameExpression { element_id: String, expression: String },
    Rename { element_id: String, name: String },
}

/// Failure never changes the input. `revision` seeds IDs; it is NOT a lock.
/// The adapter must check its revision and publish with compare-and-swap.
pub fn apply(project: &Project, command: &Command, revision: u64) -> Result<Project, String> {
    let mut section = ModelingSection::from_model(project);
    match command {
        Command::CreateElement { diagram_id, kind, name, x, y } =>
            create_element(&mut section, diagram_id, *kind, name.as_deref(), *x, *y, revision)?,
        Command::CreateRelation { diagram_id, kind, from, to } =>
            create_relation(&mut section, diagram_id, *kind, from, to, revision)?,
        Command::AddMember { owner_id, kind } => {
            members::append_member(&mut section, owner_id, *kind, revision)?;
        }
        Command::QuickEdit { diagram_id, element_id, action } =>
            quick_edit::apply(&mut section, diagram_id, element_id, *action, revision)?,
        Command::MoveView { view_id, dx, dy } => move_view(&mut section, view_id, *dx, *dy)?,
        Command::ResizeView { view_id, width, height } => resize_view(&mut section, view_id, *width, *height)?,
        Command::DeleteElement { element_id } => remove_element(&mut section, element_id)?,
        Command::SetProperty { element_id, path, value } =>
            crate::set_inspected_property(&mut section, element_id, path, value.clone())?,
        Command::SetNameExpression { element_id, expression } =>
            name_expression::apply(&mut section, element_id, expression)?,
        Command::Rename { element_id, name } => {
            if name.trim().is_empty() { return Err("name must not be blank".into()); }
            section.elements.iter_mut().find(|element| element.id == *element_id)
                .ok_or("MDE element not found")?.name = name.clone();
        }
    }
    let mut updated = section.to_model(&project.id, &project.name);
    updated.schema_version = project.schema_version;
    Graph::new(&updated).validate().map_err(|error| error.to_string())?;
    Ok(updated)
}

fn fresh_ids<const N: usize>(section: &ModelingSection, prefixes: [&str; N], revision: u64) -> Result<[String; N], String> {
    let mut sequence = revision.checked_add(1).ok_or("identifier overflow")?;
    loop {
        let ids = prefixes.map(|prefix| format!("{prefix}-{sequence}"));
        let occupied = ids.iter().any(|id|
            section.elements.iter().any(|element| &element.id == id)
                || section.relations.iter().any(|relation| &relation.id == id)
                || section.diagrams.iter().any(|diagram| &diagram.id == id
                    || diagram.views.iter().any(|view| &view.id == id)));
        if !occupied { return Ok(ids); }
        sequence = sequence.checked_add(1).ok_or("identifier overflow")?;
    }
}

fn create_element(section: &mut ModelingSection, diagram_id: &str, kind: ModelingElementKind,
    name: Option<&str>, x: f64, y: f64, revision: u64) -> Result<(), String> {
    if !x.is_finite() || !y.is_finite() { return Err("element position must be finite".into()); }
    let name = name.unwrap_or(kind.default_name());
    if name.trim().is_empty() { return Err("element name must not be empty".into()); }
    if !section.diagrams.iter().any(|diagram| diagram.id == diagram_id) { return Err("diagram not found".into()); }
    let [element_id, view_id] = fresh_ids(section, ["mde-element", "mde-view"], revision)?;
    let diagram = section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let z_index = diagram.views.iter().map(|view| view.z_index).max().unwrap_or(0).checked_add(1).ok_or("diagram z-index overflow")?;
    let (width, height) = kind.initial_size();
    diagram.views.push(View { id: view_id, subject: Some(element_id.clone()), label: None,
        geometry: Geometry::Rect { x, y, width, height }, style: Style::default(), z_index });
    section.elements.push(Element { id: element_id, kind: kind.model_kind().into(), name: name.into(),
        label: None, description: None, properties: Default::default(), annotations: Vec::new(),
        implementations: Vec::new(), content: None });
    Ok(())
}

fn create_relation(section: &mut ModelingSection, diagram_id: &str, kind: ModelingRelationKind,
    from: &str, to: &str, revision: u64) -> Result<(), String> {
    let source = section.elements.iter().find(|element| element.id == from).ok_or("source element not found")?;
    let target = section.elements.iter().find(|element| element.id == to).ok_or("target element not found")?;
    kind.validate_endpoints(&source.kind, &target.kind)?;
    let geometry = route_relationship(section, diagram_id, from, to)?;
    let [id, view_id] = fresh_ids(section, ["mde-uml-relation", "mde-uml-relation-view"], revision)?;
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("routing".into(), serde_json::json!("automatic"));
    let (semantic_kind, aggregation) = relation_kind_for_creation(kind);
    if let Some(aggregation) = aggregation { properties.insert("end1Aggregation".into(), serde_json::json!(aggregation)); }
    section.relations.push(Relation { id: id.clone(), kind: semantic_kind.into(), from: from.into(), to: to.into(), properties });
    let diagram = section.diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let z_index = diagram.views.iter().map(|view| view.z_index).max().unwrap_or(0).checked_add(1).ok_or("z-index overflow")?;
    diagram.views.push(View { id: view_id, subject: Some(id), label: None, geometry, style: Style::default(), z_index });
    reroute_relationships(section, diagram_id);
    Ok(())
}

fn move_view(section: &mut ModelingSection, view_id: &str, dx: f64, dy: f64) -> Result<(), String> {
    if !dx.is_finite() || !dy.is_finite() { return Err("view displacement must be finite".into()); }
    let diagram_id = section.diagrams.iter().find(|diagram| diagram.views.iter().any(|view| view.id == view_id))
        .map(|diagram| diagram.id.clone()).ok_or("view not found")?;
    let view = section.diagrams.iter_mut().flat_map(|diagram| &mut diagram.views)
        .find(|view| view.id == view_id).ok_or("view not found")?;
    let subject = view.subject.clone();
    view.geometry.translate(dx, dy);
    if let Some(subject) = subject { features::translate_contained_views(section, &diagram_id, &subject, dx, dy); }
    reroute_relationships(section, &diagram_id);
    Ok(())
}

fn resize_view(section: &mut ModelingSection, view_id: &str, width: f64, height: f64) -> Result<(), String> {
    if !width.is_finite() || !height.is_finite() || width < 80.0 || height < 48.0 {
        return Err("view dimensions must be finite and at least 80x48".into());
    }
    let diagram_id = section.diagrams.iter().find(|diagram| diagram.views.iter().any(|view| view.id == view_id))
        .map(|diagram| diagram.id.clone()).ok_or("view not found")?;
    let owner_id = section.diagrams.iter().flat_map(|diagram| &diagram.views)
        .find(|view| view.id == view_id).and_then(|view| view.subject.clone());
    let minimum = owner_id.as_deref().and_then(|id| features::class_minimum_bounds(section, &diagram_id, id));
    let minimum_height = owner_id.as_deref().and_then(|id| section.elements.iter().find(|element| element.id == id))
        .filter(|element| UmlType::from_kind(&element.kind) == Some(UmlType::Interface))
        .map(|element| classifier_contents(section, &element.id).minimum_interface_height()).unwrap_or(48.0);
    let view = section.diagrams.iter_mut().flat_map(|diagram| &mut diagram.views)
        .find(|view| view.id == view_id).ok_or("view not found")?;
    let (minimum_width, minimum_part_height) = minimum.unwrap_or((80.0, 48.0));
    match &mut view.geometry {
        Geometry::Rect { width: current_width, height: current_height, .. } => {
            *current_width = width.max(minimum_width);
            *current_height = height.max(minimum_height).max(minimum_part_height);
        }
        _ => return Err("only node Views can be resized with this command".into()),
    }
    if let Some(owner_id) = owner_id { features::reposition_ports(section, &diagram_id, &owner_id); }
    reroute_relationships(section, &diagram_id);
    Ok(())
}

fn remove_element(section: &mut ModelingSection, id: &str) -> Result<(), String> {
    if section.relations.iter().any(|relation| relation.id == id) {
        if !section.diagrams.iter().any(|diagram| RelationshipGraph::new(section, &diagram.id)
            .visible_relations().iter().any(|relation| relation.id == id)) {
            return Err("structural relationship cannot be deleted directly".into());
        }
        section.relations.retain(|relation| relation.id != id);
        for diagram in &mut section.diagrams { diagram.views.retain(|view| view.subject.as_deref() != Some(id)); }
        return Ok(());
    }
    if !section.elements.iter().any(|element| element.id == id) { return Err("element not found".into()); }
    let mut deleted = HashSet::from([id.to_owned()]);
    loop {
        let mut changed = false;
        for relation in &section.relations {
            if relation.kind == "owns" && deleted.contains(&relation.from) && deleted.insert(relation.to.clone()) { changed = true; }
        }
        if !changed { break; }
    }
    let incident: HashSet<_> = section.relations.iter()
        .filter(|relation| deleted.contains(&relation.from) || deleted.contains(&relation.to)).map(|relation| relation.id.clone()).collect();
    section.elements.retain(|element| !deleted.contains(&element.id));
    section.relations.retain(|relation| !incident.contains(&relation.id));
    for diagram in &mut section.diagrams {
        diagram.views.retain(|view| view.subject.as_ref().is_none_or(|subject| !deleted.contains(subject) && !incident.contains(subject)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Project { serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap() }
    #[test]
    fn failed_command_cannot_partially_change_its_source() {
        let project = fixture();
        let before = project.clone();
        for command in [
            Command::DeleteElement { element_id: "missing".into() },
            Command::CreateElement { diagram_id: "missing".into(), kind: ModelingElementKind::Class, name: None, x: 1.0, y: 1.0 },
            Command::SetProperty { element_id: "type-task".into(), path: "id".into(), value: serde_json::json!("changed") },
        ] { assert!(apply(&project, &command, 0).is_err()); assert_eq!(project, before); }
    }
    #[test]
    fn deleting_class_removes_owned_members_views_and_incident_edges_only() {
        let project = apply(&fixture(), &Command::DeleteElement { element_id: "type-task".into() }, 0).unwrap();
        for deleted in ["type-task", "field-title", "field-completed"] { assert!(!project.elements.iter().any(|element| element.id == deleted)); }
        assert!(project.element("type-list").is_some());
        assert!(project.element("file-task").is_some());
        assert!(project.relations.iter().all(|relation| relation.from != "type-task" && relation.to != "type-task"));
        assert!(project.diagrams.iter().all(|diagram| diagram.views.iter().all(|view| !matches!(view.subject.as_deref(), Some("type-task" | "ref-task")))));
    }
    #[test]
    fn toolbox_and_quick_edit_use_the_same_association_end_property() {
        let source = fixture();
        let toolbox = apply(&source, &Command::CreateRelation { diagram_id: "diagram-tasks".into(), kind: ModelingRelationKind::Composition, from: "type-list".into(), to: "type-task".into() }, 0).unwrap();
        let quick = apply(&source, &Command::QuickEdit { diagram_id: "diagram-tasks".into(), element_id: "type-list".into(), action: UmlQuickCommand::AddCompositedClass }, 0).unwrap();
        for project in [toolbox, quick] {
            assert!(project.relations.iter().any(|relation| relation.kind == "uml.association" && relation.properties.get("end1Aggregation") == Some(&serde_json::json!("composite"))));
            Graph::new(&project).validate().unwrap();
        }
    }
}
