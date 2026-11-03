//! Pure model commands. The caller owns concurrency; this module owns mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewOrder {
    BringToFront,
    SendToBack,
    BringForward,
    SendBackward,
}

mod diagrams;
mod features;
mod kinds;
mod members;
mod name_expression;
mod quick_edit;
mod validation;
mod view_style;
mod views;

pub use kinds::{ModelingElementKind, ModelingTool};
pub use members::ModelingMemberKind;
pub use name_expression::format as name_expression_text;

use crate::{
    Element, Geometry, Graph, ModelingRelationKind, ModelingSection, Project, Relation,
    RelationshipGraph, StructuralDiagramKind, Style, UmlQuickCreate, UmlType, View,
    classifier_contents, relation_kind_for_creation, reroute_relationships, route_relationship,
};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum Command {
    AdoptRootModel,
    SetOwner { element_id: String, owner_id: Option<String> },
    CreateDiagram {
        kind: StructuralDiagramKind,
        name: String,
        model_elements: Vec<String>,
    },
    RenameDiagram {
        diagram_id: String,
        name: String,
    },
    DeleteDiagram {
        diagram_id: String,
    },
    CreateElement {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
        x: f64,
        y: f64,
    },
    CreateRelation {
        diagram_id: String,
        kind: ModelingRelationKind,
        from: String,
        to: String,
    },
    AddMember {
        owner_id: String,
        kind: ModelingMemberKind,
    },
    MoveParameter { parameter_id: String, up: bool },
    MigrateAssociation { association_id: String },
    SetRedefinedElements { element_id: String, redefined_ids: Vec<String> },
    SetAssociationMemberEnds { association_id: String, member_end_ids: Vec<String> },
    QuickEdit {
        diagram_id: String,
        element_id: String,
        action: UmlQuickCreate,
    },
    PlaceElementView {
        diagram_id: String,
        element_id: String,
        x: f64,
        y: f64,
    },
    RemoveView {
        diagram_id: String,
        view_id: String,
    },
    ReorderView {
        diagram_id: String,
        view_id: String,
        order: ViewOrder,
    },
    SetViewStyle {
        diagram_id: String,
        view_id: String,
        style: Style,
    },
    SetViewRouting {
        diagram_id: String,
        view_id: String,
        routing: crate::RelationshipRouting,
    },
    MoveView {
        view_id: String,
        dx: f64,
        dy: f64,
    },
    ResizeView {
        view_id: String,
        width: f64,
        height: f64,
    },
    DeleteElement {
        element_id: String,
    },
    SetProperty {
        element_id: String,
        path: String,
        value: Value,
    },
    SetNameExpression {
        element_id: String,
        expression: String,
    },
    Rename {
        element_id: String,
        name: String,
    },
}

