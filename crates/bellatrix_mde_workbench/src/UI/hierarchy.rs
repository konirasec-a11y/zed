//! Explorer projection over semantic ownership and explicit UMLDI contexts.
use bellatrix_mde::ModelingSection;

#[derive(Clone)]
pub struct ExplorerRow {
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
    pub diagram: bool,
    pub view: Option<bellatrix_mde::VisualComponent>,
}

/// Diagram context is a reference, not ownership: display it under its single
/// resolved context, and leave context-free or ambiguous diagrams at the root.
pub fn project_explorer(section: &ModelingSection) -> Vec<ExplorerRow> {
    let semantic = bellatrix_mde::project_hierarchy(section);
    // Ownership comes from the semantic projection, never legacy presentation placements.
    let mut parents = std::collections::HashMap::new();
    let mut ancestors = Vec::new();
    for row in &semantic {
        ancestors.truncate(row.depth);
        parents.insert(row.element_id.clone(), ancestors.last().cloned());
        ancestors.push(row.element_id.clone());
    }
    let semantic_ids = parents
        .keys()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let mut nodes = semantic
        .into_iter()
        .map(|row| ExplorerRow {
            element_id: row.element_id,
            label: row.label,
            kind: row.kind,
            depth: 0,
            diagram: false,
            view: None,
        })
        .collect::<Vec<_>>();
    let mut diagrams = section.diagrams.iter().collect::<Vec<_>>();
    diagrams.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    for diagram in &diagrams {
        let parent = if diagram.model_elements.len() == 1 {
            diagram
                .model_elements
                .first()
                .filter(|id| semantic_ids.contains(*id))
                .cloned()
        } else {
            None
        };
        parents.insert(diagram.id.clone(), parent);
    }
    nodes.extend(diagrams.into_iter().map(|diagram| ExplorerRow {
        element_id: diagram.id.clone(),
        label: diagram.name.clone(),
        kind: {
            let context = if diagram.model_elements.len() == 1 {
                section
                    .elements
                    .iter()
                    .find(|element| element.id == diagram.model_elements[0])
                    .map(|element| format!(" · context: {}", element.name))
                    .unwrap_or_else(|| " · unresolved context".into())
            } else if diagram.model_elements.len() > 1 {
                " · multiple contexts".into()
            } else {
                String::new()
            };
            format!("{}{context}", diagram.uml_di_type)
        },
        depth: 0,
        diagram: true,
        view: None,
    }));
    nodes.extend(
        bellatrix_mde::visual_components(section)
            .into_iter()
            // Bound views are occurrences of an existing semantic row, not extra model nodes.
            .filter(|view| {
                !view.subject_id.as_ref().is_some_and(|id| semantic_ids.contains(id))
            })
            .map(|view| ExplorerRow {
                element_id: view.view_id.clone(),
                label: view.label.clone(),
                kind: view.kind.clone(),
                depth: 0,
                diagram: false,
                view: Some(view),
            }),
    );
    let ids = nodes
        .iter()
        .map(|row| row.element_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut children: std::collections::HashMap<Option<String>, Vec<usize>> =
        std::collections::HashMap::new();
    for (index, row) in nodes.iter().enumerate() {
        let parent = if let Some(view) = &row.view {
            let valid_parent = section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == view.diagram_id)
                .and_then(|diagram| {
                    let mut seen = std::collections::HashSet::new();
                    seen.insert(view.view_id.as_str());
                    let mut cursor = view.parent_view_id.as_deref();
                    let mut visible_parent = None;
                    while let Some(id) = cursor {
                        if !seen.insert(id) {
                            return None;
                        }
                        let parent = diagram.views.iter().find(|candidate| candidate.id == id)?;
                        if visible_parent.is_none() && ids.contains(id) {
                            visible_parent = Some(id.to_owned());
                        }
                        cursor = parent
                            .draft
                            .as_ref()
                            .and_then(|draft| draft.parent_view_id.as_deref());
                    }
                    visible_parent
                });
            Some(valid_parent.unwrap_or_else(|| view.diagram_id.clone()))
        } else {
            parents.get(&row.element_id).cloned().flatten()
        };
        let visible_parent = parent.filter(|id| ids.contains(id));
        children.entry(visible_parent).or_default().push(index);
    }
    fn visit(
        index: usize,
        depth: usize,
        nodes: &[ExplorerRow],
        children: &std::collections::HashMap<Option<String>, Vec<usize>>,
        seen: &mut std::collections::HashSet<usize>,
        result: &mut Vec<ExplorerRow>,
    ) {
        if !seen.insert(index) {
            return;
        }
        let mut row = nodes[index].clone();
        row.depth = depth;
        result.push(row);
        if let Some(child_indices) = children.get(&Some(nodes[index].element_id.clone())) {
            for child in child_indices {
                visit(*child, depth + 1, nodes, children, seen, result);
            }
        }
    }
    let mut rows = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Some(roots) = children.get(&None) {
        for root in roots {
            visit(*root, 0, &nodes, &children, &mut seen, &mut rows);
        }
    }
    // Malformed legacy cycles remain visible instead of silently dropping items.
    for index in 0..nodes.len() {
        visit(index, 0, &nodes, &children, &mut seen, &mut rows);
    }
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
