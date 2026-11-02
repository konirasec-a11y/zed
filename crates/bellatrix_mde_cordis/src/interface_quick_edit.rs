//! Interface Quick Edit commands. Every gesture creates a versioned domain
//! change through Cordis; UI overlays never own semantic state.
use crate::{create_relation, project_integration::{from_document, namespace_for}};
use bellatrix_mde_model::{Element, Geometry, ModelingSection, Relation, Style, View};
use bellatrix_mde_modeling::Graph;
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceQuickAction {
    AddNote,
    AddConstraint,
    AddReception,
    AddSubinterface,
    AddSuperinterface,
    AddRealizingClass,
}

#[derive(Clone, Debug)]
pub struct ModelingInterfaceQuickEditRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub interface_id: String,
    pub action: InterfaceQuickAction,
    pub expected_revision: u64,
}
pub struct ModelingInterfaceQuickEdit;
impl Event for ModelingInterfaceQuickEdit {
    const NAME: &'static str = "bellatrix.mde.interface-quick-edit.v1";
    type Args = ModelingInterfaceQuickEditRequested;
    type Output = Result<u64, String>;
}

fn id_in_use(section: &ModelingSection, id: &str) -> bool {
    section.elements.iter().any(|v| v.id == id)
        || section.relations.iter().any(|v| v.id == id)
        || section.diagrams.iter().any(|d|d.id==id || d.views.iter().any(|v|v.id==id))
}
fn fresh_id(section: &ModelingSection, label: &str, revision: u64) -> Result<String, String> {
    let mut i=revision.checked_add(1).ok_or("identifier overflow")?;
    loop {
        let id=format!("mde-interface-{label}-{i}");
        if !id_in_use(section,&id) {return Ok(id);}
        i=i.checked_add(1).ok_or("identifier overflow")?;
    }
}
fn unique_name(section:&ModelingSection, prefix:&str)->String {
    let existing=section.elements.iter().map(|e|e.name.as_str()).collect::<Vec<_>>();
    (1_u64..).map(|n|format!("{prefix}{n}"))
        .find(|name|!existing.contains(&name.as_str()))
        .expect("finite model cannot exhaust all names")
}

fn insert_relation(section:&mut ModelingSection, id:String, kind:&str, from:&str, to:&str) {
    section.relations.push(Relation {
        id,kind:kind.into(),from:from.into(),to:to.into(),
        properties:Default::default(),
    });
}

/// One complete model transaction. New nodes are not created without their
/// required UML connector; failures reject the entire ProjectReplace.
pub(crate) fn apply_quick_edit(
    section:&mut ModelingSection, req:&ModelingInterfaceQuickEditRequested,
) -> Result<(),String> {
    let interface=section.elements.iter().find(|e|e.id==req.interface_id)
        .ok_or("interface not found")?;
    if !matches!(interface.kind.as_str(),"rust.trait"|"uml.interface") {
        return Err("Interface Quick Edit requires a UML Interface".into());
    }
    let view=section.diagrams.iter().find(|d|d.id==req.diagram_id)
        .ok_or("diagram not found")?.views.iter()
        .find(|v|v.subject.as_deref()==Some(req.interface_id.as_str()))
        .ok_or("interface not visible in diagram")?;
    let (x,y,width,height)=match view.geometry {
        Geometry::Rect{x,y,width,height}=>(x,y,width,height),
        _=>return Err("interface view has incompatible geometry".into()),
    };
    let action=req.action;
    let (kind,prefix,offset,size,link)=match action {
        InterfaceQuickAction::AddNote=>
            ("uml.note","Note",(width+48.0,-16.0),(136.0,78.0),Some("uml.note_link")),
        InterfaceQuickAction::AddConstraint=>
            ("uml.constraint","Constraint",(-145.0,-32.0),(134.0,76.0),Some("uml.constraint_link")),
        InterfaceQuickAction::AddReception=>
            ("uml.reception","Reception",(0.0,0.0),(0.0,0.0),Some("owns")),
        InterfaceQuickAction::AddSubinterface=>
            ("rust.trait","Interface",(0.0,height+90.0),(185.0,112.0),Some("uml.generalization")),
        InterfaceQuickAction::AddSuperinterface=>
            ("rust.trait","Interface",(0.0,-height-95.0),(185.0,112.0),Some("uml.generalization")),
        InterfaceQuickAction::AddRealizingClass=>
            ("rust.struct","Class",(width+65.0,height+50.0),(185.0,112.0),Some("uml.interface_realization")),
    };
    let new_id=fresh_id(section,"node",req.expected_revision)?;
    let name=unique_name(section,prefix);
    section.elements.push(Element {
        id:new_id.clone(),kind:kind.into(),name,
        label:None,description:None,properties:Default::default(),
        annotations:Vec::new(),implementations:Vec::new(),content:None,
    });
    let Some(link)=link else{return Ok(());};
    if link=="owns" {
        let owns_id=fresh_id(section,"owns",req.expected_revision)?;
        insert_relation(section,owns_id,"owns",&req.interface_id,&new_id);
        return Ok(());
    }
    let diagram=section.diagrams.iter_mut().find(|d|d.id==req.diagram_id)
        .ok_or("diagram not found")?;
    let z_index=diagram.views.iter().map(|v|v.z_index).max().unwrap_or(0)
        .checked_add(1).ok_or("z-order overflow")?;
    let new_view=fresh_id_for_view(&new_id);
    diagram.views.push(View {
        id:new_view,subject:Some(new_id.clone()),label:None,
        geometry:Geometry::Rect { x:x+offset.0,y:y+offset.1,width:size.0,height:size.1 },
        style:Style::default(),z_index,
    });
    let (from,to)=match action {
        InterfaceQuickAction::AddSubinterface |
        InterfaceQuickAction::AddRealizingClass => (new_id.as_str(),req.interface_id.as_str()),
        InterfaceQuickAction::AddSuperinterface => (req.interface_id.as_str(),new_id.as_str()),
        InterfaceQuickAction::AddNote => (req.interface_id.as_str(),new_id.as_str()),
        InterfaceQuickAction::AddConstraint => (new_id.as_str(),req.interface_id.as_str()),
        InterfaceQuickAction::AddReception => unreachable!(),
    };
    let geometry=create_relation::route(section,&req.diagram_id,from,to)?;
    let edge_id=fresh_id(section,"link",req.expected_revision)?;
    let edge_view_id=fresh_id(section,"linkview",req.expected_revision)?;
    let mut properties=std::collections::BTreeMap::new();
    properties.insert("routing".into(),serde_json::json!("automatic"));
    section.relations.push(Relation {
        id:edge_id.clone(),kind:link.into(),from:from.into(),to:to.into(),properties,
    });
    let diagram=section.diagrams.iter_mut().find(|d|d.id==req.diagram_id)
        .ok_or("diagram not found")?;
    diagram.views.push(View {
        id:edge_view_id,subject:Some(edge_id),label:None,
        geometry,style:Style::default(),
        z_index:z_index.checked_add(1).ok_or("z-order overflow")?,
    });
    if action==InterfaceQuickAction::AddConstraint {
        let owned=fresh_id(section,"constraint-owner",req.expected_revision)?;
        insert_relation(section,owned,"owns",&req.interface_id,&new_id);
    }
    Ok(())
}
fn fresh_id_for_view(id:&str)->String {format!("{id}-view")}

