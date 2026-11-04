//! Explorer projection over domain ownership and shared diagram hierarchy.
use bellatrix_mde::ModelingSection;

#[derive(Clone)]
pub struct ExplorerRow {
    pub key: String,
    pub diagram_context: Option<String>,
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
    pub diagram: bool,
    pub view: Option<bellatrix_mde::VisualComponent>,
}

pub(crate) fn member_creation_kinds(
    section: &ModelingSection, subject: &str,
) -> Vec<(bellatrix_mde::commands::ModelingMemberKind, &'static str)> {
    use bellatrix_mde::{UmlCapability, UmlType, commands::ModelingMemberKind};
    let Some(owner_type) = section.elements.iter().find(|element| element.id == subject)
        .and_then(|element| UmlType::from_kind(&element.kind)) else { return Vec::new(); };
    [
        (ModelingMemberKind::Attribute, UmlCapability::Attribute, "Attribute"),
        (ModelingMemberKind::Operation, UmlCapability::Operation, "Operation"),
        (ModelingMemberKind::Reception, UmlCapability::Reception, "Reception"),
        (ModelingMemberKind::Parameter, UmlCapability::Parameter, "Parameter"),
    ].into_iter().filter(|(_, capability, _)| owner_type.supports(*capability))
        .map(|(kind, _, label)| (kind, label)).collect()
}

