//! Delete a semantic element from the MDE graph through ProjectService.
//! Owns-children are deleted with their owner; unrelated filesystem records
//! and other elements remain. All incident edges and diagram views are removed.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::ModelingSection;
use bellatrix_mde_modeling::{Graph, RelationshipGraph};
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct ModelingDeleteElementRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub expected_revision: u64,
}

pub struct ModelingDeleteElement;
impl Event for ModelingDeleteElement {
    const NAME: &'static str = "bellatrix.mde.delete-element.v1";
    type Args = ModelingDeleteElementRequested;
    type Output = Result<u64, String>;
}

/// The ownership closure is based only on semantic `owns` edges.
fn remove_element(section: &mut ModelingSection, id: &str) -> Result<(), String> {
    // Only diagram-visible relationships are directly deletable. "owns",
    // "declares" and "physical_parent" remain structural graph edges.
    // The graph query is shared with the Canvas and Model Explorer, so legacy
    // dependencies and typed UML connections have identical behavior.
    if section.relations.iter().any(|r| r.id == id) {
        if !section.diagrams.iter().any(|diagram|
            RelationshipGraph::new(section, &diagram.id)
                .visible_relations().iter().any(|relation| relation.id == id)
        ) {
            return Err("structural relationship cannot be deleted directly".into());
        }
        section.relations.retain(|r| r.id != id);
        for diagram in &mut section.diagrams {
            diagram.views.retain(|view| view.subject.as_deref() != Some(id));
        }
        return Ok(());
    }
    if !section.elements.iter().any(|element| element.id == id) {
        return Err("element not found".into());
    }
    let mut deleted = HashSet::from([id.to_owned()]);
    loop {
        let mut changed = false;
        for relation in &section.relations {
            if relation.kind == "owns" && deleted.contains(&relation.from)
                && deleted.insert(relation.to.clone())
            {
                changed = true;
            }
        }
        if !changed { break; }
    }
    let incident_relations: HashSet<String> = section.relations.iter()
        .filter(|relation| deleted.contains(&relation.from) || deleted.contains(&relation.to))
        .map(|relation| relation.id.clone())
        .collect();
    section.elements.retain(|element| !deleted.contains(&element.id));
    section.relations.retain(|relation| !incident_relations.contains(&relation.id));
    for diagram in &mut section.diagrams {
        diagram.views.retain(|view| view.subject.as_ref().is_none_or(
            |subject| !deleted.contains(subject) && !incident_relations.contains(subject)
        ));
    }
    Ok(())
}

pub(crate) async fn delete_element(
    ctx: &Context,
    projects: &ProjectService,
    request: ModelingDeleteElementRequested,
) -> Result<u64, String> {
    let document = projects.snapshot(&request.project_id).map_err(|e| e.to_string())?;
    if document.revision() != request.expected_revision {
        return Err("stale element deletion; refresh project snapshot".into());
    }
    let mut section = from_document(&document)?;
    let namespace = namespace_for(&document);
    remove_element(&mut section, &request.element_id)?;
    Graph::new(&section.to_model(document.id().as_str(), document.name()))
        .validate().map_err(|error| error.to_string())?;
    let state = serde_json::to_value(section).map_err(|e| e.to_string())?;
    match ctx.query::<ProjectReplace>(Routing::Unscoped, ProjectReplaceRequested {
        id: request.project_id,
        namespace: namespace.into(),
        expected_revision: request.expected_revision,
        state,
    }).await.map_err(|e| e.to_string())? {
        QueryOutcome::Answer(Ok(updated)) => Ok(updated.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_class_removes_owned_members_views_and_incident_edges_only() {
        let mut section = bellatrix_mde_model::ModelingSection::from_model(
            &serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap()
        );
        remove_element(&mut section, "type-task").unwrap();
        for deleted in ["type-task", "field-title", "field-completed"] {
            assert!(!section.elements.iter().any(|e| e.id == deleted));
        }
        assert!(section.elements.iter().any(|e| e.id == "type-list"));
        assert!(section.elements.iter().any(|e| e.id == "file-task"));
        assert!(section.relations.iter().all(|r| r.from != "type-task" && r.to != "type-task"));
        assert!(section.diagrams.iter().all(|d| d.views.iter().all(|v|
            !matches!(v.subject.as_deref(), Some("type-task" | "ref-task")))));
        Graph::new(&section.to_model("project-todo", "Task List / Todo Project")).validate().unwrap();
    }

    #[test]
    fn refusing_missing_element_does_not_modify_model() {
        let mut section = bellatrix_mde_model::ModelingSection::from_model(
            &serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap()
        );
        let before = section.clone();
        assert!(remove_element(&mut section, "not-present").is_err());
        assert_eq!(section, before);
    }
}
