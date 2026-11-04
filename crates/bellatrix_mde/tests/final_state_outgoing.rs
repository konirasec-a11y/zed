use bellatrix_mde::{Format, Project, StructuralSeverity, commands::{Command, apply}, decode, encode, uml::root::Model, validate_uml_model};
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
fn unknown() -> Vec<(StructuralSeverity, &'static str)> { vec![(StructuralSeverity::Warning, UNKNOWN)] }

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
fn missing_or_external_object_source_is_unknown_but_known_outgoing_witness_wins() -> Result {
    for source in [None,Some(json!({"href":"external.uml#Vertex"}))] {
        let mut project = fixture()?;
        object(&mut project,"incomplete","uml.transition",source)?;
        assert_eq!(outcome(&project),unknown());
        relation(&mut project,"known","uml.transition","final","state",None)?;
        assert_eq!(outcome(&project),invalid());
        project.elements.reverse();
        assert_eq!(outcome(&project),invalid());
    }
    Ok(())
}

#[test]
fn local_source_identity_excludes_final_even_when_its_metaclass_is_unknown() -> Result {
    let mut project = fixture()?;
    object(&mut project,"transition","uml.transition",Some(json!({"$ref":"unknown"})))?;
    assert!(outcome(&project).is_empty());
    assert!(Model::from_project(&project).validate().iter().any(|issue| issue.subject_id=="transition" && issue.code=="MDE_PROPERTY_CONTRACT_UNVERIFIED"));
    relation(&mut project,"edge","uml.transition","state","final",Some(json!({"href":"external.uml#Vertex"})))?;
    assert!(outcome(&project).is_empty());
    Ok(())
}

#[test]
fn invalid_source_contracts_are_not_hidden_by_external_or_known_membership() -> Result {
    for source in [json!(null),json!(42),json!([]),json!({"$ref":"missing"}),json!({"$ref":"wrong"}),json!({"href":""})] {
        let mut project = fixture()?;
        object(&mut project,"bad","uml.transition",Some(source))?;
        object(&mut project,"external","uml.transition",Some(json!({"href":"external.uml#Vertex"})))?;
        assert_eq!(outcome(&project),invalid());
    }
    for from in ["","missing","wrong"] {
        let mut project = fixture()?;
        relation(&mut project,"transition","uml.transition",from,"final",None)?;
        assert_eq!(outcome(&project),invalid());
    }
    Ok(())
}

#[test]
fn canonical_relation_source_cannot_be_overridden_by_authored_source() -> Result {
    for (from,source) in [("final",json!({"$ref":"state"})),("state",json!({"$ref":"final"})),("state",Value::Null)] {
        let mut project = fixture()?;
        relation(&mut project,"transition","uml.transition",from,"state",Some(source))?;
        assert_eq!(outcome(&project),invalid());
    }
    let mut project = fixture()?;
    relation(&mut project,"transition","uml.transition","final","state",Some(json!({"href":"external.uml#Vertex"})))?;
    assert_eq!(outcome(&project),invalid());
    Ok(())
}

#[test]
fn outgoing_cache_and_non_transition_relations_do_not_create_or_erase_outgoing_membership() -> Result {
    let mut project = fixture()?;
    project.elements.iter_mut().find(|element| element.id=="final").ok_or("FinalState")?.properties.insert("outgoing".into(),json!([{"$ref":"cached"}]));
    relation(&mut project,"cached","uml.dependency","final","state",None)?;
    assert!(outcome(&project).is_empty());
    relation(&mut project,"transition","uml.transition","final","state",None)?;
    project.elements.iter_mut().find(|element| element.id=="final").ok_or("FinalState")?.properties.insert("outgoing".into(),json!([]));
    assert_eq!(outcome(&project),invalid());
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

#[test]
fn public_api_reports_the_same_no_outgoing_violation() -> Result {
    let mut project=fixture()?;
    relation(&mut project,"transition","uml.protocol_transition","final","state",None)?;
    let response:Value=serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":project}).to_string())?)?;
    assert!(response["diagnostics"].as_array().ok_or("diagnostics")?.iter().any(|issue|issue["subject_id"]=="final"&&issue["code"]==ERROR));
    Ok(())
}
