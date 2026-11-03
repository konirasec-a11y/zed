//! Explorer projection over semantic ownership and explicit UMLDI contexts.
use bellatrix_mde::ModelingSection;

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
    let mut nodes = semantic.into_iter().map(|row| ExplorerRow {
        element_id: row.element_id, label: row.label, kind: row.kind, depth: 0, diagram: false,
    }).collect::<Vec<_>>();
    let mut diagrams = section.diagrams.iter().collect::<Vec<_>>();
    diagrams.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.id.cmp(&b.id)));
    nodes.extend(diagrams.into_iter().map(|diagram| ExplorerRow {
        element_id: diagram.id.clone(), label: diagram.name.clone(),
        kind: {
            let context = if diagram.model_elements.len() == 1 {
                section.elements.iter().find(|element| element.id == diagram.model_elements[0])
                    .map(|element| format!(" · context: {}", element.name))
                    .unwrap_or_else(|| " · unresolved context".into())
            } else if diagram.model_elements.len() > 1 { " · multiple contexts".into() }
            else { String::new() };
            let organized = if section.explorer_parents.contains_key(&diagram.id) { " · organized" } else { "" };
            format!("{}{context}{organized}", diagram.uml_di_type)
        }, depth: 0, diagram: true,
    }));
    let parents = bellatrix_mde::explorer_parent_map(section);
    let ids = nodes.iter().map(|row| row.element_id.clone()).collect::<std::collections::HashSet<_>>();
    let mut children: std::collections::HashMap<Option<String>, Vec<usize>> = std::collections::HashMap::new();
    for (index, row) in nodes.iter_mut().enumerate() {
        let parent = parents.get(&row.element_id).cloned().flatten();
        // Retain unresolved stored references, but never hide their subjects.
        if parent.as_ref().is_some_and(|id| !ids.contains(id)) {
            row.label.push_str(" · missing organization parent");
        }
        let visible_parent = parent.filter(|id| ids.contains(id));
        children.entry(visible_parent).or_default().push(index);
    }
    fn visit(index: usize, depth: usize, nodes: &[ExplorerRow], children: &std::collections::HashMap<Option<String>, Vec<usize>>, seen: &mut std::collections::HashSet<usize>, result: &mut Vec<ExplorerRow>) {
        if !seen.insert(index) { return; }
        let mut row = nodes[index].clone();
        row.depth = depth;
        result.push(row);
        if let Some(children) = children.get(&Some(nodes[index].element_id.clone())) {
            for child in children { visit(*child, depth + 1, nodes, children, seen, result); }
        }
    }
    let mut rows = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Some(roots) = children.get(&None) {
        for root in roots { visit(*root, 0, &nodes, &children, &mut seen, &mut rows); }
    }
    // Malformed legacy cycles remain visible instead of silently dropping items.
    for index in 0..nodes.len() { visit(index, 0, &nodes, &children, &mut seen, &mut rows); }
    rows
}

/// A drag carries identity and revision, not a detached copy of the model.
#[derive(Clone)]
pub struct ExplorerDrag {
    pub project_id: bellatrix_project::ProjectId,
    pub element_id: String,
    pub label: String,
    pub revision: u64,
}
impl gpui::Render for ExplorerDrag {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::prelude::*;
        let palette = crate::WorkbenchPalette::current(cx);
        gpui::div().px_2().py_1().bg(palette.surface).text_color(palette.text).child(self.label.clone())
    }
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

pub fn submit_explorer_placement(
    owner: &gpui::Entity<crate::MdeWorkbenchItem>,
    drag: &ExplorerDrag,
    parent: Option<&str>,
    cx: &mut gpui::App,
) {
    owner.update(cx, |workbench, cx| {
        if workbench.source().is_read_only() || &drag.project_id != workbench.project_id()
            || parent == Some(drag.element_id.as_str()) { return; }
        workbench.submit(bellatrix_mde_cordis::MdeUiCommand::SetExplorerParent(
            bellatrix_mde_cordis::ModelingSetExplorerParentRequested {
                project_id: drag.project_id.clone(), subject_id: drag.element_id.clone(),
                parent_id: parent.map(str::to_owned), expected_revision: drag.revision,
            }), cx);
    });
}
