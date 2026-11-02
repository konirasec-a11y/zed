//! Shared UML edits; the selected semantic type chooses the permitted specialization.
use crate::{add_member, create_relation, project_integration::{from_document, namespace_for}};
use bellatrix_mde_modeling::quick_relation_rule;
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
    if matches!(request.action,
        UmlQuickCommand::AddTemplateParameter | UmlQuickCommand::AddPort | UmlQuickCommand::AddPart
    ) {
        return crate::class_features::insert_feature(
            section, &request.diagram_id, &request.element_id, request.action,
            request.expected_revision,
        );
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
    // Contextual gesture decides which new type and placement to create.
    // The graph layer, not this command adapter, defines edge direction,
    // semantic kind and aggregation role for every action.
    let (child_kind, prefix, offset, child_size) = match request.action {
        UmlQuickCommand::AddNote =>
            ("uml.note", "Note", (width + 48.0, -16.0), (136.0, 78.0)),
        UmlQuickCommand::AddConstraint =>
            ("uml.constraint", "Constraint", (-180.0, -32.0), (134.0, 76.0)),
        UmlQuickCommand::AddSubtype | UmlQuickCommand::AddSupertype => {
            let interface = subject_type == UmlType::Interface;
            let size = if interface { (120.0, 76.0) } else { (185.0, 112.0) };
            let below = request.action == UmlQuickCommand::AddSubtype;
            let offset = if below { (0.0, height + 90.0) }
                else { (0.0, -size.1 - 90.0) };
            (subtype_kind, if interface { "Interface" } else { "Class" }, offset, size)
        }
        UmlQuickCommand::AddRealizingClass =>
            ("rust.struct", "Class", (width + 65.0, height + 50.0), (185.0, 112.0)),
        UmlQuickCommand::AddProvidedInterface =>
            ("rust.trait", "Interface", (width + 55.0, -75.0), (120.0, 76.0)),
        UmlQuickCommand::AddRequiredInterface =>
            ("rust.trait", "Interface", (width + 55.0, height + 10.0), (120.0, 76.0)),
        UmlQuickCommand::AddAssociatedClass | UmlQuickCommand::AddAggregatedClass
            | UmlQuickCommand::AddCompositedClass =>
            ("rust.struct", "Class", (width + 80.0, 0.0), (185.0, 112.0)),
        UmlQuickCommand::AddTemplateParameter | UmlQuickCommand::AddPort
            | UmlQuickCommand::AddPart =>
            return Err("contained features must use Class specialization".into()),
        UmlQuickCommand::AddReception => return Err("reception must use member command".into()),
    };
    let relation_rule = quick_relation_rule(request.action)
        .ok_or("Quick Edit action does not create a graph relationship")?;
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
    let (source, target) = if relation_rule.child_is_source {
        (new_id.as_str(), request.element_id.as_str())
    } else { (request.element_id.as_str(), new_id.as_str()) };
    let geometry = create_relation::route(section, &request.diagram_id, source, target)?;
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("routing".into(), serde_json::json!("automatic"));
    if let Some(aggregation) = relation_rule.aggregation {
        properties.insert("end1Aggregation".into(), serde_json::json!(aggregation));
    }
    section.relations.push(Relation {
        id: relation_id.clone(), kind: relation_rule.kind.into(), from: source.into(), to: target.into(), properties,
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
    // Quick Edit creates the same domain Relations as Toolbox. Its line must
    // participate in sibling-lane routing instead of remaining superimposed.
    create_relation::reroute(section, &request.diagram_id);
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
    fn every_class_specialization_creates_a_semantic_result() {
        let cases=[
            (UmlQuickCommand::AddTemplateParameter,"uml.template_parameter",None),
            (UmlQuickCommand::AddProvidedInterface,"rust.trait",Some("uml.interface_realization")),
            (UmlQuickCommand::AddRequiredInterface,"rust.trait",Some("uml.dependency")),
            (UmlQuickCommand::AddAssociatedClass,"rust.struct",Some("uml.association")),
            (UmlQuickCommand::AddAggregatedClass,"rust.struct",Some("uml.association")),
            (UmlQuickCommand::AddCompositedClass,"rust.struct",Some("uml.association")),
            (UmlQuickCommand::AddPort,"uml.port",None),
            (UmlQuickCommand::AddPart,"uml.part",None),
        ];
        for (action,expected_kind,relation_kind) in cases {
            let mut section=fixture();
            let diagram_id=section.diagrams[0].id.clone();
            let before=section.elements.len();
            let request=ModelingQuickEditRequested {
                project_id:ProjectId::new("class-specializations").unwrap(),
                diagram_id,element_id:"type-task".into(),action,expected_revision:0,
            };
            apply_quick_edit(&mut section,&request).unwrap();
            assert_eq!(section.elements.len(),before+1,"{action:?}");
            let child=section.elements.iter().find(|e|e.kind==expected_kind
                && e.id.starts_with("mde-")).unwrap();
            if let Some(kind)=relation_kind {
                let edge=section.relations.iter().find(|r|r.kind==kind).unwrap();
                assert!(edge.from==child.id || edge.to==child.id);
                let aggregation=match action {
                    UmlQuickCommand::AddAggregatedClass=>Some("shared"),
                    UmlQuickCommand::AddCompositedClass=>Some("composite"),
                    _=>None,
                };
                if let Some(value)=aggregation {
                    assert_eq!(edge.properties.get("end1Aggregation").and_then(|v|v.as_str()),Some(value));
                }
                assert!(section.diagrams.iter().flat_map(|d|&d.views).any(|v|
                    v.subject.as_deref()==Some(edge.id.as_str())));
            } else {
                assert!(section.relations.iter().any(|r|r.kind=="owns"
                    && r.from=="type-task" && r.to==child.id));
                assert_eq!(section.diagrams.iter().flat_map(|d|&d.views).any(|v|
                    v.subject.as_deref()==Some(child.id.as_str())),
                    action!=UmlQuickCommand::AddTemplateParameter);
            }
            Graph::new(&section.to_model("class-specializations","Class")).validate().unwrap();
        }
    }
    #[test]
    fn class_capability_is_not_accidentally_inherited_by_interface() {
        let mut section=fixture();
        section.elements.iter_mut().find(|e|e.id=="type-task").unwrap().kind="rust.trait".into();
        for action in [
            UmlQuickCommand::AddTemplateParameter,UmlQuickCommand::AddProvidedInterface,
            UmlQuickCommand::AddRequiredInterface,UmlQuickCommand::AddAssociatedClass,
            UmlQuickCommand::AddAggregatedClass,UmlQuickCommand::AddCompositedClass,
            UmlQuickCommand::AddPort,UmlQuickCommand::AddPart,
        ] {
            let before=section.clone();
            let diagram_id=section.diagrams[0].id.clone();
            assert!(apply_quick_edit(&mut section,&ModelingQuickEditRequested {
                project_id:ProjectId::new("class-specializations").unwrap(),
                diagram_id,
                element_id:"type-task".into(),action,expected_revision:0,
            }).is_err());
            assert_eq!(section,before,"{action:?}");
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
