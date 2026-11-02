//! Editable UML name expression, based on the reference's grammar:
//! `<< stereotype >> [+ | # | - | ~] name`.
//! One domain revision applies all fields atomically through ProjectService.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::ModelingSection;
use bellatrix_mde_modeling::Graph;
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Debug)]
pub struct ModelingSetNameExpressionRequested {
    pub project_id: ProjectId,
    pub element_id: String,
    pub expression: String,
    pub expected_revision: u64,
}
pub struct ModelingSetNameExpression;
impl Event for ModelingSetNameExpression {
    const NAME: &'static str="bellatrix.mde.set-name-expression.v1";
    type Args=ModelingSetNameExpressionRequested;
    type Output=Result<u64,String>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct NameExpression {
    name:String,
    stereotype:Option<String>,
    visibility:Option<&'static str>,
}
fn parse(expression:&str)->Result<NameExpression,String>{
    let mut rest=expression.trim();
    let stereotype=if let Some(value)=rest.strip_prefix("<<") {
        let (text,next)=value.split_once(">>").ok_or("unclosed UML stereotype")?;
        let text=text.trim();
        if text.is_empty(){return Err("empty UML stereotype".into());}
        rest=next.trim_start();
        Some(text.to_owned())
    }else if let Some(value)=rest.strip_prefix('«') {
        let (text,next)=value.split_once('»').ok_or("unclosed UML stereotype")?;
        let text=text.trim();
        if text.is_empty(){return Err("empty UML stereotype".into());}
        rest=next.trim_start();
        Some(text.to_owned())
    } else {None};
    let visibility=match rest.chars().next() {
        Some('+')=>Some("public"),Some('#')=>Some("protected"),
        Some('-')=>Some("private"),Some('~')=>Some("package"),_=>None,
    };
    if visibility.is_some(){rest=rest[1..].trim_start();}
    let name=rest.trim();
    if name.is_empty() {return Err("UML element name is required".into());}
    if name.contains("<<") || name.contains(">>") {
        return Err("UML name expression has malformed stereotype".into());
    }
    Ok(NameExpression {name:name.into(),stereotype,visibility})
}
fn apply(section:&mut ModelingSection, id:&str, expression:&str)->Result<(),String>{
    let parsed=parse(expression)?;
    let element=section.elements.iter_mut().find(|e|e.id==id).ok_or("element not found")?;
    if !matches!(element.kind.as_str(),"rust.trait"|"uml.interface"|"rust.struct"|"uml.class"
        |"uml.signal"|"uml.enumeration"|"uml.data_type"|"uml.primitive_type"
        |"uml.association_class"|"uml.template") {
        return Err("UML name expression requires a classifier".into());
    }
    element.name=parsed.name;
    if let Some(stereotype)=parsed.stereotype {
        element.properties.insert("stereotype".into(),serde_json::json!(stereotype));
    } else {
        element.properties.remove("stereotype");
    }
    if let Some(visibility)=parsed.visibility {
        element.properties.insert("visibility".into(),serde_json::json!(visibility));
    }
    Ok(())
}
pub(crate) async fn set_name_expression(
    ctx:&Context,projects:&ProjectService,req:ModelingSetNameExpressionRequested,
)->Result<u64,String>{
    let document=projects.snapshot(&req.project_id).map_err(|e|e.to_string())?;
    if document.revision()!=req.expected_revision{
        return Err("stale UML name expression; reload project".into());
    }
    let mut section=from_document(&document)?;
    apply(&mut section,&req.element_id,&req.expression)?;
    Graph::new(&section.to_model(document.id().as_str(),document.name()))
        .validate().map_err(|e|e.to_string())?;
    let state=serde_json::to_value(section).map_err(|e|e.to_string())?;
    let namespace=namespace_for(&document);
    match ctx.query::<ProjectReplace>(Routing::Unscoped,ProjectReplaceRequested{
        id:req.project_id,namespace:namespace.into(),
        expected_revision:req.expected_revision,state,
    }).await.map_err(|e|e.to_string())?{
        QueryOutcome::Answer(Ok(doc))=>Ok(doc.revision()),
        QueryOutcome::Answer(Err(e))=>Err(e.to_string()),
        QueryOutcome::Miss=>Err("ProjectService unavailable".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_stereotype_visibility_and_name() {
        assert_eq!(parse("<< service >> # Billing Api").unwrap(),NameExpression {
            name:"Billing Api".into(),stereotype:Some("service".into()),visibility:Some("protected"),
        });
        assert_eq!(parse("«deprecated» - Old").unwrap().visibility,Some("private"));
        assert_eq!(parse("Interface1").unwrap().name,"Interface1");
        for invalid in ["","<< x","<< >> Interface","+","<<x>>"]{
            assert!(parse(invalid).is_err(),"{invalid}");
        }
    }
}
