//! Read-only semantic hierarchy projection shared with presentation adapters.
use crate::ModelingSection;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchyRow {
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
}
fn semantic(kind: &str) -> bool {
    !kind.starts_with("fs.") && !kind.starts_with("cargo.")
}
fn rank(kind: &str) -> u8 {
    match kind {
        "uml.model" | "uml.package" | "rust.module" => 0,
        "rust.struct" | "rust.trait" | "rust.enum" | "uml.class" | "uml.interface" => 1,
        "rust.field" | "rust.function" | "uml.attribute" | "uml.operation" => 2,
        _ => 3,
    }
}
pub fn project_hierarchy(section: &ModelingSection) -> Vec<HierarchyRow> {
    use crate::uml::root::{ElementRef, Model};
    let model = Model::from_section(section);
    let references = crate::uml::references::ReferenceIndex::new(model);
    let elements: HashMap<_, _> = model
        .elements()
        .filter(|element| semantic(element.kind()))
        .map(|element| (element.id(), element))
        .collect();
    let mut children: HashMap<&str, Vec<ElementRef<'_>>> = HashMap::new();
    let mut roots = Vec::new();
    for element in elements.values().copied() {
        if let Some(parent) = model
            .owner(element)
            .ok()
            .flatten()
            .filter(|owner| elements.contains_key(owner.id()))
        {
            children.entry(parent.id()).or_default().push(element);
        } else {
            roots.push(element);
        }
    }
    let sort = |items: &mut Vec<ElementRef<'_>>| {
        items.sort_by(|left, right| {
            rank(left.kind())
                .cmp(&rank(right.kind()))
                .then_with(|| {
                    left.display_label()
                        .to_lowercase()
                        .cmp(&right.display_label().to_lowercase())
                })
                .then_with(|| left.id().cmp(right.id()))
        });
    };
    sort(&mut roots);
    for (owner_id, siblings) in &mut children {
        sort(siblings);
        if elements
            .get(owner_id)
            .is_some_and(|owner| owner.conforms_to("Operation"))
            && let Ok(parameters) = crate::uml::operations::owned_parameters(&references, owner_id)
        {
            siblings.sort_by_key(|element| {
                parameters
                    .iter()
                    .position(|parameter| parameter.id == element.id())
                    .unwrap_or(usize::MAX)
            });
        }
    }
    fn visit<'a>(
        element: ElementRef<'a>,
        depth: usize,
        children: &HashMap<&str, Vec<ElementRef<'a>>>,
        seen: &mut HashSet<String>,
        result: &mut Vec<HierarchyRow>,
        references: &crate::uml::references::ReferenceIndex<'_>,
    ) {
        if !seen.insert(element.id().to_owned()) {
            return;
        }
        result.push(HierarchyRow {
            element_id: element.id().into(),
            label: match element {
                ElementRef::Object(object) => {
                    crate::element_display_label_with_references(object, references)
                }
                ElementRef::Relationship(_) => element.display_label(),
            },
            kind: element.kind().into(),
            depth,
        });
        if let Some(children_of_element) = children.get(element.id()) {
            for child in children_of_element {
                visit(*child, depth + 1, children, seen, result, references);
            }
        }
    }
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        visit(root, 0, &children, &mut seen, &mut result, &references);
    }
    let mut remaining = elements
        .values()
        .copied()
        .filter(|element| !seen.contains(element.id()))
        .collect::<Vec<_>>();
    sort(&mut remaining);
    for element in remaining {
        visit(element, 0, &children, &mut seen, &mut result, &references);
    }
    for row in &mut result {
        if let Some(ElementRef::Relationship(relation)) = elements.get(row.element_id.as_str()) {
            row.label = crate::visual_consistency::relationship_display_label(
                section,
                relation,
                &references,
            );
        }
        if let Some(ElementRef::Object(element)) = elements.get(row.element_id.as_str())
            && crate::templates::is_parameter(&element.kind)
        {
            row.label =
                crate::template_parameter_label_with_references(section, element, &references);
        }
    }
    result
}
