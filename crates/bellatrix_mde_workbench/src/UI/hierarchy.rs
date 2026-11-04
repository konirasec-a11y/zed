//! Explorer projection over semantic ownership and explicit UMLDI contexts.
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

#[derive(Clone)]
pub(crate) struct ExplorerElementDrag {
    pub project_id: bellatrix_project::ProjectId,
    pub element_id: String,
    pub target_diagram_id: Option<String>,
    pub source_view_id: Option<String>,
    pub revision: u64,
    pub label: String,
    pub ownership_checks: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, Result<(), String>>>>,
}

impl gpui::Render for ExplorerElementDrag {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::prelude::*;
        let palette = crate::workbench_ui::WorkbenchPalette::current(cx);
        gpui::div().px_2().py_1().bg(palette.surface).text_color(palette.text)
            .child(self.label.clone())
    }
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
            false
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

/// Diagram occurrences replace the standalone semantic row, without changing ownership.
pub(crate) fn project_explorer(section: &ModelingSection, sort: ExplorerSortOrder) -> Vec<ExplorerRow> {
    use std::collections::{HashMap, HashSet};
    let semantic = bellatrix_mde::project_hierarchy(section);
    let semantic_order = semantic.iter().enumerate()
        .map(|(index, row)| (row.element_id.clone(), index)).collect::<HashMap<_, _>>();
    let mut owners = HashMap::new();
    let mut ancestors = Vec::new();
    for row in &semantic {
        ancestors.truncate(row.depth);
        owners.insert(row.element_id.clone(), ancestors.last().cloned());
        ancestors.push(row.element_id.clone());
    }
    // The semantic root container is not an extra project node in the Explorer.
    let root_containers = semantic.iter()
        .filter(|row| row.kind == "uml.model" && owners.get(&row.element_id).is_none_or(|owner| owner.is_none()))
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
            // A pictured subject has no global duplicate, but its members still
            // belong under owner occurrences in diagrams where they have no view.
            if occurrences.get(&row.element_id).is_some_and(|views| {
                diagram_context.as_ref().is_none_or(|diagram| views.iter().any(|(candidate, _)| candidate == diagram))
            }) { continue; }
            let key = if diagram_context.is_some() {
                format!("member:{}:{parent}:{}", parent.len(), row.element_id)
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
        // UMLDI context is a reference, not ownership. Only an unambiguous
        // external occurrence can locate this diagram without a self-cycle.
        let context_parent = if let [context] = diagram.model_elements.as_slice() {
            let candidates = placements.get(context).into_iter().flatten()
                .filter(|(owner_diagram, _)| owner_diagram.as_ref() != Some(&diagram.id))
                .map(|(_, key)| key).collect::<Vec<_>>();
            match candidates.as_slice() {
                [parent] => Some((*parent).clone()),
                _ => None,
            }
        } else { None };
        parents.insert(key.clone(), context_parent);
        nodes.push(ExplorerRow {
            key,
            diagram_context: Some(diagram.id.clone()),
            element_id: diagram.id.clone(),
            label: diagram.name.clone(),
            kind: diagram.uml_di_type.clone(),
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
            .and_then(|views| views.iter().find(|(diagram, _)| diagram.as_ref() == Some(&view.diagram_id)))
            .map(|(_, key)| key.clone());
        parents.insert(key.clone(), Some(visual_parent.or(semantic_parent)
            .unwrap_or_else(|| diagram_key(&view.diagram_id))));
        nodes.push(ExplorerRow {
            key,
            diagram_context: Some(view.diagram_id.clone()),
            element_id: view.view_id.clone(),
            label: view.subject_id.as_deref().and_then(|id| relation_labels.get(id))
                .cloned().unwrap_or_else(|| view.label.clone()),
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

pub(crate) fn explorer_view_icon(id: &str, kind: &str, selected: bool, palette: &crate::workbench_ui::WorkbenchPalette) -> impl gpui::IntoElement {
    use gpui::{div, prelude::*, px, svg};
    use bellatrix_ui_symbols::glyph;
    div().id(format!("mde-explorer-view-icon-{id}"))
        .flex().items_center().gap_1()
        .tooltip(ui::Tooltip::text("Diagram occurrence"))
        .child(ui::Icon::new(ui::IconName::Eye).size(ui::IconSize::Small)
            .color(if selected { ui::Color::Selected } else { ui::Color::Muted }))
        .child(svg().data(glyph(element_glyph(kind))).size(px(14.0)).flex_none()
            .text_color(if selected { palette.selection } else { palette.muted }))
}

pub(crate) fn element_glyph(kind: &str) -> bellatrix_ui_symbols::ShapeKind {
    use bellatrix_ui_symbols::ShapeKind;
    let identity = bellatrix_mde::semantic_identity(kind);
    // Member and relationship symbols are metaclass based, never inferred from names.
    match kind {
        "uml.composition" => return ShapeKind::Composition,
        "uml.aggregation" => return ShapeKind::Aggregate,
        _ => {}
    }
    match identity.type_label {
        "Attribute" | "Property" => return ShapeKind::Aggregate,
        "Operation" => return ShapeKind::Process,
        "Reception" => return ShapeKind::AcceptEvent,
        "Parameter" => return ShapeKind::Input,
        "Constraint" => return ShapeKind::Note,
        "Generalization" => return ShapeKind::Inheritance,
        "Frame" => return ShapeKind::Boundary,
        _ => {}
    }
    let label = if kind == "rust.module" {
        "Package"
    } else {
        identity.type_label
    };
    bellatrix_ui_symbols::groups(bellatrix_ui_symbols::Family::Uml)
        .iter()
        .flat_map(|group| group.items)
        .find(|symbol| symbol.label.eq_ignore_ascii_case(label))
        .map(|symbol| symbol.kind)
        .unwrap_or(ShapeKind::Topic)
}