/// Failure never changes the input. `revision` seeds IDs; it is NOT a lock.
/// The adapter must check its revision and publish with compare-and-swap.
pub fn apply(project: &Project, command: &Command, revision: u64) -> Result<Project, String> {
    let previous_section = ModelingSection::from_model(project);
    let mut section = previous_section.clone();
    if let Command::SetProperty { element_id, path, .. } = command
        && crate::uml::associations::is_projection_path(path)
        && section.relations.iter().any(|relation| relation.id == *element_id
            && crate::uml::root::relationship_metaclass(&relation.kind) == Some("Association")
            && !relation.properties.contains_key("memberEnd"))
    {
        crate::uml::associations::migrate(&mut section, &project.id, element_id)?;
    }
    match command {
        Command::SetOwner { element_id, owner_id } => crate::uml::root::reparent(&mut section, &project.id, element_id, owner_id.as_deref())?,
        Command::AdoptRootModel => { crate::uml::root::adopt_root_model(&mut section, &project.id, &project.name)?; }
        Command::CreateDiagram {
            kind,
            name,
            model_elements,
        } => diagrams::create(&mut section, project, *kind, name, model_elements, revision)?,
        Command::RenameDiagram { diagram_id, name } => {
            diagrams::rename(&mut section, diagram_id, name)?
        }
        Command::DeleteDiagram { diagram_id } => diagrams::delete(&mut section, diagram_id)?,
        Command::CreateElement { .. } => {
            create_element(&mut section, command, revision, &project.id)?
        }
        Command::CreateRelation {
            diagram_id,
            kind,
            from,
            to,
        } => create_relation(
            &mut section,
            diagram_id,
            *kind,
            from,
            to,
            revision,
            &project.id,
        )?,
        Command::AddMember { owner_id, kind } => {
            members::append_member(&mut section, owner_id, *kind, revision, &project.id)?;
        }
        Command::MoveParameter { parameter_id, up } => crate::uml::operations::move_parameter(&mut section, parameter_id, *up)?,
        Command::SetRedefinedElements { element_id, redefined_ids } => crate::uml::classifiers::set_redefined_elements(&mut section, element_id, redefined_ids)?,
        Command::MigrateAssociation { association_id } => crate::uml::associations::migrate(&mut section, &project.id, association_id)?,
        Command::SetAssociationMemberEnds { association_id, member_end_ids } => crate::uml::associations::set_member_ends(&mut section, association_id, member_end_ids)?,
        Command::QuickEdit {
            diagram_id,
            element_id,
            action,
        } => quick_edit::apply(
            &mut section,
            diagram_id,
            element_id,
            *action,
            revision,
            &project.id,
        )?,
        Command::PlaceElementView {
            diagram_id,
            element_id,
            x,
            y,
        } => views::place(
            &mut section,
            diagram_id,
            element_id,
            *x,
            *y,
            revision,
            &project.id,
        )?,
        Command::RemoveView {
            diagram_id,
            view_id,
        } => views::remove(&mut section, diagram_id, view_id)?,
        Command::ReorderView { diagram_id, view_id, order } => view_style::reorder(&mut section, diagram_id, view_id, *order)?,
        Command::SetViewStyle {
            diagram_id,
            view_id,
            style,
        } => view_style::set_style(&mut section, diagram_id, view_id, style)?,
        Command::SetViewRouting {
            diagram_id,
            view_id,
            routing,
        } => view_style::set_routing(&mut section, diagram_id, view_id, *routing)?,
        Command::MoveView { view_id, dx, dy } => move_view(&mut section, view_id, *dx, *dy)?,
        Command::ResizeView {
            view_id,
            width,
            height,
        } => resize_view(&mut section, view_id, *width, *height)?,
        Command::DeleteElement { element_id } => remove_element(&mut section, element_id)?,
        Command::SetProperty {
            element_id,
            path,
            value,
        } => {
            if path != "name"
                || !crate::templates::rename_parameter(
                    &mut section,
                    element_id,
                    value.as_str().ok_or("expected string")?,
                )?
            {
                crate::set_inspected_property(&mut section, element_id, path, value.clone())?;
            }
        }
        Command::SetNameExpression {
            element_id,
            expression,
        } => name_expression::apply(&mut section, element_id, expression)?,
        Command::Rename { element_id, name } => {
            if !crate::templates::rename_parameter(&mut section, element_id, name)? {
                section
                    .elements
                    .iter_mut()
                    .find(|element| element.id == *element_id)
                    .ok_or("MDE element not found")?
                    .name = name.clone();
            }
        }
    }
    let presentation_only = matches!(command, Command::SetViewStyle { .. } | Command::SetViewRouting { .. } | Command::ReorderView { .. });
    if !presentation_only {
        crate::uml::associations::materialize_new(project, &mut section)?;
        crate::uml::associations::synchronize_changes(&previous_section, &mut section)?;
        crate::uml::associations::validate_changes(&previous_section, &section)?;
    }
    let mut updated = section.to_model(&project.id, &project.name);
    updated.schema_version = project.schema_version;
    let graph = Graph::new(&updated);
    if presentation_only { graph.validate_presentation() } else { graph.validate() }
        .map_err(|error| error.to_string())?;
    if !presentation_only {
        validation::new_relationships(project, &updated)?;
        crate::templates::validate_changes(project, &updated)?;
        crate::uml::root::validate_changes(project, &updated)?;
    }
    Ok(updated)
}

