//! Read-only semantic hierarchy projection shared with presentation adapters.
use crate::{Element, ModelingSection};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchyRow {
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
}
fn semantic(element: &Element) -> bool { !element.kind.starts_with("fs.") && !element.kind.starts_with("cargo.") }
fn rank(element: &Element) -> u8 {
    match element.kind.as_str() {
        "uml.model" | "uml.package" | "rust.module" => 0,
        "rust.struct" | "rust.trait" | "rust.enum" | "uml.class" | "uml.interface" => 1,
        "rust.field" | "rust.function" | "uml.attribute" | "uml.operation" => 2,
        _ => 3,
    }
}
pub fn project_hierarchy(section: &ModelingSection) -> Vec<HierarchyRow> {
    let elements: HashMap<&str, &Element> = section.elements.iter().filter(|element| semantic(element))
        .map(|element| (element.id.as_str(), element)).collect();
    let parents: HashMap<&str, &str> = section.relations.iter().filter(|relation| relation.kind == "owns")
        .filter(|relation| elements.contains_key(relation.from.as_str()) && elements.contains_key(relation.to.as_str()))
        .map(|relation| (relation.to.as_str(), relation.from.as_str())).collect();
    let mut children: HashMap<&str, Vec<&Element>> = HashMap::new();
    let mut roots: Vec<&Element> = Vec::new();
    for element in elements.values() {
        if let Some(parent) = parents.get(element.id.as_str()) { children.entry(parent).or_default().push(element); }
        else { roots.push(element); }
    }
    let sort = |items: &mut Vec<&Element>| {
        items.sort_by(|left, right| rank(left).cmp(&rank(right))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())).then_with(|| left.id.cmp(&right.id)));
    };
    sort(&mut roots);
    for siblings in children.values_mut() { sort(siblings); }
    fn visit(element: &Element, depth: usize, children: &HashMap<&str, Vec<&Element>>,
        seen: &mut HashSet<String>, result: &mut Vec<HierarchyRow>) {
        if !seen.insert(element.id.clone()) { return; }
        result.push(HierarchyRow { element_id: element.id.clone(), label: element.name.clone(), kind: element.kind.clone(), depth });
        if let Some(child_list) = children.get(element.id.as_str()) {
            for child in child_list { visit(child, depth + 1, children, seen, result); }
        }
    }
    let mut result = Vec::with_capacity(elements.len());
    let mut seen = HashSet::new();
    for root in roots { visit(root, 0, &children, &mut seen, &mut result); }
    // Keep malformed/incomplete ownership data visible for diagnosis.
    let mut remaining = elements.values().copied().filter(|element| !seen.contains(&element.id)).collect::<Vec<_>>();
    sort(&mut remaining);
    for element in remaining { visit(element, 0, &children, &mut seen, &mut result); }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn models_only_semantic_nodes_with_owned_members() {
        let model = serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap();
        let section = ModelingSection::from_model(&model);
        let tree = project_hierarchy(&section);
        let row = |id: &str| tree.iter().find(|row| row.element_id == id).unwrap();
        assert_eq!(row("type-task").depth, 0);
        assert_eq!(row("field-title").depth, row("type-task").depth + 1);
        assert_eq!(row("fn-complete").depth, row("type-list").depth + 1);
        assert!(tree.iter().position(|row| row.element_id == "type-task").unwrap() < tree.iter().position(|row| row.element_id == "field-title").unwrap());
        assert!(tree.iter().all(|row| !row.kind.starts_with("fs.") && !row.kind.starts_with("cargo.")));
        assert!(!tree.iter().any(|row| row.element_id == "file-task" || row.element_id == "dir-src"));
        assert_eq!(tree.len(), 7);
    }
    #[test]
    fn empty_semantic_model_does_not_expose_files_or_cargo_metadata() {
        let mut model: crate::Project = serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap();
        model.elements.retain(|element| !semantic(element));
        assert!(project_hierarchy(&ModelingSection::from_model(&model)).is_empty());
    }
}
