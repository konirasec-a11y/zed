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
    let anchor = diagram.all_views().find(|view| view.id == *view_id)
        .and_then(|view| view.subject.as_deref()).ok_or("View occurrence subject not found")?;
    let subjects: HashSet<&str> = section.elements.iter().map(|element| element.id.as_str())
        .chain(section.relations.iter().map(|relation| relation.id.as_str())).collect();
    if !subjects.contains(anchor) { return Err("View occurrence subject no longer exists".into()); }
    let mut resolved_subject = anchor;
    let mut seen = HashSet::from([anchor]);
    for member_id in member_path {
        if !seen.insert(member_id.as_str()) { return Err("View occurrence member path contains a cycle".into()); }
        if !subjects.contains(member_id.as_str()) {
            return Err("View occurrence member no longer exists".into());
        }
        resolved_subject = member_id;
    }
    if resolved_subject != parent_id { return Err("View occurrence does not resolve to the requested parent".into()); }
    Ok(())
}

pub fn validate_model_hierarchy(project: &crate::Project) -> Result<(), String> {
    // Import and authoring preserve domain-invalid paths for explicit kernel diagnostics.
    project.validate_hierarchy()
}