fn fresh_ids<const N: usize>(
    section: &ModelingSection,
    prefixes: [&str; N],
    revision: u64,
    project_id: &str,
) -> Result<[String; N], String> {
    let mut sequence = revision.checked_add(1).ok_or("identifier overflow")?;
    loop {
        let ids = prefixes.map(|prefix| format!("{prefix}-{sequence}"));
        let occupied = ids.iter().any(|id| {
            id == project_id
                || section.elements.iter().any(|element| &element.id == id)
                || section.relations.iter().any(|relation| &relation.id == id)
                || section.diagrams.iter().any(|diagram| {
                    &diagram.id == id || diagram.views.iter().any(|view| &view.id == id)
                })
        });
        if !occupied {
            return Ok(ids);
        }
        sequence = sequence.checked_add(1).ok_or("identifier overflow")?;
    }
}

fn create_element(
    section: &mut ModelingSection,
    command: &Command,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    let Command::CreateElement {
        diagram_id,
        owner_id,
        kind,
        name,
        x,
        y,
    } = command
    else {
        return Err("expected element creation command".into());
    };
    if !x.is_finite() || !y.is_finite() {
        return Err("element position must be finite".into());
    }
    if ModelingElementKind::DETAILS.contains(kind) && !section.diagrams.iter().any(|diagram|
        diagram.id == *diagram_id && diagram.views.iter().any(|view| view.subject.as_ref() == owner_id.as_ref())) {
        return Err("place the containing Action or CombinedFragment in this diagram before adding its detail".into());
    }
    let requested_name = name.as_deref();
    let mut chosen_name = requested_name.unwrap_or(kind.default_name()).to_owned();
    if kind.requires_owner() && owner_id.is_none() {
        return Err(if kind.requires_nested_owner() {
            "Select the containing Activity, StateMachine, State, Region, or Interaction first"
        } else { "this element requires an explicit Package or Model owner" }.into());
    }
    let resolved_owner = if let Some(owner_id) = owner_id { Some(owner_id.clone()) } else if crate::uml::root::object_metaclass(kind.model_kind()).is_some() {
        Some(crate::uml::root::adopt_root_model(section, project_id, "Model")?)
    } else { None };
    if let Some(owner_id) = &resolved_owner {
        let owner = section
            .elements
            .iter()
            .find(|element| element.id == *owner_id)
            .ok_or("element owner not found")?;
        // This creation slice supports package ownership, not every UML ownership context.
        if !kind.requires_nested_owner() && !matches!(owner.kind.as_str(), "uml.package" | "uml.model") {
            return Err("this creation command requires a Package or Model owner".into());
        }
    }
    if !section
        .diagrams
        .iter()
        .any(|diagram| diagram.id == *diagram_id)
    {
        return Err("diagram not found".into());
    }
    if requested_name.is_none() {
        let base = chosen_name.clone();
        for sequence in 1_u64.. {
            let duplicate = section.elements.iter().any(|element| element.name == chosen_name && section.relations.iter().any(|relation| relation.kind == "owns" && Some(&relation.from) == resolved_owner.as_ref() && relation.to == element.id));
            if !duplicate { break; }
            chosen_name = format!("{base}{sequence}");
        }
    }
    let [element_id, view_id, ownership_id] = fresh_ids(
        section,
        ["mde-element", "mde-view", "mde-ownership"],
        revision,
        project_id,
    )?;
    let diagram = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == *diagram_id)
        .ok_or("diagram not found")?;
    let z_index = diagram
        .views
        .iter()
        .map(|view| view.z_index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("diagram z-index overflow")?;
    diagram.views.push(View {
        id: view_id,
        subject: Some(element_id.clone()),
        label: None,
        geometry: views::initial_geometry(*kind, *x, *y),
        style: Style::default(),
        z_index,
    });
    let mut properties = std::collections::BTreeMap::new();
    if crate::uml::root::object_metaclass(kind.model_kind()).is_some_and(|metaclass|
        crate::uml::metamodel::conforms_to(metaclass, "PackageableElement")) {
        properties.insert("visibility".into(), serde_json::json!("public"));
    }
    if matches!(
        kind,
        ModelingElementKind::Component | ModelingElementKind::Artifact
    ) {
        properties.insert("isAbstract".into(), serde_json::json!(false));
        properties.insert("isFinalSpecialization".into(), serde_json::json!(false));
    }
    if *kind == ModelingElementKind::Component {
        properties.insert("isIndirectlyInstantiated".into(), serde_json::json!(true));
        properties.insert("isActive".into(), serde_json::json!(false));
    }
    if *kind == ModelingElementKind::Pseudostate {
        properties.insert("kind".into(), serde_json::json!("initial"));
    }
    if crate::uml::root::object_metaclass(kind.model_kind()) == Some("Comment") {
        properties.insert("body".into(), Value::String(chosen_name.clone()));
    }
    section.elements.push(Element {
        id: element_id.clone(),
        kind: kind.model_kind().into(),
        name: chosen_name,
        label: None,
        description: None,
        properties,
        annotations: Vec::new(),
        implementations: Vec::new(),
        content: None,
    });
    if let Some(owner_id) = resolved_owner {
        if kind.requires_nested_owner() {
            crate::uml::root::set_owner(section, project_id, &element_id, &owner_id)?;
            features::reposition_boundary_nodes(section, diagram_id, &owner_id);
            features::layout_interaction_operands(section, diagram_id, &owner_id);
        } else {
        section.relations.push(Relation {
            id: ownership_id,
            kind: "owns".into(),
            from: owner_id.clone(),
            to: element_id,
            properties: Default::default(),
        });
        }
    }
    Ok(())
}

