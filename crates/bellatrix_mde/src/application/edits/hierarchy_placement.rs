use crate::ModelingSection;
use std::collections::HashSet;

pub(crate) fn set_diagram_parent(
    section: &mut ModelingSection, subject: &str, parent: Option<&str>, occurrence: Option<&crate::ViewOccurrence>,
) -> Result<(), String> {
    if !section.diagrams.iter().any(|diagram| diagram.id == subject) {
        return Err("Semantic elements must be moved through domain ownership commands".into());
    }
    if let Some(parent) = parent {
        let exists = section.elements.iter().any(|element| element.id == parent)
            || section.relations.iter().any(|relation| relation.id == parent)
            || section.diagrams.iter().any(|diagram| diagram.id == parent);
        if !exists { return Err("Hierarchy parent not found".into()); }
    }
    if let Some(occurrence) = occurrence {
        let parent = parent.ok_or("View occurrence requires a semantic parent")?;
        validate_diagram_occurrence(section, parent, occurrence)?;
    }
    section.hierarchy.insert(subject.to_owned(), crate::HierarchyPlacement {
        parent_id: parent.map(str::to_owned),
        occurrence: occurrence.cloned(),
    });
    Ok(())
}


pub fn validate_diagram_occurrence(
    section: &ModelingSection,
    parent_id: &str,
    occurrence: &crate::ViewOccurrence,
) -> Result<(), String> {
    let crate::ViewOccurrence { diagram_id, view_id, member_path } = occurrence;
    let diagram = section.diagrams.iter().find(|diagram| diagram.id == *diagram_id)
        .ok_or("View occurrence diagram not found")?;
    let anchor = diagram.views.iter().find(|view| view.id == *view_id)
        .and_then(|view| view.subject.as_deref()).ok_or("View occurrence subject not found")?;
    let model = crate::uml::root::Model::from_section(section);
    if model.element(anchor).is_none() { return Err("View occurrence subject no longer exists".into()); }
    let mut owner_id = anchor;
    let mut seen = HashSet::from([anchor]);
    for member_id in member_path {
        if !seen.insert(member_id.as_str()) { return Err("View occurrence member path contains a cycle".into()); }
        let member = model.element(member_id).ok_or("View occurrence member no longer exists")?;
        if model.owner(member)?.is_none_or(|owner| owner.id() != owner_id) {
            return Err("View occurrence member ownership changed".into());
        }
        if diagram.views.iter().any(|view| view.subject.as_deref() == Some(member_id.as_str())) {
            return Err("View occurrence member now has its own occurrence".into());
        }
        owner_id = member_id;
    }
    if owner_id != parent_id { return Err("View occurrence does not resolve to the requested parent".into()); }
    Ok(())
}

pub(crate) fn validate_occurrences(section: &ModelingSection) -> Result<(), String> {
    for (diagram_id, placement) in &section.hierarchy {
        if let Some(occurrence) = &placement.occurrence {
            let parent = placement.parent_id.as_deref().ok_or("View occurrence requires a parent")?;
            if occurrence.diagram_id == *diagram_id {
                return Err("Diagram cannot belong to its own view occurrence".into());
            }
            validate_diagram_occurrence(section, parent, occurrence)?;
        }
    }
    Ok(())
}

pub fn validate_model_hierarchy(project: &crate::Project) -> Result<(), String> {
    project.validate_hierarchy()?;
    validate_occurrences(&ModelingSection::from_model(project))
}
