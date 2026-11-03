use crate::ModelingSection;
use std::collections::{BTreeMap, HashSet};

pub fn explorer_parent_map(section: &ModelingSection) -> BTreeMap<String, Option<String>> {
    let model = crate::uml::root::Model::from_section(section);
    let elements: BTreeMap<_, _> = model.elements()
        .filter(|element| !element.kind().starts_with("fs.") && !element.kind().starts_with("cargo."))
        .map(|element| (element.id(), element)).collect();
    let mut result: BTreeMap<String, Option<String>> = elements.iter().map(|(id, element)| {
        let parent = model.owner(*element).ok().flatten()
            .filter(|owner| elements.contains_key(owner.id()))
            .map(|owner| owner.id().to_owned());
        ((*id).to_owned(), parent)
    }).collect();
    for diagram in &section.diagrams {
        let parent = if diagram.model_elements.len() == 1 && result.contains_key(&diagram.model_elements[0]) {
            Some(diagram.model_elements[0].clone())
        } else { None };
        result.insert(diagram.id.clone(), parent);
    }
    for (id, parent) in &section.explorer_parents {
        if result.contains_key(id) { result.insert(id.clone(), parent.clone()); }
    }
    result
}

pub(crate) fn set_explorer_parent(section: &mut ModelingSection, subject: &str, parent: Option<&str>) -> Result<(), String> {
    let mut parents = explorer_parent_map(section);
    if !parents.contains_key(subject) { return Err("Explorer item not found".into()); }
    if let Some(parent) = parent && !parents.contains_key(parent) { return Err("Explorer parent not found".into()); }
    parents.insert(subject.to_owned(), parent.map(str::to_owned));
    let mut cursor = Some(subject);
    let mut seen = HashSet::new();
    while let Some(id) = cursor {
        if !seen.insert(id.to_owned()) { return Err("Explorer placement would create a cycle".into()); }
        cursor = parents.get(id).and_then(|parent| parent.as_deref());
    }
    section.explorer_parents.insert(subject.to_owned(), parent.map(str::to_owned));
    Ok(())
}

