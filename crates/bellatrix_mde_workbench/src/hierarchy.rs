//! Explorer projection over semantic ownership and explicit UMLDI contexts.
use bellatrix_mde::{HierarchyRow, ModelingSection};

#[derive(Clone)]
pub struct ExplorerRow {
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
    pub diagram: bool,
}

/// Diagram context is a reference, not ownership: display it under its single
/// resolved context, and leave context-free or ambiguous diagrams at the root.
pub fn project_explorer(section: &ModelingSection) -> Vec<ExplorerRow> {
    let semantic = bellatrix_mde::project_hierarchy(section);
    let mut diagrams = section.diagrams.iter().collect::<Vec<_>>();
    diagrams.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.id.cmp(&b.id)));
    let diagram_row = |diagram: &bellatrix_mde::Diagram, depth, contextual| ExplorerRow {
        element_id: diagram.id.clone(), label: diagram.name.clone(),
        kind: if contextual { format!("{} · context", diagram.uml_di_type) } else { diagram.uml_di_type.clone() }, depth, diagram: true,
    };
    let mut rows = Vec::new();
    for HierarchyRow { element_id, label, kind, depth } in &semantic {
        rows.push(ExplorerRow { element_id: element_id.clone(), label: label.clone(), kind: kind.clone(), depth: *depth, diagram: false });
        for diagram in &diagrams {
            if diagram.model_elements.len() == 1 && diagram.model_elements[0] == *element_id {
                rows.push(diagram_row(diagram, depth + 1, true));
            }
        }
    }
    for diagram in diagrams {
        if diagram.model_elements.len() != 1 || !semantic.iter().any(|row| row.element_id == diagram.model_elements[0]) {
            rows.push(diagram_row(diagram, 0, false));
        }
    }
    rows
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
