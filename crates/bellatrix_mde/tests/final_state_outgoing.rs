use bellatrix_mde::{Format, Project, StructuralSeverity, commands::{Command, apply}, decode, encode, uml::root::Model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_FINAL_STATE_NO_OUTGOING_TRANSITIONS";
const UNKNOWN: &str = "MDE_FINAL_STATE_OUTGOING_UNVERIFIED";
fn fixture() -> Result<Project> {
    Ok(serde_json::from_value(json!({"id":"model","name":"FinalState outgoing","elements":[
        {"id":"final","kind":"uml.final_state","name":"Done"},
        {"id":"state","kind":"uml.state","name":"State"},
        {"id":"unknown","kind":"extension.unknown","name":"Unknown"},
        {"id":"wrong","kind":"uml.class","name":"Wrong"}
    ]}))?)
}
fn relation(project: &mut Project, id: &str, kind: &str, from: &str, to: &str, source: Option<Value>) -> Result {
    let properties = source.map(|value| json!({"source":value})).unwrap_or(json!({}));
    project.relations.push(serde_json::from_value(json!({"id":id,"kind":kind,"from":from,"to":to,"properties":properties}))?);
    Ok(())
}
fn object(project: &mut Project, id: &str, kind: &str, source: Option<Value>) -> Result {
    let properties = source.map(|value| json!({"source":value})).unwrap_or(json!({}));
    project.elements.push(serde_json::from_value(json!({"id":id,"kind":kind,"name":id,"properties":properties}))?);
    Ok(())
}
fn outcome(project: &Project) -> Vec<(StructuralSeverity, &'static str)> {
    Model::from_project(project).validate().into_iter()
        .filter(|issue| issue.subject_id == "final" && matches!(issue.code, ERROR | UNKNOWN))
        .map(|issue| (issue.severity, issue.code)).collect()
}
fn invalid() -> Vec<(StructuralSeverity, &'static str)> { vec![(StructuralSeverity::Error, ERROR)] }

#[test]
fn transitions_and_protocol_transitions_use_source_identity_in_both_representations() -> Result {
    for kind in ["uml.transition","uml.protocol_transition"] {
        for source in ["final","state"] {
            for is_relation in [true,false] {
                let mut project = fixture()?;
                if is_relation { relation(&mut project,"transition",kind,source,"final",None)?; }
                else { object(&mut project,"transition",kind,Some(json!({"$ref":source})))?; }
                let expected = if source == "final" { invalid() } else { vec![] };
                for format in [Format::Json, Format::Protobuf] {
                    assert_eq!(outcome(&decode(&encode(&project,format)?,format)?),expected);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn duplicate_transition_identity_is_invalid_and_delete_can_repair_an_import() -> Result {
    let mut project = fixture()?;
    relation(&mut project,"transition","uml.transition","final","state",None)?;
    let repaired=apply(&project,&Command::DeleteElement{element_id:"transition".into()},0)?;
    assert!(outcome(&repaired).is_empty());
    assert_eq!(outcome(&project),invalid());
    object(&mut project,"transition","uml.transition",Some(json!({"$ref":"state"})))?;
    assert_eq!(outcome(&project),invalid());
    Ok(())
}