pub(crate) fn contextual_diagram_kinds(
    section: &ModelingSection, subject: &str,
) -> Vec<bellatrix_mde::StructuralDiagramKind> {
    let Some(element) = section.elements.iter().find(|element| element.id == subject) else {
        return Vec::new();
    };
    let metaclass = bellatrix_mde::uml::root::object_metaclass(&element.kind);
    bellatrix_mde::StructuralDiagramKind::ALL_DIAGRAMS.into_iter().filter(|kind| {
        if let Some(expected) = kind.context_metaclass() {
            metaclass.is_some_and(|actual| bellatrix_mde::uml::metamodel::conforms_to(actual, expected))
        } else if *kind == bellatrix_mde::StructuralDiagramKind::CompositeStructure {
            // Instance classifier evidence is not exposed by this menu projection yet.
            // Do not infer it from raw properties or offer an unverified context.
            bellatrix_mde::UmlType::from_kind(&element.kind)
                .is_some_and(bellatrix_mde::UmlType::is_structured_classifier)
        } else {
            matches!(element.kind.as_str(), "uml.package" | "uml.model")
        }
    }).collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExplorerSortOrder {
    Added,
    Name,
}

pub(crate) fn filter_explorer(rows: Vec<ExplorerRow>, query: &str) -> Vec<ExplorerRow> {
    let query = query.trim().to_lowercase();
    if query.is_empty() { return rows; }
    let words = query.split_whitespace().collect::<Vec<_>>();
    let mut retained = std::collections::HashSet::new();
    let mut ancestors = Vec::new();
    let mut matching_depth = None;
    for (index, row) in rows.iter().enumerate() {
        ancestors.truncate(row.depth);
        if matching_depth.is_some_and(|depth| row.depth <= depth) { matching_depth = None; }
        let text = format!("{} {} {} {}", row.label, row.kind, row.element_id,
            row.view.as_ref().and_then(|view| view.subject_id.as_deref()).unwrap_or("")).to_lowercase();
        if matching_depth.is_some() || words.iter().all(|word| text.contains(*word)) {
            retained.extend(ancestors.iter().copied());
            retained.insert(index);
            if matching_depth.is_none() { matching_depth = Some(row.depth); }
        }
        ancestors.push(index);
    }
    rows.into_iter().enumerate().filter_map(|(index, row)| retained.contains(&index).then_some(row)).collect()
}

fn semantic_key(id: &str) -> String {
    format!("semantic:{id}")
}

fn diagram_key(id: &str) -> String {
    format!("diagram:{id}")
}

fn view_key(diagram: &str, view: &str) -> String {
    format!("view:{}:{diagram}:{view}", diagram.len())
}

fn member_key(parent: &str, member: &str) -> String {
    format!("member:{}:{parent}:{member}", parent.len())
}

fn relationship_label(section: &ModelingSection, relation: &bellatrix_mde::Relation) -> String {
    use bellatrix_mde::RelationshipMarker;
    let endpoint = |id: &str| {
        section.elements.iter().find(|element| element.id == id)
            .map(|element| element.name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or(id)
            .to_owned()
    };
    let presentation = bellatrix_mde::relationship_presentation(relation);
    let relation_type = bellatrix_mde::semantic_identity(&presentation.notation).type_label;
    let name = presentation.name.as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("{relation_type}: {name}"))
        .unwrap_or_else(|| relation_type.to_owned());
    let marker = |marker, source| match marker {
        RelationshipMarker::None => "",
        RelationshipMarker::OpenArrow => if source { "<" } else { ">" },
        RelationshipMarker::Triangle => if source { "◁" } else { "▷" },
        RelationshipMarker::FilledTriangle => if source { "◀" } else { "▶" },
        RelationshipMarker::Diamond => "◇",
        RelationshipMarker::FilledDiamond => "◆",
        RelationshipMarker::FilledCircle => "●",
        RelationshipMarker::OpenArrowAndCircle => if source { "○<" } else { ">○" },
        RelationshipMarker::FilledTriangleAndCircle => if source { "○◀" } else { "▶○" },
        RelationshipMarker::Containment => "⊕",
    };
    let line = match presentation.style.line_mode {
        bellatrix_mde::LineMode::Solid => "─",
        bellatrix_mde::LineMode::Dashed => "┄",
        bellatrix_mde::LineMode::Dotted => "··",
    };
    format!("{} {}{line} {} {}{line}{} {}",
        endpoint(&relation.from), marker(presentation.style.source_marker, true), name,
        marker(presentation.style.middle_marker, false),
        marker(presentation.style.target_marker, false), endpoint(&relation.to))
}

/// Occurrences retain their diagram identity; explicit semantic ownership remains visible.
pub(crate) fn project_explorer(section: &ModelingSection, sort: ExplorerSortOrder) -> Vec<ExplorerRow> {
    use std::collections::{HashMap, HashSet};
    let semantic = bellatrix_mde::project_hierarchy(section);
    let semantic_labels = semantic.iter().map(|row| (row.element_id.clone(), row.label.clone()))
        .collect::<HashMap<_, _>>();
    let semantic_order = semantic.iter().enumerate()
        .map(|(index, row)| (row.element_id.clone(), index)).collect::<HashMap<_, _>>();
    let mut owners = HashMap::new();
    let mut ancestors = Vec::new();
    for row in &semantic {
        ancestors.truncate(row.depth);
        owners.insert(row.element_id.clone(), ancestors.last().cloned());
        ancestors.push(row.element_id.clone());
    }
    // An authored semantic diagram parent needs its full semantic path, even
    // when an ancestor is otherwise represented only by multiple diagram views.
    let mut hierarchy_ancestors = HashSet::new();
    for placement in section.hierarchy.values().filter(|placement| placement.occurrence.is_none()) {
        let mut ancestor = placement.parent_id.as_ref();
        while let Some(id) = ancestor {
            if !hierarchy_ancestors.insert(id.clone()) { break; }
            ancestor = owners.get(id).and_then(|owner| owner.as_ref());
        }
    }
    // The semantic root container is not an extra project node in the Explorer.
    let root_containers = semantic.iter()
        .filter(|row| row.kind == "uml.model" && section.root_model.as_deref() == Some(row.element_id.as_str()))
        .map(|row| row.element_id.clone()).collect::<HashSet<_>>();
    let mut views = bellatrix_mde::visual_components(section);
    views.sort_by(|left, right| left.diagram_id.cmp(&right.diagram_id)
        .then_with(|| left.view_id.cmp(&right.view_id)));
    let mut occurrences: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for view in &views {
        if let Some(subject) = &view.subject_id {
            occurrences.entry(subject.clone()).or_default()
                .push((view.diagram_id.clone(), view_key(&view.diagram_id, &view.view_id)));
        }
    }
    let relation_labels = section.relations.iter()
        .map(|relation| (relation.id.as_str(), relationship_label(section, relation)))
        .collect::<HashMap<_, _>>();
    let relation_kinds = section.relations.iter()
        .map(|relation| (relation.id.as_str(), bellatrix_mde::relationship_presentation(relation).notation))
        .collect::<HashMap<_, _>>();
    let mut nodes = Vec::new();
    let mut parents: HashMap<String, Option<String>> = HashMap::new();
    let mut placements: HashMap<String, Vec<(Option<String>, String)>> = occurrences.iter()
        .map(|(subject, rows)| (subject.clone(), rows.iter()
            .map(|(diagram, key)| (Some(diagram.clone()), key.clone())).collect()))
        .collect();
    for row in semantic {
        if root_containers.contains(&row.element_id) { continue; }
        let owner = owners.get(&row.element_id).and_then(|owner| owner.as_ref())
            .filter(|owner| !root_containers.contains(*owner));
        let contexts = owner.and_then(|owner| placements.get(owner)).cloned()
            .unwrap_or_else(|| vec![(None, owner.map(|owner| semantic_key(owner)).unwrap_or_default())]);
        let mut row_placements = placements.get(&row.element_id).cloned().unwrap_or_default();
        for (diagram_context, parent) in contexts {
            // Root-level pictured subjects need no extra row. Under an explicit
            // owner, retain the semantic row so reparenting is visible even when
            // the existing views remain in their original diagrams.
            if occurrences.get(&row.element_id).is_some_and(|views| {
                match diagram_context.as_ref() {
                    Some(diagram) => views.iter().any(|(candidate, _)| candidate == diagram),
                    None => owner.is_none() && !hierarchy_ancestors.contains(&row.element_id),
                }
            }) { continue; }
            let key = if diagram_context.is_some() {
                member_key(&parent, &row.element_id)
            } else { semantic_key(&row.element_id) };
            parents.insert(key.clone(), (!parent.is_empty()).then_some(parent));
            row_placements.push((diagram_context.clone(), key.clone()));
            nodes.push(ExplorerRow {
                key,
                diagram_context,
                label: relation_labels.get(row.element_id.as_str()).cloned().unwrap_or_else(|| row.label.clone()),
                element_id: row.element_id.clone(),
                kind: relation_kinds.get(row.element_id.as_str()).cloned().unwrap_or_else(|| row.kind.clone()),
                depth: 0,
                diagram: false,
                view: None,
            });
        }
        placements.insert(row.element_id, row_placements);
    }
    let mut diagrams = section.diagrams.iter().collect::<Vec<_>>();
    diagrams.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase())
        .then_with(|| left.id.cmp(&right.id)));
    for diagram in diagrams {
        let key = diagram_key(&diagram.id);
        let placement = section.hierarchy.get(&diagram.id);
        let parent_subject = placement.and_then(|placement| placement.parent_id.as_deref());
        let explicit_occurrence = placement.and_then(|placement| placement.occurrence.as_ref())
            .and_then(|occurrence| {
                let subject = parent_subject?;
                if occurrence.diagram_id == diagram.id
                    || bellatrix_mde::validate_diagram_occurrence(section, subject, occurrence).is_err() {
                    return None;
                }
                let key = occurrence.member_path.iter().fold(
                    view_key(&occurrence.diagram_id, &occurrence.view_id),
                    |parent, member| member_key(&parent, member),
                );
                placements.get(subject).into_iter().flatten()
                    .any(|(context, candidate)| context.as_deref() == Some(occurrence.diagram_id.as_str()) && *candidate == key)
                    .then_some(key)
            });
        let context_parent = if explicit_occurrence.is_some() {
            explicit_occurrence
        } else if let Some(parent) = parent_subject.filter(|parent| section.diagrams.iter().any(|candidate| candidate.id == *parent)) {
            Some(diagram_key(parent))
        } else if let Some(context) = parent_subject {
            let candidates = placements.get(context).into_iter().flatten()
                .filter(|(owner_diagram, _)| owner_diagram.as_ref() != Some(&diagram.id))
                .collect::<Vec<_>>();
            if let Some((_, parent)) = candidates.iter().find(|(owner_diagram, _)| owner_diagram.is_none()) {
                Some(parent.clone())
            } else {
                match candidates.as_slice() {
                    [(_, parent)] => Some(parent.clone()),
                    _ => None,
                }
            }
        } else { None };
        parents.insert(key.clone(), context_parent);
        nodes.push(ExplorerRow {
            key,
            diagram_context: Some(diagram.id.clone()),
            element_id: diagram.id.clone(),
            label: diagram.name.clone(),
            kind: match bellatrix_mde::StructuralDiagramKind::from_diagram(diagram) {
                Some(bellatrix_mde::StructuralDiagramKind::Sequence) => "UMLSequenceDiagram".into(),
                Some(bellatrix_mde::StructuralDiagramKind::Communication) => "UMLCommunicationDiagram".into(),
                Some(bellatrix_mde::StructuralDiagramKind::InteractionOverview) => "UMLInteractionOverviewDiagram".into(),
                Some(bellatrix_mde::StructuralDiagramKind::Timing) => "UMLTimingDiagram".into(),
                _ => diagram.uml_di_type.clone(),
            },
            depth: 0,
            diagram: true,
            view: None,
        });
    }
    for view in views {
        let key = view_key(&view.diagram_id, &view.view_id);
        let visual_parent = view.parent_view_id.as_ref().filter(|parent| {
            section.diagrams.iter().find(|diagram| diagram.id == view.diagram_id)
                .is_some_and(|diagram| diagram.views.iter().any(|candidate| &candidate.id == *parent))
        }).map(|parent| view_key(&view.diagram_id, parent));
        let semantic_parent = view.subject_id.as_ref()
            .and_then(|subject| owners.get(subject)).and_then(|owner| owner.as_ref())
            .and_then(|owner| placements.get(owner))
            .and_then(|placements| {
                let mut candidates = placements.iter().filter(|(diagram, _)| diagram.as_ref() == Some(&view.diagram_id));
                let (_, key) = candidates.next()?;
                candidates.next().is_none().then(|| key.clone())
            });
        parents.insert(key.clone(), Some(visual_parent.or(semantic_parent)
            .unwrap_or_else(|| diagram_key(&view.diagram_id))));
        nodes.push(ExplorerRow {
            key,
            diagram_context: Some(view.diagram_id.clone()),
            element_id: view.view_id.clone(),
            label: view.subject_id.as_deref().and_then(|id| relation_labels.get(id))
                .cloned().or_else(|| view.subject_id.as_deref().and_then(|id| semantic_labels.get(id)).cloned())
                .unwrap_or_else(|| view.label.clone()),
            kind: view.subject_id.as_deref().and_then(|id| relation_kinds.get(id))
                .cloned().unwrap_or_else(|| view.kind.clone()),
            depth: 0,
            diagram: false,
            view: Some(view),
        });
    }
    let ids = nodes.iter().map(|row| row.key.clone()).collect::<HashSet<_>>();
    let mut children: HashMap<Option<String>, Vec<usize>> = HashMap::new();
    for (index, row) in nodes.iter().enumerate() {
        let mut parent = parents.get(&row.key).cloned().flatten();
        let mut cursor = parent.as_deref();
        let mut seen = HashSet::from([row.key.as_str()]);
        while let Some(id) = cursor {
            if !seen.insert(id) {
                parent = row.view.as_ref().map(|view| diagram_key(&view.diagram_id));
                break;
            }
            cursor = parents.get(id).and_then(|parent| parent.as_deref());
        }
        children.entry(parent.filter(|id| ids.contains(id))).or_default().push(index);
    }
    let element_order = section.elements.iter().enumerate()
        .map(|(index, element)| (element.id.as_str(), index))
        .chain(section.relations.iter().enumerate()
            .map(|(index, relation)| (relation.id.as_str(), section.elements.len() + index)))
        .collect::<HashMap<_, _>>();
    let diagram_order = section.diagrams.iter().enumerate()
        .map(|(index, diagram)| (diagram.id.as_str(), index)).collect::<HashMap<_, _>>();
    let view_order = section.diagrams.iter().flat_map(|diagram| diagram.views.iter().enumerate()
        .map(move |(index, view)| ((diagram.id.as_str(), view.id.as_str()), index)))
        .collect::<HashMap<_, _>>();
    let sort_keys = nodes.iter().map(|row| {
        let added = if let Some(view) = &row.view {
            (2, view_order.get(&(view.diagram_id.as_str(), view.view_id.as_str())).copied().unwrap_or(usize::MAX))
        } else if row.diagram {
            (1, diagram_order.get(row.element_id.as_str()).copied().unwrap_or(usize::MAX))
        } else {
            (0, element_order.get(row.element_id.as_str()).copied().unwrap_or(usize::MAX))
        };
        let subject = row.view.as_ref().and_then(|view| view.subject_id.as_deref()).unwrap_or(&row.element_id);
        (
            bellatrix_mde::semantic_identity(&row.kind).type_label == "Parameter",
            semantic_order.get(subject).copied().unwrap_or(usize::MAX),
            added,
            row.label.to_lowercase(),
        )
    }).collect::<Vec<_>>();
    for siblings in children.values_mut() {
        siblings.sort_by(|left, right| {
            let left_key = &sort_keys[*left];
            let right_key = &sort_keys[*right];
            // Parameter order is part of an operation signature, not cosmetic sorting.
            let ordering = if left_key.0 != right_key.0 {
                right_key.0.cmp(&left_key.0)
            } else if left_key.0 {
                left_key.1.cmp(&right_key.1)
            } else {
                match sort {
                    ExplorerSortOrder::Added => left_key.2.cmp(&right_key.2),
                    ExplorerSortOrder::Name => left_key.3.cmp(&right_key.3),
                }
            };
            ordering.then_with(|| nodes[*left].key.cmp(&nodes[*right].key))
        });
    }
    fn visit(index: usize, depth: usize, nodes: &[ExplorerRow],
        children: &HashMap<Option<String>, Vec<usize>>, seen: &mut HashSet<usize>,
        result: &mut Vec<ExplorerRow>) {
        if !seen.insert(index) { return; }
        let mut row = nodes[index].clone();
        row.depth = depth;
        result.push(row);
        if let Some(child_indices) = children.get(&Some(nodes[index].key.clone())) {
            for child in child_indices { visit(*child, depth + 1, nodes, children, seen, result); }
        }
    }
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    if let Some(roots) = children.get(&None) {
        for root in roots { visit(*root, 0, &nodes, &children, &mut seen, &mut rows); }
    }
    for index in 0..nodes.len() { visit(index, 0, &nodes, &children, &mut seen, &mut rows); }
    rows
}

