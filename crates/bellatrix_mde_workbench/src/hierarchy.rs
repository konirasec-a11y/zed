//! Model Explorer projection from MDE containment edges, not source order.
//! The flat list in the project format does not determine tree hierarchy.
use bellatrix_mde_model::{Element, ModelingSection};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchyRow {
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
}

fn rank(element: &Element) -> u8 {
    match element.kind.as_str() {
        "cargo.workspace" | "cargo.package" => 0,
        "fs.directory" => 1,
        "fs.file" => 2,
        "rust.struct" | "rust.trait" | "rust.enum" => 3,
        "rust.field" | "rust.function" => 4,
        _ => 5,
    }
}

/// Render a validated graph as a stable tree. Priority is semantic ownership,
/// then declaration, then physical filesystem containment. An element appears
/// at most once, including when referenced by multiple relationships.
pub fn project_hierarchy(section: &ModelingSection) -> Vec<HierarchyRow> {
    let elements: HashMap<&str, &Element> = section.elements.iter()
        .map(|element| (element.id.as_str(), element)).collect();
    let mut parents: HashMap<&str, (&str, u8)> = HashMap::new();
    for relation in &section.relations {
        let priority = match relation.kind.as_str() {
            "owns" => 0,
            "declares" => 1,
            "physical_parent" => 2,
            _ => continue,
        };
        if !elements.contains_key(relation.from.as_str()) || !elements.contains_key(relation.to.as_str()) {
            continue;
        }
        let to = relation.to.as_str();
        if parents.get(to).is_none_or(|(_, p)| priority < *p) {
            parents.insert(to, (relation.from.as_str(), priority));
        }
    }

    let mut children: HashMap<&str, Vec<&Element>> = HashMap::new();
    let mut roots: Vec<&Element> = Vec::new();
    for element in &section.elements {
        if let Some((parent, _)) = parents.get(element.id.as_str()) {
            children.entry(parent).or_default().push(element);
        } else {
            roots.push(element);
        }
    }
    let sort_children = |items: &mut Vec<&Element>| {
        items.sort_by(|a, b| rank(a).cmp(&rank(b))
            .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase())));
    };
    sort_children(&mut roots);
    for siblings in children.values_mut() { sort_children(siblings); }

    fn visit(
        element: &Element, depth: usize, children: &HashMap<&str, Vec<&Element>>,
        seen: &mut HashSet<String>, result: &mut Vec<HierarchyRow>,
    ) {
        if !seen.insert(element.id.clone()) { return; }
        result.push(HierarchyRow {
            element_id: element.id.clone(),
            label: element.name.clone(),
            kind: element.kind.clone(),
            depth,
        });
        if let Some(child_list) = children.get(element.id.as_str()) {
            for child in child_list { visit(child, depth + 1, children, seen, result); }
        }
    }
    let mut result = Vec::with_capacity(section.elements.len());
    let mut seen = HashSet::new();
    for root in roots { visit(root, 0, &children, &mut seen, &mut result); }
    // Defensively show invalid/orphaned nodes rather than hiding model data.
    for element in &section.elements {
        if !seen.contains(&element.id) { visit(element, 0, &children, &mut seen, &mut result); }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shows_physical_files_declarations_and_owned_members_in_a_tree() {
        let model = serde_json::from_str(
            include_str!("../../bellatrix_mde_model/fixtures/todo.json")
        ).unwrap();
        let section = ModelingSection::from_model(&model);
        let tree = project_hierarchy(&section);
        let row = |id: &str| tree.iter().find(|r| r.element_id == id).unwrap();
        assert_eq!(row("dir-src").depth, 0);
        assert_eq!(row("file-task").depth, row("dir-src").depth + 1);
        assert_eq!(row("type-task").depth, row("file-task").depth + 1);
        assert_eq!(row("field-title").depth, row("type-task").depth + 1);
        assert!(tree.iter().position(|r| r.element_id == "dir-src").unwrap()
            < tree.iter().position(|r| r.element_id == "file-task").unwrap());
        assert_eq!(tree.len(), section.elements.len());
    }
}