fn diagram_relation_owner(section: &ModelingSection, kind: ModelingRelationKind, from: &str, to: &str) -> Result<Option<String>, String> {
    use ModelingRelationKind::*;
    match kind {
        Include | Extend | PackageImport | PackageMerge | Manifestation => return Ok(Some(from.into())),
        Deployment => return Ok(Some(to.into())),
        _ => {}
    }
    let expected = match kind { ControlFlow | ObjectFlow => "Activity", Transition => "Region", _ => return Ok(None) };
    let model = crate::uml::root::Model::from_section(section);
    let ancestor = |id: &str| -> Result<String, String> {
        let mut element = model.element(id).ok_or("relationship endpoint missing")?;
        let mut seen = HashSet::new();
        while seen.insert(element.id().to_owned()) {
            let owner = model.owner(element)?.ok_or_else(|| format!("endpoint has no containing {expected}"))?;
            if owner.conforms_to(expected) { return Ok(owner.id().to_owned()); }
            element = owner;
        }
        Err("cyclic relationship endpoint ownership".into())
    };
    let source_owner = ancestor(from)?;
    if source_owner != ancestor(to)? {
        return Err(format!("this creation tool requires endpoints in the same {expected}"));
    }
    Ok(Some(source_owner))
}

fn create_relation(
    section: &mut ModelingSection,
    diagram_id: &str,
    kind: ModelingRelationKind,
    from: &str,
    to: &str,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    let source = section
        .elements
        .iter()
        .find(|element| element.id == from)
        .ok_or("source element not found")?;
    let target = section
        .elements
        .iter()
        .find(|element| element.id == to)
        .ok_or("target element not found")?;
    kind.validate_endpoints(&source.kind, &target.kind)?;
    let specialized_owner = diagram_relation_owner(section, kind, from, to)?;
    let geometry = route_relationship(section, diagram_id, from, to)?;
    let [id, view_id] = fresh_ids(
        section,
        ["mde-uml-relation", "mde-uml-relation-view"],
        revision,
        project_id,
    )?;
    let mut properties = std::collections::BTreeMap::new();
    if kind == ModelingRelationKind::PackageImport {
        properties.insert("visibility".into(), serde_json::json!("public"));
    }
    let (semantic_kind, aggregation) = relation_kind_for_creation(kind);
    if let Some(aggregation) = aggregation {
        properties.insert("end1Aggregation".into(), serde_json::json!(aggregation));
    }
    section.relations.push(Relation {
        id: id.clone(),
        kind: semantic_kind.into(),
        from: from.into(),
        to: to.into(),
        properties,
    });
    if crate::uml::root::relationship_metaclass(semantic_kind).is_some()
        && !matches!(semantic_kind, "uml.generalization" | "uml.interface_realization" | "uml.template_binding") {
        let owner_id = if let Some(owner) = specialized_owner { owner } else {
            crate::uml::root::adopt_root_model(section, project_id, "Model")?
        };
        crate::uml::root::set_owner(section, project_id, &id, &owner_id)?;
    }
    let diagram = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let z_index = diagram
        .views
        .iter()
        .map(|view| view.z_index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("z-index overflow")?;
    diagram.views.push(View {
        id: view_id,
        subject: Some(id),
        label: None,
        geometry,
        style: Style {
            routing: Some(crate::RelationshipRouting::Automatic),
            ..Style::default()
        },
        z_index,
    });
    reroute_relationships(section, diagram_id);
    Ok(())
}

fn move_view(section: &mut ModelingSection, view_id: &str, dx: f64, dy: f64) -> Result<(), String> {
    if !dx.is_finite() || !dy.is_finite() {
        return Err("view displacement must be finite".into());
    }
    let diagram_id = section
        .diagrams
        .iter()
        .find(|diagram| diagram.views.iter().any(|view| view.id == view_id))
        .map(|diagram| diagram.id.clone())
        .ok_or("view not found")?;
    let view = section
        .diagrams
        .iter_mut()
        .flat_map(|diagram| &mut diagram.views)
        .find(|view| view.id == view_id)
        .ok_or("view not found")?;
    let subject = view.subject.clone();
    view.geometry.translate(dx, dy);
    if let Some(subject) = subject {
        features::translate_contained_views(section, &diagram_id, &subject, dx, dy);
    }
    reroute_relationships(section, &diagram_id);
    Ok(())
}

fn resize_view(
    section: &mut ModelingSection,
    view_id: &str,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if !width.is_finite() || !height.is_finite() || width < 80.0 || height < 48.0 {
        return Err("view dimensions must be finite and at least 80x48".into());
    }
    let diagram_id = section
        .diagrams
        .iter()
        .find(|diagram| diagram.views.iter().any(|view| view.id == view_id))
        .map(|diagram| diagram.id.clone())
        .ok_or("view not found")?;
    let owner_id = section
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.id == view_id)
        .and_then(|view| view.subject.clone());
    let minimum = owner_id
        .as_deref()
        .and_then(|id| features::class_minimum_bounds(section, &diagram_id, id));
    let notation_minimum = owner_id.as_deref()
        .and_then(|id| section.elements.iter().find(|element| element.id == id))
        .map(|element| crate::uml_notation(&element.kind).minimum_size())
        .unwrap_or((80.0, 48.0));
    let minimum_height = owner_id
        .as_deref()
        .and_then(|id| section.elements.iter().find(|element| element.id == id))
        .map(|element| {
            if UmlType::from_kind(&element.kind) == Some(UmlType::Interface) {
                classifier_contents(section, &element.id).minimum_interface_height()
            } else {
                crate::uml_header(
                    &element.kind,
                    element
                        .properties
                        .get("stereotype")
                        .and_then(serde_json::Value::as_str),
                )
                .minimum_height(crate::uml_notation(&element.kind))
            }
        })
        .unwrap_or(48.0);
    let view = section
        .diagrams
        .iter_mut()
        .flat_map(|diagram| &mut diagram.views)
        .find(|view| view.id == view_id)
        .ok_or("view not found")?;
    let (minimum_width, minimum_part_height) = minimum.unwrap_or(notation_minimum);
    match &mut view.geometry {
        Geometry::Rect {
            width: current_width,
            height: current_height,
            ..
        } => {
            *current_width = width.max(minimum_width);
            *current_height = height.max(minimum_height).max(minimum_part_height);
        }
        _ => return Err("only node Views can be resized with this command".into()),
    }
    if let Some(owner_id) = owner_id {
        features::reposition_boundary_nodes(section, &diagram_id, &owner_id);
        features::layout_interaction_operands(section, &diagram_id, &owner_id);
    }
    reroute_relationships(section, &diagram_id);
    Ok(())
}