pub(crate) fn explorer_type_label(kind: &str, relationship: bool) -> String {
    if relationship { return "Relationship".into(); }
    let label = bellatrix_mde::semantic_identity(kind).type_label;
    if label == kind { "Element".into() } else { label.into() }
}


pub(crate) fn model_creation_kinds(section: &ModelingSection, owner: Option<&str>) -> Vec<bellatrix_mde::commands::ModelingElementKind> {
    use bellatrix_mde::commands::ModelingElementKind::*;
    if owner.is_some_and(|id| !section.elements.iter().any(|element|
        element.id == id && matches!(element.kind.as_str(), "uml.model" | "uml.package"))) {
        return Vec::new();
    }
    vec![Model, Subsystem, Package, Profile, Class, Interface, Signal, DataType,
        PrimitiveType, Enumeration, Artifact, Component, Node, UseCase, Actor,
        Activity, StateMachine, Interaction]
}

pub(crate) fn project_diagram_kinds() -> Vec<bellatrix_mde::StructuralDiagramKind> {
    bellatrix_mde::StructuralDiagramKind::ALL_DIAGRAMS.into_iter()
        .filter(|kind| kind.context_metaclass().is_none()
            && *kind != bellatrix_mde::StructuralDiagramKind::CompositeStructure)
        .collect()
}

