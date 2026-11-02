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

fn contained_ids(section: &ModelingSection, owner_id: &str) -> std::collections::HashSet<String> {
    section.relations.iter().filter(|relation|relation.kind=="owns"&&relation.from==owner_id)
        .filter_map(|relation|section.elements.iter().find(|element|element.id==relation.to))
        .filter(|element|matches!(element.kind.as_str(),"uml.port"|"uml.part"))
        .map(|element|element.id.clone()).collect()
}

/// A parent Class move transports its contained Port and Part Views.
/// Their identities and owners remain in the MDE graph.
pub(crate) fn translate_contained_views(
    section: &mut ModelingSection, diagram_id: &str, owner_id: &str, dx: f64, dy: f64
) {
    let children=contained_ids(section,owner_id);
    if let Some(diagram)=section.diagrams.iter_mut().find(|d|d.id==diagram_id) {
        for view in &mut diagram.views {
            if view.subject.as_ref().is_some_and(|id|children.contains(id)) {
                view.geometry.translate(dx,dy);
            }
        }
    }
}

/// Ports are bound to the right border of the containing Class.
pub(crate) fn reposition_ports(section:&mut ModelingSection,diagram_id:&str,owner_id:&str) {
    let port_ids:std::collections::HashSet<_>=section.relations.iter()
        .filter(|r|r.kind=="owns"&&r.from==owner_id)
        .filter_map(|r|section.elements.iter().find(|e|e.id==r.to))
        .filter(|e|e.kind=="uml.port").map(|e|e.id.clone()).collect();
    if let Some(diagram)=section.diagrams.iter_mut().find(|d|d.id==diagram_id) {
        let box_position=diagram.views.iter()
            .find(|view|view.subject.as_deref()==Some(owner_id))
            .and_then(|view|match view.geometry {
                Geometry::Rect{x,y,width,height}=>Some((x,y,width,height)),_=>None,
            });
        if let Some((x,y,width,height))=box_position {
            for view in &mut diagram.views {
                if !view.subject.as_ref().is_some_and(|id|port_ids.contains(id)) {continue}
                if let Geometry::Rect{x:child_x,y:child_y,width:child_width,height:child_height}=&mut view.geometry {
                    *child_x=x+width-*child_width/2.0;
                    *child_y=(*child_y).clamp(y-*child_height/2.0,
                        y+height-*child_height/2.0);
                }
            }
        }
    }
}

/// A part must remain inside its Class when the parent is resized.
pub(crate) fn class_minimum_bounds(
    section:&ModelingSection,diagram_id:&str,owner_id:&str
) -> Option<(f64,f64)> {
    let diagram=section.diagrams.iter().find(|d|d.id==diagram_id)?;
    let (x,y)=diagram.views.iter().find(|v|v.subject.as_deref()==Some(owner_id))
        .and_then(|view|match view.geometry {
            Geometry::Rect{x,y,..}=>Some((x,y)),_=>None,
        })?;
    let part_ids:std::collections::HashSet<_>=section.relations.iter()
        .filter(|r|r.kind=="owns"&&r.from==owner_id)
        .filter_map(|r|section.elements.iter().find(|e|e.id==r.to))
        .filter(|e|e.kind=="uml.part").map(|e|e.id.clone()).collect();
    let mut min_width:f64=80.0;
    let mut min_height:f64=48.0;
    for view in &diagram.views {
        if !view.subject.as_ref().is_some_and(|id|part_ids.contains(id)) {continue}
        if let Geometry::Rect{x:child_x,y:child_y,width,height} = view.geometry {
            min_width=min_width.max(child_x+width-x+12.0);
            min_height=min_height.max(child_y+height-y+12.0);
        }
    }
    Some((min_width,min_height))
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
    let z_index=diagram.views.iter().map(|view|view.z_index).max()
        .unwrap_or(0).checked_add(1).ok_or("z-index overflow")?;
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
    if action==UmlQuickCommand::AddPart {
        if let Some((min_width,min_height))=class_minimum_bounds(section,diagram_id,owner_id) {
            if let Some(view)=section.diagrams.iter_mut().find(|d|d.id==diagram_id)
                .and_then(|d|d.views.iter_mut().find(|v|v.subject.as_deref()==Some(owner_id))) {
                if let Geometry::Rect{width,height,..}=&mut view.geometry {
                    *width=(*width).max(min_width);
                    *height=(*height).max(min_height);
                }
            }
        }
        reposition_ports(section,diagram_id,owner_id);
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
