//! Pure model commands. The caller owns concurrency; this module owns mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewOrder {
    BringToFront,
    SendToBack,
    BringForward,
    SendBackward,
}

#[path = "association_ends.rs"]
mod association_ends;
#[path = "diagrams.rs"]
mod diagrams;
#[path = "features.rs"]
mod features;
#[path = "kinds.rs"]
mod kinds;
#[path = "members.rs"]
mod members;
#[path = "name_expression.rs"]
mod name_expression;
#[path = "quick_edit.rs"]
mod quick_edit;
#[path = "validation.rs"]
mod validation;
#[path = "view_style.rs"]
mod view_style;
#[path = "views.rs"]
mod views;

pub use association_ends::{
    AssociationEndDraft, AssociationEndEdit, AssociationEndOwner, RemovedAssociationEnd,
};
pub use kinds::{ModelingElementKind, ModelingTool};
pub use members::ModelingMemberKind;
pub use name_expression::format as name_expression_text;

use crate::{
    Element, Geometry, Graph, ModelingRelationKind, ModelingSection, Project, Relation,
    RelationshipGraph, StructuralDiagramKind, Style, UmlQuickCreate, UmlType, View,
    classifier_contents, relation_kind_for_creation, reroute_relationships,
};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum Command {
    AdoptRootModel,
    SetOwner {
        element_id: String,
        owner_id: Option<String>,
    },
    SetExplorerParent {
        subject_id: String,
        parent_id: Option<String>,
        parent_occurrence: Option<crate::ViewOccurrence>,
    },
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
    CreateExplorerDiagram {
        kind: StructuralDiagramKind,
        name: String,
        model_elements: Vec<String>,
        parent_id: Option<String>,
        parent_occurrence: Option<crate::ViewOccurrence>,
    },
    CreateModelElement {
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
    },
    CreateElement {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
        x: f64,
        y: f64,
    },
    CreateElementOrVisualDraft {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
        x: f64,
        y: f64,
    },
    CreateRelationOrVisualDraft {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        from: String,
        to: String,
    },
    CreateRelationAtViewsOrVisualDraft {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        source_view_id: String,
        target_view_id: String,
        from: Option<String>,
        to: Option<String>,
    },
    CreateRelationAtViews {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        from: String,
        to: String,
        endpoints: crate::ViewEndpoints,
    },
    CreateRelation {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        from: String,
        to: String,
    },
    AddMember {
        owner_id: String,
        kind: ModelingMemberKind,
    },
    MoveParameter {
        parameter_id: String,
        up: bool,
    },
    MigrateAssociation {
        association_id: String,
    },
    SetRedefinedElements {
        element_id: String,
        redefined_ids: Vec<String>,
    },
    SetSubsettedProperties {
        property_id: String,
        subsetted_ids: Vec<String>,
    },
    SetNamespaceImport {
        import_id: String,
        namespace_id: String,
        target_id: String,
        is_package: bool,
        alias: Option<String>,
        visibility: String,
    },
    SetAssociationEndNavigation {
        association_id: String,
        property_id: String,
        navigation: crate::NavigationKnowledge,
    },
    SetAssociationMemberEnds {
        association_id: String,
        member_end_ids: Vec<String>,
    },
    EditAssociationEnds {
        association_id: String,
        edits: Vec<AssociationEndEdit>,
    },
    QuickEdit {
        diagram_id: String,
        element_id: String,
        action: UmlQuickCreate,
    },
    CreateVisualDraft {
        diagram_id: String,
        notation_kind: String,
        label: Option<String>,
        geometry: Geometry,
        properties: std::collections::BTreeMap<String, Value>,
        parent_view_id: Option<String>,
        source_view_id: Option<String>,
        target_view_id: Option<String>,
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
    SetViewLabel {
        view_id: String,
        label: Option<String>,
    },
    SetViewTiming {
        view_id: String,
        timing: Option<crate::TimingPresentation>,
    },
    SetViewEndpoints {
        diagram_id: String,
        view_id: String,
        endpoints: crate::ViewEndpoints,
    },
    SetViewStyle {
        diagram_id: String,
        view_id: String,
        style: Style,
    },
    SetViewPath {
        diagram_id: String,
        view_id: String,
        geometry: Geometry,
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
pub(crate) fn apply_with_diagram_operations(
    project: &Project,
    command: &Command,
    revision: u64,
    diagram_operations: &dyn crate::diagram_operations::DiagramOperations,
) -> Result<Project, String> {
    ensure_visual_edit_supported(project, command)?;
    if let Command::CreateElementOrVisualDraft {
        diagram_id,
        owner_id,
        kind,
        name,
        x,
        y,
    } = command
    {
        let semantic = Command::CreateElement {
            diagram_id: diagram_id.clone(),
            owner_id: owner_id.clone(),
            kind: *kind,
            name: name.clone(),
            x: *x,
            y: *y,
        };
        return match apply_with_diagram_operations(project, &semantic, revision, diagram_operations)
        {
            Ok(updated) => Ok(updated),
            Err(reason) => {
                let diagram = project
                    .diagrams
                    .iter()
                    .find(|diagram| diagram.id == *diagram_id)
                    .ok_or("diagram not found")?;
                let mut properties = std::collections::BTreeMap::new();
                properties.insert("semantic_creation_diagnostic".into(), Value::String(reason));
                if matches!(kind, ModelingElementKind::Part | ModelingElementKind::Port) {
                    properties.insert("aggregation".into(), serde_json::json!("composite"));
                }
                if let Some(pseudostate) = kind.resolve_pseudostate_kind() {
                    properties.insert("kind".into(), Value::String(pseudostate.into()));
                }
                if let Some(owner) = owner_id {
                    properties.insert("requested_owner_id".into(), Value::String(owner.clone()));
                }
                apply_with_diagram_operations(
                    project,
                    &Command::CreateVisualDraft {
                        diagram_id: diagram_id.clone(),
                        notation_kind: kind.model_kind().into(),
                        label: name.clone(),
                        geometry: views::initial_geometry_in_diagram(*kind, diagram, *x, *y),
                        properties,
                        parent_view_id: None,
                        source_view_id: None,
                        target_view_id: None,
                    },
                    revision,
                    diagram_operations,
                )
            }
        };
    }
    if let Command::CreateRelationAtViewsOrVisualDraft {
        diagram_id,
        owner_id,
        kind,
        source_view_id,
        target_view_id,
        from,
        to,
    } = command
    {
        let diagram = project
            .diagrams
            .iter()
            .find(|diagram| diagram.id == *diagram_id)
            .ok_or("diagram not found")?;
        let source = diagram
            .views
            .iter()
            .find(|view| view.id == *source_view_id)
            .ok_or("source View not found in diagram")?;
        let target = diagram
            .views
            .iter()
            .find(|view| view.id == *target_view_id)
            .ok_or("target View not found in diagram")?;
        if source.subject != *from || target.subject != *to {
            return Err("selected endpoint View subject changed; select endpoints again".into());
        }
        let endpoints = crate::ViewEndpoints {
            source_view_id: Some(source_view_id.clone()),
            target_view_id: Some(target_view_id.clone()),
        };
        let semantic = match (from, to) {
            (Some(from), Some(to)) => apply_with_diagram_operations(
                project,
                &Command::CreateRelationAtViews {
                    diagram_id: diagram_id.clone(),
                    owner_id: owner_id.clone(),
                    kind: *kind,
                    from: from.clone(),
                    to: to.clone(),
                    endpoints,
                },
                revision,
                diagram_operations,
            ),
            _ => Err("relationship endpoint has no semantic subject".into()),
        };
        return match semantic {
            Ok(updated) => Ok(updated),
            Err(reason) => {
                let mut properties = std::collections::BTreeMap::new();
                properties.insert("semantic_creation_diagnostic".into(), Value::String(reason));
                if let Some(owner) = owner_id {
                    properties.insert("requested_owner_id".into(), Value::String(owner.clone()));
                }
                if let Some(from) = from {
                    properties.insert("requested_source_id".into(), Value::String(from.clone()));
                }
                if let Some(to) = to {
                    properties.insert("requested_target_id".into(), Value::String(to.clone()));
                }
                let center = crate::resolve_visual_center(&source.geometry)
                    .ok_or("source View has no center")?;
                let geometry = Geometry::Path {
                    start: center,
                    segments: Vec::new(),
                };
                apply_with_diagram_operations(
                    project,
                    &Command::CreateVisualDraft {
                        diagram_id: diagram_id.clone(),
                        notation_kind: kind.model_kind().into(),
                        label: None,
                        geometry,
                        properties,
                        parent_view_id: None,
                        source_view_id: Some(source_view_id.clone()),
                        target_view_id: Some(target_view_id.clone()),
                    },
                    revision,
                    diagram_operations,
                )
            }
        };
    }
    if let Command::CreateRelationOrVisualDraft {
        diagram_id,
        owner_id,
        kind,
        from,
        to,
    } = command
    {
        let diagram = project
            .diagrams
            .iter()
            .find(|diagram| diagram.id == *diagram_id)
            .ok_or("diagram not found")?;
        let source = diagram
            .views
            .iter()
            .find(|view| view.subject.as_ref() == Some(from))
            .ok_or("source view not found")?;
        let target = diagram
            .views
            .iter()
            .find(|view| view.subject.as_ref() == Some(to))
            .ok_or("target view not found")?;
        return apply_with_diagram_operations(
            project,
            &Command::CreateRelationAtViewsOrVisualDraft {
                diagram_id: diagram_id.clone(),
                owner_id: owner_id.clone(),
                kind: *kind,
                from: Some(from.clone()),
                to: Some(to.clone()),
                source_view_id: source.id.clone(),
                target_view_id: target.id.clone(),
            },
            revision,
            diagram_operations,
        );
    }
    crate::composition_paths(project)?;
    let previous_section = ModelingSection::from_model(project);
    let mut section = previous_section.clone();
    if let Command::SetProperty {
        element_id, path, ..
    } = command
        && crate::uml::associations::is_projection_path(path)
        && section.relations.iter().any(|relation| {
            relation.id == *element_id
                && crate::uml::root::relationship_metaclass(&relation.kind) == Some("Association")
                && !relation.properties.contains_key("memberEnd")
        })
    {
        crate::uml::associations::migrate(&mut section, &project.id, element_id)?;
    }
    match command {
        Command::SetExplorerParent {
            subject_id,
            parent_id,
            parent_occurrence,
        } => crate::hierarchy_placement::set_diagram_parent(
            &mut section,
            subject_id,
            parent_id.as_deref(),
            parent_occurrence.as_ref(),
        )?,
        Command::SetOwner {
            element_id,
            owner_id,
        } => {
            crate::uml::root::reparent(&mut section, &project.id, element_id, owner_id.as_deref())?
        }
        Command::AdoptRootModel => {
            crate::uml::root::adopt_root_model(&mut section, &project.id, &project.name)?;
        }
        Command::CreateDiagram {
            kind,
            name,
            model_elements,
        } => diagrams::create(&mut section, project, *kind, name, model_elements, revision)?,
        Command::CreateExplorerDiagram {
            kind,
            name,
            model_elements,
            parent_id,
            parent_occurrence,
        } => {
            diagrams::create(&mut section, project, *kind, name, model_elements, revision)?;
            let diagram_id = section
                .diagrams
                .last()
                .ok_or("created diagram missing")?
                .id
                .clone();
            crate::hierarchy_placement::set_diagram_parent(
                &mut section,
                &diagram_id,
                parent_id.as_deref(),
                parent_occurrence.as_ref(),
            )?;
        }
        Command::RenameDiagram { diagram_id, name } => {
            diagrams::rename(&mut section, diagram_id, name)?
        }
        Command::DeleteDiagram { diagram_id } => diagrams::delete(&mut section, diagram_id)?,
        Command::CreateRelationOrVisualDraft { .. }
        | Command::CreateRelationAtViewsOrVisualDraft { .. } => {
            return Err("visual relation dispatch failed".into());
        }
        Command::CreateElementOrVisualDraft { .. } => {
            return Err("visual creation dispatch failed".into());
        }
        Command::CreateElement { .. } | Command::CreateModelElement { .. } => {
            create_element(&mut section, command, revision, &project.id)?
        }
        Command::CreateRelation {
            diagram_id,
            owner_id,
            kind,
            from,
            to,
        } => create_relation(
            &mut section,
            diagram_id,
            *kind,
            from,
            to,
            owner_id.as_deref(),
            None,
            revision,
            &project.id,
        )?,
        Command::CreateRelationAtViews {
            diagram_id,
            owner_id,
            kind,
            from,
            to,
            endpoints,
        } => create_relation(
            &mut section,
            diagram_id,
            *kind,
            from,
            to,
            owner_id.as_deref(),
            Some(endpoints),
            revision,
            &project.id,
        )?,
        Command::AddMember { owner_id, kind } => {
            members::append_member(&mut section, owner_id, *kind, revision, &project.id)?;
        }
        Command::MoveParameter { parameter_id, up } => {
            crate::uml::operations::move_parameter(&mut section, parameter_id, *up)?
        }
        Command::SetRedefinedElements {
            element_id,
            redefined_ids,
        } => crate::uml::classifiers::set_redefined_elements(
            &mut section,
            element_id,
            redefined_ids,
        )?,
        Command::SetSubsettedProperties {
            property_id,
            subsetted_ids,
        } => crate::uml::property_subsetting::set_subsetted_properties(
            &mut section,
            property_id,
            subsetted_ids,
        )?,
        Command::SetNamespaceImport {
            import_id,
            namespace_id,
            target_id,
            is_package,
            alias,
            visibility,
        } => crate::uml::namespace_imports::set_namespace_import(
            &mut section,
            import_id,
            namespace_id,
            target_id,
            *is_package,
            alias.as_deref(),
            visibility,
        )?,
        Command::MigrateAssociation { association_id } => {
            crate::uml::associations::migrate(&mut section, &project.id, association_id)?
        }
        Command::SetAssociationEndNavigation {
            association_id,
            property_id,
            navigation,
        } => crate::uml::associations::set_navigation_knowledge(
            &mut section,
            association_id,
            property_id,
            *navigation,
        )?,
        Command::SetAssociationMemberEnds {
            association_id,
            member_end_ids,
        } => {
            crate::uml::associations::set_member_ends(&mut section, association_id, member_end_ids)?
        }
        Command::EditAssociationEnds {
            association_id,
            edits,
        } => association_ends::apply(&mut section, association_id, edits, revision, &project.id)?,
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
        Command::CreateVisualDraft {
            diagram_id,
            notation_kind,
            label,
            geometry,
            properties,
            parent_view_id,
            source_view_id,
            target_view_id,
        } => views::create_draft(
            &mut section,
            diagram_id,
            notation_kind,
            label,
            geometry,
            properties,
            parent_view_id,
            source_view_id,
            target_view_id,
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
        Command::ReorderView {
            diagram_id,
            view_id,
            order,
        } => view_style::reorder(&mut section, diagram_id, view_id, *order)?,
        Command::SetViewLabel { view_id, label } => {
            let view = section
                .diagrams
                .iter_mut()
                .find_map(|diagram| diagram.view_mut(view_id))
                .ok_or("view not found")?;
            view.label = label.clone();
        }
        Command::SetViewTiming { view_id, timing } => {
            views::set_timing(&mut section, view_id, timing)?
        }
        Command::SetViewEndpoints {
            diagram_id,
            view_id,
            endpoints,
        } => views::set_endpoints(&mut section, diagram_id, view_id, endpoints)?,
        Command::SetViewStyle {
            diagram_id,
            view_id,
            style,
        } => view_style::set_style(&mut section, diagram_id, view_id, style)?,
        Command::SetViewPath {
            diagram_id,
            view_id,
            geometry,
        } => view_style::set_path(
            &mut section,
            diagram_id,
            view_id,
            geometry,
            diagram_operations,
        )?,
        Command::SetViewRouting {
            diagram_id,
            view_id,
            routing,
        } => view_style::set_routing(
            &mut section,
            diagram_id,
            view_id,
            *routing,
            diagram_operations,
        )?,
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
    if let Command::MoveView { view_id, .. } | Command::ResizeView { view_id, .. } = command
        && let Some(diagram) = previous_section
            .diagrams
            .iter()
            .find(|diagram| diagram.all_views().any(|view| view.id == *view_id))
    {
        // Reanchor only after an authored geometry change, never while projecting imported paths.
        diagram_operations.reanchor_manual_paths(&previous_section, &mut section, &diagram.id)?;
    }
    let draft_view_edit = match command {
        Command::MoveView { view_id, .. } | Command::ResizeView { view_id, .. } => project
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .any(|view| view.id == *view_id && view.draft.is_some()),
        Command::RemoveView {
            diagram_id,
            view_id,
        } => project
            .diagrams
            .iter()
            .find(|diagram| diagram.id == *diagram_id)
            .is_some_and(|diagram| {
                diagram
                    .views
                    .iter()
                    .any(|view| view.id == *view_id && view.draft.is_some())
            }),
        _ => false,
    };
    let presentation_only = draft_view_edit
        || matches!(
            command,
            Command::CreateDiagram { .. }
                | Command::CreateExplorerDiagram { .. }
                | Command::CreateVisualDraft { .. }
                | Command::SetExplorerParent { .. }
                | Command::SetViewLabel { .. }
                | Command::SetViewTiming { .. }
                | Command::SetViewEndpoints { .. }
                | Command::SetViewStyle { .. }
                | Command::SetViewRouting { .. }
                | Command::SetViewPath { .. }
                | Command::ReorderView { .. }
        );
    if !presentation_only {
        crate::uml::associations::materialize_new(project, &mut section)?;
        crate::uml::associations::synchronize_changes(&previous_section, &mut section)?;
        crate::uml::associations::validate_authoring_changes(&previous_section, &section)?;
    }
    update_composition_placement(&previous_section, &mut section, &project.id, command)?;
    let mut updated = section.to_model(&project.id, &project.name);
    updated.schema_version = project.schema_version;
    crate::composition_paths(&updated)?;
    crate::validate_model_hierarchy(&updated)?;
    // Explorer placement must not be blocked by unrelated semantic or diagram drafts.
    if matches!(command, Command::SetExplorerParent { .. }) {
        return Ok(updated);
    }
    let graph = Graph::new(&updated);
    graph
        .validate_presentation()
        .map_err(|error| error.to_string())?;
    Ok(updated)
}

fn update_composition_placement(
    previous: &ModelingSection,
    current: &mut ModelingSection,
    project_id: &str,
    command: &Command,
) -> Result<(), String> {
    if current.composition.is_empty() {
        return Ok(());
    }
    let (subject, parent) = match command {
        Command::SetOwner {
            element_id,
            owner_id,
        } if current.composition.contains_key(element_id) =>
        {
            let parent = if owner_id.is_some() {
                let ownership = current
                    .relations
                    .iter()
                    .find(|relation| relation.kind == "owns" && relation.to == *element_id)
                    .ok_or("composition placement requires the canonical ownership edge")?;
                crate::CompositionParent::SemanticOwner {
                    relation_id: ownership.id.clone(),
                }
            } else {
                crate::CompositionParent::Object {
                    id: project_id.into(),
                }
            };
            (element_id, parent)
        }
        Command::SetExplorerParent {
            subject_id,
            parent_id,
            ..
        } if current.composition.contains_key(subject_id) => {
            (
                subject_id,
                crate::CompositionParent::Object {
                    id: parent_id.as_deref().unwrap_or(project_id).into(),
                },
            )
        }
        Command::DeleteElement { .. } | Command::DeleteDiagram { .. } => {
            let identities: HashSet<_> = current
                .elements
                .iter()
                .map(|element| element.id.as_str())
                .chain(current.relations.iter().map(|relation| relation.id.as_str()))
                .chain(current.diagrams.iter().map(|diagram| diagram.id.as_str()))
                .collect();
            current.composition.retain(|id, _| identities.contains(id.as_str()));
            for (id, placement) in &current.composition {
                let parent = placement.parent_id(id, &current.relations)
                    .ok_or_else(|| format!("Cannot delete the ownership edge of surviving composition object {id}"))?;
                if parent != project_id && !identities.contains(parent) {
                    return Err(format!("Cannot delete container {parent} while composition object {id} remains; move or delete that object first"));
                }
            }
            return Ok(());
        }
        _ => return Ok(()),
    };
    let mut placement = current
        .composition
        .get(subject)
        .cloned()
        .ok_or("composition placement disappeared")?;
    placement.parent = parent;
    let parent_id = placement
        .parent_id(subject, &current.relations)
        .ok_or("composition ownership reference is unresolved")?;
    if parent_id != project_id && !current.composition.contains_key(parent_id) {
        return Err(format!("Composition destination {parent_id} has no authored placement; no parent or order was inferred"));
    }
    let previous_parent = previous
        .composition
        .get(subject)
        .and_then(|placement| placement.parent_id(subject, &previous.relations));
    if previous_parent != Some(parent_id) && placement.order.is_some() {
        // Keep sibling order and slots intact; only the moved object is appended.
        let last = current
            .composition
            .iter()
            .filter(|(id, sibling)| {
                *id != subject && sibling.slot == placement.slot
                    && sibling.parent_id(id, &current.relations) == Some(parent_id)
            })
            .filter_map(|(_, sibling)| sibling.order)
            .max();
        placement.order = Some(match last {
            Some(order) => order.checked_add(1).ok_or("composition order overflow")?,
            None => 0,
        });
    }
    current.composition.insert(subject.clone(), placement);
    Ok(())
}

fn ensure_visual_edit_supported(project: &Project, command: &Command) -> Result<(), String> {
    if let Command::MoveView { view_id, .. } | Command::ResizeView { view_id, .. } = command {
        let view = project
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.id == *view_id)
            .ok_or("view not found")?;
        let mut pending = vec![view];
        while let Some(view) = pending.pop() {
            if matches!(view.geometry, Geometry::Layout { .. }) {
                return Err("this View contains unresolved layout; its authored presentation was not changed".into());
            }
            pending.extend(view.children.iter());
        }
        return Ok(());
    }
    let diagram = match command {
        Command::MoveView { view_id, .. } | Command::ResizeView { view_id, .. } => project
            .diagrams
            .iter()
            .find(|diagram| diagram.all_views().any(|view| view.id == *view_id)),
        Command::CreateElement { diagram_id, .. }
        | Command::CreateElementOrVisualDraft { diagram_id, .. }
        | Command::CreateRelation { diagram_id, .. }
        | Command::CreateRelationOrVisualDraft { diagram_id, .. }
        | Command::CreateRelationAtViews { diagram_id, .. }
        | Command::CreateRelationAtViewsOrVisualDraft { diagram_id, .. }
        | Command::CreateVisualDraft { diagram_id, .. }
        | Command::PlaceElementView { diagram_id, .. }
        | Command::RemoveView { diagram_id, .. }
        | Command::ReorderView { diagram_id, .. }
        | Command::SetViewEndpoints { diagram_id, .. }
        | Command::SetViewRouting { diagram_id, .. }
        | Command::QuickEdit { diagram_id, .. } => project
            .diagrams
            .iter()
            .find(|diagram| diagram.id == *diagram_id),
        _ => None,
    };
    if let Some(diagram) = diagram {
        if diagram.all_views().any(|view| {
            !view.children.is_empty() || matches!(view.geometry, Geometry::Layout { .. })
        }) {
            return Err(format!(
                "diagram {} requires hierarchy-aware layout before this visual operation; authored presentation was not changed",
                diagram.id
            ));
        }
    }
    Ok(())
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
                || section
                    .hierarchy
                    .values()
                    .filter_map(|placement| placement.occurrence.as_ref())
                    .any(|occurrence| {
                        &occurrence.diagram_id == id
                            || &occurrence.view_id == id
                            || occurrence.member_path.contains(id)
                    })
                || section.hierarchy.contains_key(id)
                || section
                    .hierarchy
                    .values()
                    .any(|placement| placement.parent_id.as_ref() == Some(id))
                || section.elements.iter().any(|element| &element.id == id)
                || section.relations.iter().any(|relation| &relation.id == id)
                || section.diagrams.iter().any(|diagram| {
                    &diagram.id == id || diagram.all_views().any(|view| &view.id == id)
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
    let (diagram_id, owner_id, kind, name, x, y) = match command {
        Command::CreateElement {
            diagram_id,
            owner_id,
            kind,
            name,
            x,
            y,
        } => (Some(diagram_id.as_str()), owner_id, kind, name, *x, *y),
        Command::CreateModelElement {
            owner_id,
            kind,
            name,
        } => (None, owner_id, kind, name, 0.0, 0.0),
        _ => return Err("expected element creation command".into()),
    };
    if !x.is_finite() || !y.is_finite() {
        return Err("element position must be finite".into());
    }
    if ModelingElementKind::DETAILS.contains(kind)
        && !section.diagrams.iter().any(|diagram| {
            Some(diagram.id.as_str()) == diagram_id
                && diagram
                    .views
                    .iter()
                    .any(|view| view.subject.as_ref() == owner_id.as_ref())
        })
    {
        return Err("place the containing Action or CombinedFragment in this diagram before adding its detail".into());
    }
    let requested_name = name.as_deref();
    let mut chosen_name = requested_name.unwrap_or(kind.default_name()).to_owned();
    let basic_classifier = matches!(
        kind,
        ModelingElementKind::Class | ModelingElementKind::Interface
    );
    if kind.requires_owner()
        && owner_id.is_none()
        && !basic_classifier
        && (diagram_id.is_some() || kind.requires_nested_owner())
    {
        return Err(if kind.requires_nested_owner() {
            "Select the containing Activity, StateMachine, State, Region, or Interaction first"
        } else {
            "this element requires an explicit Package or Model owner"
        }
        .into());
    }
    let resolved_owner = if let Some(owner_id) = owner_id {
        Some(owner_id.clone())
    } else if crate::uml::root::object_metaclass(kind.model_kind()).is_some() {
        // Creating one element must not adopt unrelated rootless elements.
        crate::uml::root::Model::from_section(section)
            .root()?
            .map(|root| root.element().id().to_owned())
    } else {
        None
    };
    if let Some(owner_id) = &resolved_owner {
        let owner = section
            .elements
            .iter()
            .find(|element| element.id == *owner_id)
            .ok_or("element owner not found")?;
        // Basic classifiers use the canonical composite-role check below,
        // including nestedClassifier, rather than guessing from the selection.
        if !kind.requires_nested_owner()
            && !basic_classifier
            && !matches!(owner.kind.as_str(), "uml.package" | "uml.model")
        {
            return Err("this creation command requires a Package or Model owner".into());
        }
    }
    if diagram_id.is_some_and(|id| !section.diagrams.iter().any(|diagram| diagram.id == id)) {
        return Err("diagram not found".into());
    }
    if requested_name.is_some_and(|name| name.trim().is_empty()) {
        return Err("element name must not be blank".into());
    }
    if requested_name.is_none() {
        let base = chosen_name.clone();
        for sequence in 1_u64.. {
            let model = crate::uml::root::Model::from_section(section);
            let mut duplicate = false;
            for element in section
                .elements
                .iter()
                .filter(|element| element.name == chosen_name)
            {
                let owner = model.owner(crate::uml::root::ElementRef::Object(element))?;
                if owner.map(|owner| owner.id()) == resolved_owner.as_deref() {
                    duplicate = true;
                    break;
                }
            }
            if !duplicate {
                break;
            }
            chosen_name = format!("{base}{sequence}");
        }
    }
    let [element_id, view_id, ownership_id] = fresh_ids(
        section,
        ["mde-element", "mde-view", "mde-ownership"],
        revision,
        project_id,
    )?;
    if let Some(diagram_id) = diagram_id {
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
            .ok_or("diagram z-index overflow")?;
        diagram.views.push(View {
            children: Vec::new(),
            presentation: Default::default(),
            endpoints: None,
            timing: None,
            draft: None,
            id: view_id,
            subject: Some(element_id.clone()),
            label: None,
            geometry: views::initial_geometry_in_diagram(*kind, diagram, x, y),
            style: Style::default(),
            z_index,
        });
    }
    let mut properties = std::collections::BTreeMap::new();
    if crate::uml::root::object_metaclass(kind.model_kind()).is_some_and(|metaclass| {
        crate::uml::metamodel::conforms_to(metaclass, "PackageableElement")
    }) {
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
    if matches!(kind, ModelingElementKind::Part | ModelingElementKind::Port) {
        properties.insert("aggregation".into(), serde_json::json!("composite"));
    }
    if let Some(pseudostate_kind) = kind.resolve_pseudostate_kind() {
        properties.insert("kind".into(), serde_json::json!(pseudostate_kind));
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
        if kind.requires_nested_owner() || basic_classifier {
            crate::uml::root::set_owner(section, project_id, &element_id, &owner_id)?;
            if kind.requires_nested_owner()
                && let Some(diagram_id) = diagram_id
            {
                features::reposition_boundary_nodes(section, diagram_id, &owner_id);
                features::layout_interaction_operands(section, diagram_id, &owner_id);
            }
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

#[allow(clippy::too_many_arguments)]
fn create_relation(
    section: &mut ModelingSection,
    diagram_id: &str,
    kind: ModelingRelationKind,
    from: &str,
    to: &str,
    owner_id: Option<&str>,
    endpoints: Option<&crate::ViewEndpoints>,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    if kind.is_visual_only() {
        return Err("this connector is View-only notation, not a semantic UML relationship".into());
    }
    for endpoint in [from, to] {
        if !section
            .elements
            .iter()
            .any(|element| element.id == endpoint)
        {
            return Err(format!("relationship endpoint {endpoint} not found"));
        }
    }
    if let Some(owner_id) = owner_id {
        if !section
            .elements
            .iter()
            .any(|element| element.id == owner_id)
            && !section
                .relations
                .iter()
                .any(|relation| relation.id == owner_id)
        {
            return Err(format!("requested relationship owner {owner_id} not found"));
        }
    }
    let endpoints = match endpoints {
        Some(endpoints) => endpoints.clone(),
        None => crate::relationships::resolve_view_endpoints(section, diagram_id, from, to)?,
    };
    let geometry = crate::relationships::route_at_views(section, diagram_id, from, to, &endpoints)?;
    let [id, view_id, ownership_id] = fresh_ids(
        section,
        [
            "mde-uml-relation",
            "mde-uml-relation-view",
            "mde-relation-owner",
        ],
        revision,
        project_id,
    )?;
    let mut properties = std::collections::BTreeMap::new();
    if kind == ModelingRelationKind::PackageImport {
        properties.insert("visibility".into(), serde_json::json!("public"));
    }
    if matches!(
        kind,
        ModelingRelationKind::InterfaceRealization
            | ModelingRelationKind::Deployment
            | ModelingRelationKind::Manifestation
    ) {
        // These composite owners do not project Namespace::ownedMember, so
        // PackageableElement's default public visibility must be cleared.
        properties.insert("visibility".into(), serde_json::Value::Null);
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
    // Source-owned relationship records already encode this owner in `from`.
    // Other authored ownership stays explicit, including incomplete UML drafts.
    let owner_is_encoded_by_source = matches!(
        semantic_kind,
        "uml.generalization"
            | "uml.interface_realization"
            | "uml.template_binding"
            | "uml.element_import"
            | "uml.package_import"
    );
    if let Some(owner_id) = owner_id
        && !(owner_is_encoded_by_source && owner_id == from)
    {
        section.relations.push(Relation {
            id: ownership_id,
            kind: "owns".into(),
            from: owner_id.into(),
            to: id.clone(),
            properties: Default::default(),
        });
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
        children: Vec::new(),
        presentation: Default::default(),
        endpoints: Some(endpoints),
        timing: None,
        draft: None,
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
        .find(|diagram| diagram.all_views().any(|view| view.id == view_id))
        .map(|diagram| diagram.id.clone())
        .ok_or("view not found")?;
    let view = section
        .diagrams
        .iter_mut()
        .find_map(|diagram| diagram.view_mut(view_id))
        .ok_or("view not found")?;
    let subject = view.subject.clone();
    let mut pending = vec![view];
    while let Some(view) = pending.pop() {
        view.geometry.translate(dx, dy);
        pending.extend(view.children.iter_mut());
    }
    drop(pending);
    if let Some(subject) = subject {
        features::translate_contained_views(section, &diagram_id, &subject, dx, dy);
    }
    reroute_relationships(section, &diagram_id);
    if let Some(diagram) = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
    {
        crate::visual_draft_routing::translate_message_anchors(diagram, view_id, dy);
        crate::reroute_visual_drafts(diagram);
    }
    Ok(())
}

fn resize_view(
    section: &mut ModelingSection,
    view_id: &str,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let (is_draft, authored_layout) = section
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == view_id)
        .map(|view| {
            (
                view.draft.is_some(),
                view.presentation.internal
                    || view.children.iter().any(|child| child.presentation.internal)
                    || matches!(view.presentation.notation_kind.as_deref(),
                        Some("presentation.label" | "presentation.compartment" | "presentation.rectangle")),
            )
        })
        .ok_or("view not found")?;
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("view dimensions must be finite and positive".into());
    }
    if !is_draft && !authored_layout && (width < 80.0 || height < 48.0) {
        return Err("view dimensions must be at least 80x48".into());
    }
    let diagram_id = section
        .diagrams
        .iter()
        .find(|diagram| diagram.all_views().any(|view| view.id == view_id))
        .map(|diagram| diagram.id.clone())
        .ok_or("view not found")?;
    let owner_id = section
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == view_id)
        .and_then(|view| view.subject.clone());
    let minimum = owner_id
        .as_deref()
        .and_then(|id| features::class_minimum_bounds(section, &diagram_id, id));
    let notation_minimum = owner_id
        .as_deref()
        .and_then(|id| section.elements.iter().find(|element| element.id == id))
        .map(|element| crate::resolve_element_notation(element).minimum_size())
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
                    crate::stereotype_presentation(section, element).label(),
                )
                .minimum_height(crate::resolve_element_notation(element))
            }
        })
        .unwrap_or(48.0);
    let view = section
        .diagrams
        .iter_mut()
        .find_map(|diagram| diagram.view_mut(view_id))
        .ok_or("view not found")?;
    let (minimum_width, minimum_part_height) = minimum.unwrap_or(notation_minimum);
    let (origin_x, origin_y, scale_x, scale_y) = match &mut view.geometry {
        Geometry::Rect {
            x,
            y,
            width: current_width,
            height: current_height,
        } => {
            if *current_width <= 0.0 || *current_height <= 0.0 {
                return Err("node bounds must be positive before resizing contained Views".into());
            }
            let previous_width = *current_width;
            let previous_height = *current_height;
            *current_width = if authored_layout {
                width.max(1.0)
            } else if is_draft {
                width
            } else {
                width.max(minimum_width)
            };
            *current_height = if authored_layout {
                height.max(1.0)
            } else if is_draft {
                height
            } else {
                height.max(minimum_height).max(minimum_part_height)
            };
            (*x, *y, *current_width / previous_width, *current_height / previous_height)
        }
        _ => return Err("only node Views can be resized with this command".into()),
    };
    let mut pending: Vec<_> = view.children.iter_mut().collect();
    while let Some(view) = pending.pop() {
        scale_child_geometry(&mut view.geometry, origin_x, origin_y, scale_x, scale_y)?;
        pending.extend(view.children.iter_mut());
    }
    drop(pending);
    if let Some(owner_id) = owner_id {
        features::reposition_boundary_nodes(section, &diagram_id, &owner_id);
        features::layout_interaction_operands(section, &diagram_id, &owner_id);
    }
    reroute_relationships(section, &diagram_id);
    if let Some(diagram) = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
    {
        crate::reroute_visual_drafts(diagram);
    }
    Ok(())
}

fn scale_child_geometry(
    geometry: &mut Geometry,
    origin_x: f64,
    origin_y: f64,
    scale_x: f64,
    scale_y: f64,
) -> Result<(), String> {
    let scale_point = |point: &mut crate::Point| {
        point.x = origin_x + (point.x - origin_x) * scale_x;
        point.y = origin_y + (point.y - origin_y) * scale_y;
    };
    match geometry {
        Geometry::Rect { x, y, width, height } => {
            *x = origin_x + (*x - origin_x) * scale_x;
            *y = origin_y + (*y - origin_y) * scale_y;
            *width *= scale_x;
            *height *= scale_y;
        }
        Geometry::Ellipse { cx, cy, rx, ry } => {
            *cx = origin_x + (*cx - origin_x) * scale_x;
            *cy = origin_y + (*cy - origin_y) * scale_y;
            *rx *= scale_x;
            *ry *= scale_y;
        }
        Geometry::Polygon { points } => {
            for point in points {
                scale_point(point);
            }
        }
        Geometry::Path { start, segments } => {
            scale_point(start);
            for segment in segments {
                match segment {
                    crate::PathSegment::Line { to } => scale_point(to),
                    crate::PathSegment::Cubic { control1, control2, to } => {
                        scale_point(control1);
                        scale_point(control2);
                        scale_point(to);
                    }
                }
            }
        }
        Geometry::Layout { .. } => return Err("contained View has unresolved layout".into()),
    }
    Ok(())
}

fn remove_element(section: &mut ModelingSection, id: &str) -> Result<(), String> {
    let semantic = crate::uml::root::Model::from_section(section);
    if semantic.element(id).is_none() {
        if section.diagrams.iter().any(|diagram| {
            RelationshipGraph::new(section, &diagram.id)
                .visible_relations()
                .iter()
                .any(|relation| relation.id == id)
        }) {
            ensure_view_deletion_supported(section, &HashSet::from([id.to_owned()]))?;
            section.relations.retain(|relation| relation.id != id);
            for diagram in &mut section.diagrams {
                diagram
                    .views
                    .retain(|view| view.subject.as_deref() != Some(id));
            }
            return Ok(());
        }
        return Err("element not found or structural link cannot be deleted directly".into());
    }
    let mut deleted = HashSet::from([id.to_owned()]);
    loop {
        let mut changed = false;
        for element in semantic.elements() {
            if semantic
                .owner(element)?
                .is_some_and(|owner| deleted.contains(owner.id()))
                && deleted.insert(element.id().to_owned())
            {
                changed = true;
            }
        }
        for relation in &section.relations {
            if (deleted.contains(&relation.from) || deleted.contains(&relation.to))
                && deleted.insert(relation.id.clone())
            {
                changed = true;
            }
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
    let affected_subjects = deleted.union(&incident).cloned().collect();
    ensure_view_deletion_supported(section, &affected_subjects)?;
    section
        .elements
        .retain(|element| !deleted.contains(&element.id));
    section
        .relations
        .retain(|relation| !incident.contains(&relation.id) && !deleted.contains(&relation.id));
    section.hierarchy.retain(|subject, placement| {
        if placement
            .parent_id
            .as_ref()
            .is_some_and(|id| deleted.contains(id) || incident.contains(id))
        {
            placement.parent_id = None;
            placement.occurrence = None;
        }
        !deleted.contains(subject) && !incident.contains(subject)
    });
    if section
        .root_model
        .as_ref()
        .is_some_and(|root| deleted.contains(root))
    {
        section.root_model = None;
    }
    for diagram in &mut section.diagrams {
        diagram.views.retain(|view| {
            view.subject
                .as_ref()
                .is_none_or(|subject| !deleted.contains(subject) && !incident.contains(subject))
        });
    }
    Ok(())
}

fn ensure_view_deletion_supported(
    section: &ModelingSection,
    affected_subjects: &HashSet<String>,
) -> Result<(), String> {
    for diagram in &section.diagrams {
        let mut pending: Vec<_> = diagram.views.iter().map(|view| (view, false)).collect();
        while let Some((view, nested)) = pending.pop() {
            if view
                .subject
                .as_ref()
                .is_some_and(|subject| affected_subjects.contains(subject))
                && (nested
                    || !view.children.is_empty()
                    || matches!(view.geometry, Geometry::Layout { .. }))
            {
                return Err(format!(
                    "deleting this element affects View {} in diagram {}, which requires hierarchy-aware deletion; no elements or authored presentation were changed",
                    view.id, diagram.id
                ));
            }
            pending.extend(view.children.iter().map(|child| (child, true)));
        }
    }
    Ok(())
}
