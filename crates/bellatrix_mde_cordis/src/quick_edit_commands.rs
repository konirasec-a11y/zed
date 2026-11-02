//! Shared UML edits; the selected semantic type chooses the permitted specialization.
use crate::{add_member, create_relation, project_integration::{from_document, namespace_for}};
use bellatrix_mde_model::{Element, Geometry, ModelingSection, Relation, Style, View};
use bellatrix_mde_modeling::{Graph, UmlQuickCommand, UmlType};
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::{Context, Event, Routing};
use cordis::event::QueryOutcome;

#[derive(Clone, Debug)]
pub struct ModelingQuickEditRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub element_id: String,
    pub action: UmlQuickCommand,
    pub expected_revision: u64,
}

pub struct ModelingQuickEdit;
impl Event for ModelingQuickEdit {
    const NAME: &'static str = "bellatrix.mde.quick-edit.v1";
    type Args = ModelingQuickEditRequested;
    type Output = Result<u64, String>;
}

fn fresh_id(section: &ModelingSection, purpose: &str, revision: u64) -> Result<String, String> {
    let mut sequence = revision.checked_add(1).ok_or("identifier overflow")?;
    loop {
        let id = format!("mde-quick-{purpose}-{sequence}");
        let occupied = section.elements.iter().any(|element| element.id == id)
            || section.relations.iter().any(|relation| relation.id == id)
            || section.diagrams.iter().any(|diagram| diagram.id == id
                || diagram.views.iter().any(|view| view.id == id));
        if !occupied { return Ok(id); }
        sequence = sequence.checked_add(1).ok_or("identifier overflow")?;
    }
}

fn unique_name(section: &ModelingSection, prefix: &str) -> Result<String, String> {
    let mut sequence = 1_u64;
    loop {
        let name = format!("{prefix}{sequence}");
        if !section.elements.iter().any(|element| element.name == name) { return Ok(name); }
        sequence = sequence.checked_add(1).ok_or("name sequence overflow")?;
    }
}

pub(crate) fn apply_quick_edit(section: &mut ModelingSection, request: &ModelingQuickEditRequested) -> Result<(), String> {
    // Do not publish even an in-memory partial edit if a later route or ID check fails.
    let mut candidate = section.clone();
    apply_to_candidate(&mut candidate, request)?;
    *section = candidate;
    Ok(())
}

fn apply_to_candidate(section: &mut ModelingSection, request: &ModelingQuickEditRequested) -> Result<(), String> {
    let element = section.elements.iter().find(|element| element.id == request.element_id)
        .ok_or("Quick Edit subject not found")?;
    let kind = element.kind.clone();
    let subject_type = UmlType::from_kind(&kind).ok_or("Quick Edit subject is not a supported UML type")?;
    if !request.action.accepts(&kind) {
        return Err(format!("{kind} does not support {:?}", request.action));
    }
    if request.action == UmlQuickCommand::AddReception {
        add_member::append_member(section, &request.element_id,
            add_member::ModelingMemberKind::Reception, request.expected_revision)?;
        return Ok(());
    }
    let diagram = section.diagrams.iter().find(|diagram| diagram.id == request.diagram_id)
        .ok_or("diagram not found")?;
    let view = diagram.views.iter().find(|view| view.subject.as_deref() == Some(request.element_id.as_str()))
        .ok_or("Quick Edit subject has no View in the active diagram")?;
    let (left, top, width, height) = match &view.geometry {
        Geometry::Rect { x, y, width, height } => (*x, *y, *width, *height),
        _ => return Err("Quick Edit requires a node View".into()),
    };
    let subtype_kind = kind.as_str();
    let (child_kind, prefix, offset, child_size, relation_kind, child_is_source) = match request.action {
        UmlQuickCommand::AddNote => ("uml.note", "Note", (width + 48.0, -16.0), (136.0, 78.0), "uml.note_link", false),
        UmlQuickCommand::AddConstraint => ("uml.constraint", "Constraint", (-180.0, -32.0), (134.0, 76.0), "uml.constraint_link", true),
        UmlQuickCommand::AddSubtype | UmlQuickCommand::AddSupertype => {
            let interface = subject_type == UmlType::Interface;
            let size = if interface { (120.0, 76.0) } else { (185.0, 112.0) };
            let below = request.action == UmlQuickCommand::AddSubtype;
            let offset = if below { (0.0, height + 90.0) } else { (0.0, -size.1 - 90.0) };
            (subtype_kind, if interface { "Interface" } else { "Class" }, offset, size, "uml.generalization", below)
        }
        UmlQuickCommand::AddRealizingClass => ("rust.struct", "Class", (width + 65.0, height + 50.0), (185.0, 112.0), "uml.interface_realization", true),
        UmlQuickCommand::AddReception => return Err("reception must use the member command".into()),
    };
    let new_id = fresh_id(section, "node", request.expected_revision)?;
    let new_view_id = fresh_id(section, "view", request.expected_revision)?;
    let relation_id = fresh_id(section, "relation", request.expected_revision)?;
    let relation_view_id = fresh_id(section, "relation-view", request.expected_revision)?;
    let owner_id = fresh_id(section, "owner", request.expected_revision)?;
    let name = unique_name(section, prefix)?;
    let z_index = diagram.views.iter().map(|view| view.z_index).max().unwrap_or(0)
        .checked_add(1).ok_or("z-order overflow")?;
    let edge_z_index = z_index.checked_add(1).ok_or("z-order overflow")?;
    let child_x = left + offset.0;
    let mut child_y = top + offset.1;
    // Repeated contextual creation must not pile several nodes into the same position.
    for _ in 0..=diagram.views.len() {
        let overlaps = diagram.views.iter().any(|view| match view.geometry {
            Geometry::Rect { x, y, width, height } => child_x < x + width + 8.0
                && child_x + child_size.0 + 8.0 > x
                && child_y < y + height + 8.0 && child_y + child_size.1 + 8.0 > y,
            _ => false,
        });
        if !overlaps { break; }
        child_y += child_size.1 + 24.0;
    }
    section.elements.push(Element {
        id: new_id.clone(), kind: child_kind.into(), name,
        label: None, description: None, properties: Default::default(),
        annotations: Vec::new(), implementations: Vec::new(), content: None,
    });
    section.diagrams.iter_mut().find(|diagram| diagram.id == request.diagram_id)
        .ok_or("diagram not found")?.views.push(View {
            id: new_view_id, subject: Some(new_id.clone()), label: None,
            geometry: Geometry::Rect { x: child_x, y: child_y, width: child_size.0, height: child_size.1 },
            style: Style::default(), z_index,
        });
    let (source, target) = if child_is_source {
        (new_id.as_str(), request.element_id.as_str())
    } else { (request.element_id.as_str(), new_id.as_str()) };
    let geometry = create_relation::route(section, &request.diagram_id, source, target)?;
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("routing".into(), serde_json::json!("automatic"));
    section.relations.push(Relation {
        id: relation_id.clone(), kind: relation_kind.into(), from: source.into(), to: target.into(), properties,
    });
    section.diagrams.iter_mut().find(|diagram| diagram.id == request.diagram_id)
        .ok_or("diagram not found")?.views.push(View {
            id: relation_view_id, subject: Some(relation_id), label: None,
            geometry, style: Style::default(), z_index: edge_z_index,
        });
    if request.action == UmlQuickCommand::AddConstraint {
        section.relations.push(Relation {
            id: owner_id, kind: "owns".into(), from: request.element_id.clone(), to: new_id,
            properties: Default::default(),
        });
    }
    Ok(())
}

