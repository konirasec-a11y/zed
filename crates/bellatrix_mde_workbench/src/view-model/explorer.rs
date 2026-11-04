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

/// The Explorer is an authoring tree, not a projection of canvas occurrences.
pub(crate) fn project_explorer(section: &ModelingSection, sort: ExplorerSortOrder) -> Vec<ExplorerRow> {
    project_authored_composition(section, sort)
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

/// Document objects remain document objects even when pictured by multiple views.
/// View navigation continues through the canvas; it is not semantic containment.
fn project_authored_composition(section: &ModelingSection, sort: ExplorerSortOrder) -> Vec<ExplorerRow> {
    use std::collections::{HashMap, HashSet};
    let semantic = bellatrix_mde::project_hierarchy(section);
    let labels = semantic.iter().map(|row| (row.element_id.clone(), row.label.clone())).collect::<HashMap<_, _>>();
    let semantic_order = semantic.iter().enumerate().map(|(index, row)| (row.element_id.as_str(), index)).collect::<HashMap<_, _>>();
    let mut native_parents = HashMap::new();
    let mut ancestors = Vec::new();
    for row in &semantic {
        ancestors.truncate(row.depth);
        native_parents.insert(row.element_id.clone(), ancestors.last().cloned());
        ancestors.push(row.element_id.clone());
    }
    let mut identities = semantic.iter().map(|row| row.element_id.clone())
        .chain(section.diagrams.iter().map(|diagram| diagram.id.clone())).collect::<Vec<_>>();
    identities.sort(); identities.dedup();
    let source_order = section.elements.iter().map(|element| element.id.as_str())
        .chain(section.relations.iter().map(|relation| relation.id.as_str()))
        .chain(section.diagrams.iter().map(|diagram| diagram.id.as_str()))
        .enumerate().map(|(order, id)| (id, order)).collect::<HashMap<_, _>>();
    let mut nodes = HashMap::new();
    for id in &identities {
        let row = if let Some(diagram) = section.diagrams.iter().find(|diagram| diagram.id == *id) {
            ExplorerRow { key: diagram_key(id), element_id: id.clone(), label: diagram.name.clone(),
                kind: diagram.uml_di_type.clone(), depth: 0, diagram: true, view: None, diagram_context: None }
        } else if let Some(element) = section.elements.iter().find(|element| element.id == *id) {
            ExplorerRow { key: semantic_key(id), element_id: id.clone(),
                label: labels.get(id).cloned().unwrap_or_else(|| element.name.clone()), kind: element.kind.clone(),
                depth: 0, diagram: false, view: None, diagram_context: None }
        } else if let Some(relation) = section.relations.iter().find(|relation| relation.id == *id) {
            ExplorerRow { key: semantic_key(id), element_id: id.clone(), label: relationship_label(section, relation),
                kind: bellatrix_mde::relationship_presentation(relation).notation,
                depth: 0, diagram: false, view: None, diagram_context: None }
        } else { continue; };
        nodes.insert(id.clone(), row);
    }
    let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for id in nodes.keys() {
        let parent = if let Some(placement) = section.composition.get(id) {
            placement.parent_id(id, &section.relations).map(str::to_owned)
        } else if nodes[id].diagram {
            section.hierarchy.get(id).and_then(|placement| placement.parent_id.clone())
        } else {
            native_parents.get(id).cloned().flatten()
        }.filter(|parent| nodes.contains_key(parent));
        children.entry(parent).or_default().push(id.clone());
    }
    for siblings in children.values_mut() {
        siblings.sort_by(|left, right| {
            let left_composed = section.composition.contains_key(left);
            let right_composed = section.composition.contains_key(right);
            if left_composed != right_composed { return right_composed.cmp(&left_composed); }
            if !section.composition.contains_key(left) && !section.composition.contains_key(right) {
                let left_parameter = bellatrix_mde::semantic_identity(&nodes[left].kind).type_label == "Parameter";
                let right_parameter = bellatrix_mde::semantic_identity(&nodes[right].kind).type_label == "Parameter";
                if left_parameter != right_parameter { return right_parameter.cmp(&left_parameter); }
                if left_parameter { return semantic_order.get(left.as_str()).cmp(&semantic_order.get(right.as_str())).then_with(|| left.cmp(right)); }
            }
            let ordering = match sort {
                ExplorerSortOrder::Name => nodes[left].label.to_lowercase().cmp(&nodes[right].label.to_lowercase()),
                ExplorerSortOrder::Added => {
                    match (section.composition.get(left), section.composition.get(right)) {
                        (Some(left), Some(right)) => left.slot.cmp(&right.slot).then_with(|| left.order.cmp(&right.order)),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => source_order.get(left.as_str()).cmp(&source_order.get(right.as_str())),
                    }
                }
            };
            ordering.then_with(|| left.cmp(right))
        });
    }
    fn visit(id: &str, depth: usize, nodes: &HashMap<String, ExplorerRow>,
        children: &HashMap<Option<String>, Vec<String>>, seen: &mut HashSet<String>, rows: &mut Vec<ExplorerRow>) {
        if !seen.insert(id.to_owned()) { return; }
        let Some(row) = nodes.get(id) else { return; };
        let mut row = row.clone(); row.depth = depth; rows.push(row);
        if let Some(children_ids) = children.get(&Some(id.to_owned())) {
            for child in children_ids { visit(child, depth + 1, nodes, children, seen, rows); }
        }
    }
    let mut rows = Vec::new(); let mut seen = HashSet::new();
    if let Some(roots) = children.get(&None) {
        for root in roots { visit(root, 0, &nodes, &children, &mut seen, &mut rows); }
    }
    // Structural validation diagnoses invalid graphs; projection must still terminate.
    for id in &identities { visit(id, 0, &nodes, &children, &mut seen, &mut rows); }
    rows
}
