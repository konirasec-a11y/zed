//! The owner type supplies capabilities; member storage is independent of its symbol.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::{Element, Geometry, ModelingSection, Relation};
use bellatrix_mde_modeling::{Graph, UmlCapability, UmlType, classifier_contents};
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelingMemberKind { Attribute, Operation, Reception }

#[derive(Clone, Debug)]
pub struct ModelingAddMemberRequested {
    pub project_id: ProjectId,
    pub owner_id: String,
    pub kind: ModelingMemberKind,
    pub expected_revision: u64,
}
pub struct ModelingAddMember;
impl Event for ModelingAddMember {
    const NAME: &'static str = "bellatrix.mde.add-member.v1";
    type Args = ModelingAddMemberRequested;
    type Output = Result<u64, String>;
}

pub(crate) fn grow_classifier_views(section: &mut ModelingSection, owner_id: &str) {
    let Some(kind) = section.elements.iter().find(|element| element.id == owner_id)
        .and_then(|element| UmlType::from_kind(&element.kind)) else { return };
    let contents = classifier_contents(section, owner_id);
    let minimum = if kind == UmlType::Interface {
        contents.minimum_interface_height()
    } else { 44.0 + contents.compartments_height() };
    let mut diagrams = Vec::new();
    for diagram in &mut section.diagrams {
        let mut changed = false;
        for view in diagram.views.iter_mut().filter(|view| view.subject.as_deref() == Some(owner_id)) {
            if let Geometry::Rect { height, .. } = &mut view.geometry {
                if *height < minimum { *height = minimum; changed = true; }
            }
        }
        if changed { diagrams.push(diagram.id.clone()); }
    }
    for diagram_id in diagrams { crate::create_relation::reroute(section, &diagram_id); }
}

pub(crate) fn append_member(
    section: &mut ModelingSection, owner_id: &str, kind: ModelingMemberKind, expected_revision: u64,
) -> Result<String, String> {
    let owner = section.elements.iter().find(|element| element.id == owner_id)
        .ok_or("member owner not found")?;
    let owner_type = UmlType::from_kind(&owner.kind).ok_or("member owner is not a supported UML type")?;
    let capability = match kind {
        ModelingMemberKind::Attribute => UmlCapability::Attribute,
        ModelingMemberKind::Operation => UmlCapability::Operation,
        ModelingMemberKind::Reception => UmlCapability::Reception,
    };
    if !owner_type.supports(capability) { return Err("element type cannot own this member kind".into()); }
    let rust_classifier = matches!(owner.kind.as_str(), "rust.struct" | "rust.trait");
    let (model_kind, prefix, base) = match kind {
        ModelingMemberKind::Attribute => (if rust_classifier { "rust.field" } else { "uml.attribute" }, "attribute", "Attribute"),
        ModelingMemberKind::Operation => (if rust_classifier { "rust.function" } else { "uml.operation" }, "operation", "Operation"),
        ModelingMemberKind::Reception => ("uml.reception", "reception", "Reception"),
    };
    let mut name_sequence = 1_u64;
    let name = loop {
        let candidate = format!("{base}{name_sequence}");
        let duplicate = section.relations.iter().any(|relation| relation.kind == "owns" && relation.from == owner_id
            && section.elements.iter().any(|element| element.id == relation.to && element.name == candidate));
        if !duplicate { break candidate; }
        name_sequence = name_sequence.checked_add(1).ok_or("name sequence overflow")?;
    };
    let mut sequence = expected_revision.checked_add(1).ok_or("identifier overflow")?;
    let (member_id, relation_id) = loop {
        let member_id = format!("mde-{prefix}-{sequence}");
        let relation_id = format!("mde-owns-{prefix}-{sequence}");
        let occupied = section.elements.iter().any(|element| element.id == member_id || element.id == relation_id)
            || section.relations.iter().any(|relation| relation.id == member_id || relation.id == relation_id)
            || section.diagrams.iter().any(|diagram| diagram.id == member_id || diagram.id == relation_id
                || diagram.views.iter().any(|view| view.id == member_id || view.id == relation_id));
        if !occupied { break (member_id, relation_id); }
        sequence = sequence.checked_add(1).ok_or("identifier overflow")?;
    };
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("visibility".into(), serde_json::json!("public"));
    if model_kind == "rust.field" {
        properties.insert("declaration".into(), serde_json::json!(format!("pub {prefix}: String")));
    } else if model_kind == "rust.function" {
        properties.insert("signature".into(), serde_json::json!(format!("pub fn {prefix}(&self)")));
    }
    section.elements.push(Element {
        id: member_id.clone(), kind: model_kind.into(), name,
        label: None, description: None, properties, annotations: Vec::new(),
        implementations: Vec::new(), content: None,
    });
    section.relations.push(Relation {
        id: relation_id, kind: "owns".into(), from: owner_id.into(),
        to: member_id.clone(), properties: Default::default(),
    });
    grow_classifier_views(section, owner_id);
    Ok(member_id)
}

pub(crate) async fn add_member(
    context: &Context, projects: &ProjectService, request: ModelingAddMemberRequested,
) -> Result<u64, String> {
    let document = projects.snapshot(&request.project_id).map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision { return Err("stale member creation; reload MDE snapshot".into()); }
    let mut section = from_document(&document)?;
    append_member(&mut section, &request.owner_id, request.kind, request.expected_revision)?;
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
    fn shared_member_command_accepts_both_classifier_types_and_rejects_files() {
        for owner_kind in ["rust.struct", "rust.trait", "uml.class", "uml.interface"] {
            let mut section = fixture();
            section.elements.iter_mut().find(|element| element.id == "type-task").unwrap().kind = owner_kind.into();
            for kind in [ModelingMemberKind::Attribute, ModelingMemberKind::Operation, ModelingMemberKind::Reception] {
                let id = append_member(&mut section, "type-task", kind, 0).unwrap();
                assert!(section.relations.iter().any(|relation| relation.kind == "owns" && relation.from == "type-task" && relation.to == id));
            }
            let before = section.clone();
            assert!(append_member(&mut section, "file-main", ModelingMemberKind::Attribute, 0).is_err());
            assert_eq!(section, before);
            Graph::new(&section.to_model("project-todo", "Task List / Todo Project")).validate().unwrap();
        }
    }
}