pub(crate) async fn execute_quick_edit(
    ctx:&Context,projects:&ProjectService,req:ModelingInterfaceQuickEditRequested,
)->Result<u64,String>{
    let document=projects.snapshot(&req.project_id).map_err(|e|e.to_string())?;
    if document.revision()!=req.expected_revision {
        return Err("stale Interface Quick Edit; refresh project revision".into());
    }
    let mut section=from_document(&document)?;
    apply_quick_edit(&mut section,&req)?;
    Graph::new(&section.to_model(document.id().as_str(),document.name()))
        .validate().map_err(|e|e.to_string())?;
    let state=serde_json::to_value(section).map_err(|e|e.to_string())?;
    let namespace=namespace_for(&document);
    match ctx.query::<ProjectReplace>(Routing::Unscoped,ProjectReplaceRequested {
        id:req.project_id,namespace:namespace.into(),expected_revision:req.expected_revision,state
    }).await.map_err(|e|e.to_string())? {
        QueryOutcome::Answer(Ok(document))=>Ok(document.revision()),
        QueryOutcome::Answer(Err(e))=>Err(e.to_string()),
        QueryOutcome::Miss=>Err("ProjectService unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->ModelingSection {
        let m=serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        ModelingSection::from_model(&m)
    }
    #[test]
    fn every_interface_quick_action_creates_a_real_graph_change() {
        let actions=[
            InterfaceQuickAction::AddNote,InterfaceQuickAction::AddConstraint,
            InterfaceQuickAction::AddReception,InterfaceQuickAction::AddSubinterface,
            InterfaceQuickAction::AddSuperinterface,InterfaceQuickAction::AddRealizingClass,
        ];
        let mut section=fixture();
        let diagram_id=section.diagrams.first().unwrap().id.clone();
        let iface=section.elements.iter().find(|e|e.kind=="rust.trait")
            .map(|e|e.id.clone());
        if let Some(interface_id)=iface {
            for (i,action) in actions.iter().enumerate() {
                let before=section.elements.len();
                let req=ModelingInterfaceQuickEditRequested {
                    project_id:ProjectId::new("test-quick").unwrap(),diagram_id:diagram_id.clone(),
                    interface_id:interface_id.clone(),action:*action,expected_revision:i as u64,
                };
                apply_quick_edit(&mut section,&req).unwrap();
                assert_eq!(section.elements.len(),before+1);
                Graph::new(&section.to_model("test-quick","test")).validate().unwrap();
            }
        }
    }
}
