//! Add a classifier member through a typed Cordis command. The child is a
//! semantic Element linked by `owns`, never a string drawn into the Canvas.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::{Element, ModelingSection, Relation};
use bellatrix_mde_modeling::Graph;
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelingMemberKind { Attribute, Operation }

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

fn append_member(
    section: &mut ModelingSection,
    owner_id: &str,
    kind: ModelingMemberKind,
    expected_revision: u64,
) -> Result<String, String> {
    let owner = section.elements.iter().find(|e| e.id == owner_id)
        .ok_or("member owner not found")?;
    let owner_is_interface=matches!(owner.kind.as_str(),"rust.trait"|"uml.interface");
    let model_kind = match (owner.kind.as_str(), kind) {
        ("rust.struct" | "rust.trait", ModelingMemberKind::Attribute) => "rust.field",
        ("rust.struct" | "rust.trait", ModelingMemberKind::Operation) => "rust.function",
        ("uml.class" | "uml.interface", ModelingMemberKind::Attribute) => "uml.attribute",
        ("uml.class" | "uml.interface", ModelingMemberKind::Operation) => "uml.operation",
        _ => return Err("element type cannot own this member kind".into()),
    };
    let (prefix,base) = match kind {
        ModelingMemberKind::Attribute => ("attribute", "Attribute"),
        ModelingMemberKind::Operation => ("operation", "Operation"),
    };
    let mut name_sequence: u64 = 1;
    let name = loop {
        let candidate = format!("{base}{name_sequence}");
        let duplicate = section.relations.iter().any(|r| r.kind=="owns" && r.from==owner_id
            && section.elements.iter().any(|e| e.id==r.to && e.name==candidate));
        if !duplicate {break candidate;}
        name_sequence = name_sequence.checked_add(1).ok_or("name sequence overflow")?;
    };
    let mut sequence = expected_revision.checked_add(1).ok_or("identifier overflow")?;
    let (member_id,relation_id) = loop {
        let member_id = format!("mde-{prefix}-{sequence}");
        let relation_id = format!("mde-owns-{prefix}-{sequence}");
        let occupied = section.elements.iter().any(|e| e.id==member_id || e.id==relation_id)
            || section.relations.iter().any(|r| r.id==member_id || r.id==relation_id)
            || section.diagrams.iter().any(|d| d.id==member_id || d.id==relation_id ||
                d.views.iter().any(|v| v.id==member_id || v.id==relation_id));
        if !occupied {break (member_id,relation_id);}
        sequence=sequence.checked_add(1).ok_or("identifier overflow")?;
    };
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("visibility".into(), serde_json::json!("public"));
    if model_kind=="rust.field" {
        properties.insert("declaration".into(),
            serde_json::json!(format!("pub {prefix}: String")));
    } else if model_kind=="rust.function" {
        properties.insert("signature".into(),
            serde_json::json!(format!("pub fn {prefix}(&self)")));
    }
    section.elements.push(Element {
        id:member_id.clone(),kind:model_kind.into(),name,
        label:None,description:None,properties,annotations:Vec::new(),
        implementations:Vec::new(),content:None,
    });
    section.relations.push(Relation {
        id:relation_id,kind:"owns".into(),from:owner_id.into(),
        to:member_id.clone(),properties:Default::default(),
    });
    if owner_is_interface {
        crate::interface_quick_edit::grow_interface_views(section,owner_id);
    }
    Ok(member_id)
}

pub(crate) async fn add_member(
    ctx: &Context, projects: &ProjectService, req: ModelingAddMemberRequested,
) -> Result<u64, String> {
    let doc=projects.snapshot(&req.project_id).map_err(|e| e.to_string())?;
    if doc.revision()!=req.expected_revision {
        return Err("stale member creation; reload MDE snapshot".into());
    }
    let mut section=from_document(&doc)?;
    let namespace=namespace_for(&doc);
    append_member(&mut section, &req.owner_id, req.kind, req.expected_revision)?;
    Graph::new(&section.to_model(doc.id().as_str(),doc.name()))
        .validate().map_err(|e| e.to_string())?;
    let state=serde_json::to_value(section).map_err(|e| e.to_string())?;
    match ctx.query::<ProjectReplace>(Routing::Unscoped,ProjectReplaceRequested{
        id:req.project_id,namespace:namespace.into(),expected_revision:req.expected_revision,
        state,
    }).await.map_err(|e|e.to_string())? {
        QueryOutcome::Answer(Ok(document))=>Ok(document.revision()),
        QueryOutcome::Answer(Err(error))=>Err(error.to_string()),
        QueryOutcome::Miss=>Err("ProjectService unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->ModelingSection {
        let m=serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json"))
            .unwrap();
        ModelingSection::from_model(&m)
    }
    #[test]
    fn class_quick_edit_adds_owned_attribute_without_touching_files() {
        let mut s=fixture();
        let prev=s.clone();
        let id=append_member(&mut s,"type-list",ModelingMemberKind::Attribute,0).unwrap();
        assert!(s.elements.iter().any(|e|e.id==id && e.kind=="rust.field"));
        assert!(s.relations.iter().any(|r|r.kind=="owns" && r.from=="type-list" && r.to==id));
        assert_eq!(s.elements.iter().filter(|e|e.kind=="fs.file").count(),
            prev.elements.iter().filter(|e|e.kind=="fs.file").count());
        Graph::new(&s.to_model("project-todo","Task List / Todo Project")).validate().unwrap();
    }
    #[test]
    fn class_quick_edit_adds_operation_and_rejects_wrong_owner() {
        let mut s=fixture();
        let id=append_member(&mut s,"type-list",ModelingMemberKind::Operation,1).unwrap();
        assert!(s.elements.iter().any(|e|e.id==id && e.kind=="rust.function"));
        let prev=s.clone();
        assert!(append_member(&mut s,"file-main",ModelingMemberKind::Attribute,2).is_err());
        assert_eq!(prev,s);
    }
}