pub(crate) async fn execute_quick_edit(
    context: &Context, projects: &ProjectService, request: ModelingQuickEditRequested,
) -> Result<u64, String> {
    let document = projects.snapshot(&request.project_id).map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision {
        return Err("stale UML Quick Edit; refresh project revision".into());
    }
    let mut section = from_document(&document)?;
    apply_quick_edit(&mut section, &request)?;
    Graph::new(&section.to_model(document.id().as_str(), document.name()))
        .validate().map_err(|error| error.to_string())?;
    let state = serde_json::to_value(section).map_err(|error| error.to_string())?;
    match context.query::<ProjectReplace>(Routing::Unscoped, ProjectReplaceRequested {
        id: request.project_id, namespace: namespace_for(&document).into(),
        expected_revision: request.expected_revision, state,
    }).await.map_err(|error| error.to_string())? {
        QueryOutcome::Answer(Ok(document)) => Ok(document.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ModelingSection {
        let model = serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        ModelingSection::from_model(&model)
    }
    #[test]
    fn shared_commands_work_for_class_and_interface() {
        for subject_kind in ["rust.struct", "rust.trait", "uml.class", "uml.interface"] {
            for action in [UmlQuickCommand::AddNote, UmlQuickCommand::AddConstraint,
                UmlQuickCommand::AddReception, UmlQuickCommand::AddSubtype, UmlQuickCommand::AddSupertype] {
                let mut section = fixture();
                section.elements.iter_mut().find(|element| element.id == "type-task").unwrap().kind = subject_kind.into();
                let before = section.elements.len();
                let request = ModelingQuickEditRequested {
                    project_id: ProjectId::new("test-quick").unwrap(), diagram_id: section.diagrams[0].id.clone(),
                    element_id: "type-task".into(), action, expected_revision: 0,
                };
                apply_quick_edit(&mut section, &request).unwrap();
                assert_eq!(section.elements.len(), before + 1);
                Graph::new(&section.to_model("test-quick", "test")).validate().unwrap();
                if matches!(action, UmlQuickCommand::AddSubtype | UmlQuickCommand::AddSupertype) {
                    let child = section.elements.last().unwrap();
                    assert_eq!(child.kind, subject_kind);
                    let edge = section.relations.iter().find(|edge| edge.kind == "uml.generalization").unwrap();
                    if action == UmlQuickCommand::AddSubtype {
                        assert_eq!(edge.from, child.id);
                        assert_eq!(edge.to, "type-task");
                    } else {
                        assert_eq!(edge.from, "type-task");
                        assert_eq!(edge.to, child.id);
                    }
                }
            }
        }
    }
    #[test]
    fn a_package_can_be_annotated_but_cannot_receive_a_reception() {
        let mut section = fixture();
        section.elements.iter_mut().find(|element| element.id == "type-task").unwrap().kind = "uml.package".into();
        let mut request = ModelingQuickEditRequested {
            project_id: ProjectId::new("test-quick").unwrap(), diagram_id: section.diagrams[0].id.clone(),
            element_id: "type-task".into(), action: UmlQuickCommand::AddReception, expected_revision: 0,
        };
        let before = section.clone();
        assert!(apply_quick_edit(&mut section, &request).is_err());
        assert_eq!(section, before);
        request.action = UmlQuickCommand::AddNote;
        apply_quick_edit(&mut section, &request).unwrap();
        assert!(section.elements.iter().any(|element| element.kind == "uml.note"));
    }
}
