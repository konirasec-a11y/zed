//! UML Class-only feature composition. Shared Quick Edit is the transaction
//! boundary: this module only inserts typed members, Views and ownership.
use bellatrix_mde_model::{Element, Geometry, ModelingSection, Relation, Style, View};
use bellatrix_mde_modeling::UmlQuickCommand;

fn fresh(section: &ModelingSection, prefix: &str, revision: u64) -> Result<String, String> {
    let mut index=revision.checked_add(1).ok_or("identifier overflow")?;
    loop {
        let id=format!("mde-class-{prefix}-{index}");
        if !section.elements.iter().any(|element|element.id==id)
            && !section.relations.iter().any(|relation|relation.id==id)
            && !section.diagrams.iter().any(|diagram|diagram.id==id
                || diagram.views.iter().any(|view|view.id==id)) { return Ok(id); }
        index=index.checked_add(1).ok_or("identifier overflow")?;
    }
}

fn name(section: &ModelingSection, prefix: &str) -> Result<String,String> {
    for sequence in 1_u64.. {
        let candidate=format!("{prefix}{sequence}");
        if !section.elements.iter().any(|element|element.name==candidate) {return Ok(candidate);}
    }
    Err("member name overflow".into())
}

pub(crate) fn insert_feature(
    section: &mut ModelingSection, diagram_id: &str,
    owner_id: &str, action: UmlQuickCommand, revision: u64,
) -> Result<(),String> {
    let owner=section.elements.iter().find(|element|element.id==owner_id)
        .ok_or("Class missing")?;
    if !matches!(owner.kind.as_str(),"rust.struct"|"uml.class") {
        return Err("Class specialization requires a Class".into());
    }
    let diagram=section.diagrams.iter().find(|diagram|diagram.id==diagram_id)
        .ok_or("diagram missing")?;
    let owner_view=diagram.views.iter().find(|view|view.subject.as_deref()==Some(owner_id))
        .ok_or("Class has no View in diagram")?;
    let (x,y,width,height)=match owner_view.geometry {
        Geometry::Rect {x,y,width,height}=>(x,y,width,height),
        _=>return Err("Class geometry must be Rect".into()),
    };
    let (kind,prefix,shape)=match action {
        UmlQuickCommand::AddTemplateParameter=>("uml.template_parameter","TemplateParameter",None),
        UmlQuickCommand::AddPort=>("uml.port","Port",Some((16.0,16.0))),
        UmlQuickCommand::AddPart=>("uml.part","Part",Some((86.0,44.0))),
        _=>return Err("not a contained Class specialization".into()),
    };
    let previous=section.relations.iter()
        .filter(|relation|relation.kind=="owns" && relation.from==owner_id)
        .filter(|relation|section.elements.iter().any(|element|element.id==relation.to&&element.kind==kind))
        .count();
    let node_id=fresh(section,"feature",revision)?;
    let owner_relation_id=fresh(section,"owns",revision)?;
    let label=name(section,prefix)?;
    let mut properties=std::collections::BTreeMap::new();
    properties.insert("visibility".into(),serde_json::json!("public"));
    if action==UmlQuickCommand::AddTemplateParameter {
        properties.insert("parameterType".into(),serde_json::json!("UMLClassifier"));
    }
    if action==UmlQuickCommand::AddPort {
        properties.insert("isService".into(),serde_json::json!(false));
    }
    if action==UmlQuickCommand::AddPart {
        properties.insert("type".into(),serde_json::json!(""));
    }
    section.elements.push(Element {
        id:node_id.clone(),kind:kind.into(),name:label,label:None,
        description:None,properties,annotations:Vec::new(),implementations:Vec::new(),content:None,
    });
    section.relations.push(Relation {
        id:owner_relation_id,kind:"owns".into(),from:owner_id.into(),to:node_id.clone(),
        properties:Default::default(),
    });
    if let Some((element_width,element_height))=shape {
        let view_id=fresh(section,"feature-view",revision)?;
        let z_index=diagram.views.iter().map(|view|view.z_index).max()
            .unwrap_or(0).checked_add(1).ok_or("z-index overflow")?;
        let (position_x,position_y)=if action==UmlQuickCommand::AddPort {
            (x+width-element_width/2.0,
                y+height*0.3+previous as f64*24.0)
        }else {
            (x+12.0+(previous%2) as f64*94.0,
             y+38.0+(previous/2) as f64*52.0)
        };
        let diagram=section.diagrams.iter_mut().find(|diagram|diagram.id==diagram_id)
            .ok_or("diagram missing")?;
        diagram.views.push(View {
            id:view_id,subject:Some(node_id),label:None,
            geometry:Geometry::Rect {
                x:position_x,y:position_y,width:element_width,height:element_height,
            },
            style:Style::default(),z_index,
        });
    }
    if action==UmlQuickCommand::AddTemplateParameter {
        crate::add_member::grow_classifier_views(section,owner_id);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->ModelingSection {
        let model=serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        ModelingSection::from_model(&model)
    }
    #[test]
    fn template_port_part_are_owned_and_visually_distinct() {
        let mut section=fixture();
        for (idx,action) in [
            UmlQuickCommand::AddTemplateParameter,
            UmlQuickCommand::AddPort,
            UmlQuickCommand::AddPart,
        ].into_iter().enumerate() {
            let diagram_id=section.diagrams[0].id.clone();
            insert_feature(&mut section,&diagram_id,"type-task",action,idx as u64).unwrap();
        }
        for kind in ["uml.template_parameter","uml.port","uml.part"] {
            let element=section.elements.iter().find(|element|element.kind==kind).unwrap();
            assert!(section.relations.iter().any(|r|r.kind=="owns"
                && r.from=="type-task" && r.to==element.id));
            let has_view=section.diagrams.iter().flat_map(|d|&d.views)
                .any(|v|v.subject.as_deref()==Some(element.id.as_str()));
            assert_eq!(has_view,kind!="uml.template_parameter");
        }
    }
}