pub(crate) fn parent_occurrence(
    section: &ModelingSection, subject: Option<&str>, row_key: &str,
) -> Option<bellatrix_mde::ViewOccurrence> {
    let rows = project_explorer(section, ExplorerSortOrder::Added);
    parent_occurrence_from_rows(section, subject, row_key, &rows)
}

pub(crate) fn parent_occurrence_from_rows(
    section: &ModelingSection, subject: Option<&str>, row_key: &str, rows: &[ExplorerRow],
) -> Option<bellatrix_mde::ViewOccurrence> {
    let subject = subject?;
    let index = rows.iter().position(|row| row.key == row_key)?;
    let row = rows.get(index)?;
    if row.diagram || row.view.as_ref().and_then(|view| view.subject_id.as_deref())
        .unwrap_or(row.element_id.as_str()) != subject { return None; }
    let mut member_path = Vec::new();
    let mut depth = row.depth;
    for candidate in std::iter::once(row).chain(rows[..index].iter().rev().filter(|candidate| candidate.depth < row.depth)) {
        if candidate.key != row.key && candidate.depth >= depth { continue; }
        depth = candidate.depth;
        if candidate.diagram { return None; }
        if let Some(view) = &candidate.view {
            member_path.reverse();
            let occurrence = bellatrix_mde::ViewOccurrence {
                diagram_id: view.diagram_id.clone(), view_id: view.view_id.clone(), member_path,
            };
            return bellatrix_mde::validate_diagram_occurrence(section, subject, &occurrence)
                .is_ok().then_some(occurrence);
        }
        member_path.push(candidate.element_id.clone());
    }
    None
}