fn remove_element(section: &mut ModelingSection, id: &str) -> Result<(), String> {
    let semantic = crate::uml::root::Model::from_section(section);
    if semantic.element(id).is_none() {
        if section.diagrams.iter().any(|diagram| RelationshipGraph::new(section, &diagram.id).visible_relations().iter().any(|relation| relation.id == id)) {
            section.relations.retain(|relation| relation.id != id);
            for diagram in &mut section.diagrams { diagram.views.retain(|view| view.subject.as_deref() != Some(id)); }
            return Ok(());
        }
        return Err("element not found or structural link cannot be deleted directly".into());
    }
    let mut deleted = HashSet::from([id.to_owned()]);
    loop {
        let mut changed = false;
        for element in semantic.elements() {
            if semantic.owner(element)?.is_some_and(|owner| deleted.contains(owner.id()))
                && deleted.insert(element.id().to_owned()) { changed = true; }
        }
        for relation in &section.relations {
            if (deleted.contains(&relation.from) || deleted.contains(&relation.to))
                && deleted.insert(relation.id.clone()) { changed = true; }
        }
        if !changed {
            break;
        }
    }
    crate::templates::extend_deletion(section, &mut deleted)?;
    for diagram in &section.diagrams {
        if let Some(context) = diagram
            .model_elements
            .iter()
            .find(|context| deleted.contains(*context))
        {
            return Err(format!(
                "UMLDI_CONTEXT_IN_USE: diagram {} references context {}; remove or update the diagram before deleting this element",
                diagram.id, context
            ));
        }
    }
    let incident: HashSet<_> = section
        .relations
        .iter()
        .filter(|relation| deleted.contains(&relation.from) || deleted.contains(&relation.to))
        .map(|relation| relation.id.clone())
        .collect();
    section
        .elements
        .retain(|element| !deleted.contains(&element.id));
    section
        .relations
        .retain(|relation| !incident.contains(&relation.id) && !deleted.contains(&relation.id));
    if section.root_model.as_ref().is_some_and(|root| deleted.contains(root)) { section.root_model = None; }
    for diagram in &mut section.diagrams {
        diagram.views.retain(|view| {
            view.subject
                .as_ref()
                .is_none_or(|subject| !deleted.contains(subject) && !incident.contains(subject))
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Project {
        serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap()
    }
    #[test]
    fn failed_command_cannot_partially_change_its_source() {
        let project = fixture();
        let before = project.clone();
        for command in [
            Command::DeleteElement {
                element_id: "missing".into(),
            },
            Command::CreateElement {
                owner_id: None,
                diagram_id: "missing".into(),
                kind: ModelingElementKind::Class,
                name: None,
                x: 1.0,
                y: 1.0,
            },
            Command::SetProperty {
                element_id: "type-task".into(),
                path: "id".into(),
                value: serde_json::json!("changed"),
            },
        ] {
            assert!(apply(&project, &command, 0).is_err());
            assert_eq!(project, before);
        }
    }
    #[test]
    fn deleting_class_removes_owned_members_views_and_incident_edges_only() {
        let project = apply(
            &fixture(),
            &Command::DeleteElement {
                element_id: "type-task".into(),
            },
            0,
        )
        .unwrap();
        for deleted in ["type-task", "field-title", "field-completed"] {
            assert!(!project.elements.iter().any(|element| element.id == deleted));
        }
        assert!(project.element("type-list").is_some());
        assert!(project.element("file-task").is_some());
        assert!(
            project
                .relations
                .iter()
                .all(|relation| relation.from != "type-task" && relation.to != "type-task")
        );
        assert!(project.diagrams.iter().all(|diagram| {
            diagram
                .views
                .iter()
                .all(|view| !matches!(view.subject.as_deref(), Some("type-task" | "ref-task")))
        }));
    }
    #[test]
    fn toolbox_and_quick_edit_use_the_same_association_end_property() {
        let source = fixture();
        let toolbox = apply(
            &source,
            &Command::CreateRelation {
                diagram_id: "diagram-tasks".into(),
                kind: ModelingRelationKind::Composition,
                from: "type-list".into(),
                to: "type-task".into(),
            },
            0,
        )
        .unwrap();
        let quick = apply(
            &source,
            &Command::QuickEdit {
                diagram_id: "diagram-tasks".into(),
                element_id: "type-list".into(),
                action: UmlQuickCreate::CompositedClass,
            },
            0,
        )
        .unwrap();
        for project in [toolbox, quick] {
            assert!(
                project
                    .relations
                    .iter()
                    .any(|relation| relation.kind == "uml.association"
                        && relation.properties.get("end1Aggregation")
                            == Some(&serde_json::json!("composite")))
            );
            Graph::new(&project).validate().unwrap();
        }
    }
}

#[cfg(test)]
mod deletion_tests;

#[cfg(test)]
mod placement_tests;

#[cfg(test)]
mod structural_creation_tests;
