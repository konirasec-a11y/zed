use bellatrix_mde::{Format, Project, StructuralSeverity, commands::{Command, apply}, decode, encode, uml::root::Model, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PSEUDOSTATE_INITIAL_VERTEX";
const UNKNOWN: &str = "MDE_PSEUDOSTATE_INITIAL_VERTEX_UNVERIFIED";
fn fixture(kind: Option<Value>) -> Result<Project> {
    let mut project:Project=serde_json::from_value(json!({"id":"model","name":"Initial vertex","elements":[
        {"id":"point","kind":"uml.pseudostate","name":"Initial"},
        {"id":"other","kind":"uml.state","name":"Other"},
        {"id":"unknown","kind":"extension.unknown","name":"Unknown"},
        {"id":"wrong","kind":"uml.class","name":"Wrong"}
    ]}))?;
    if let Some(kind)=kind { project.elements.iter_mut().find(|element|element.id=="point").ok_or("Pseudostate")?.properties.insert("kind".into(),kind); }
    Ok(project)
}
fn edge(project:&mut Project,id:&str,kind:&str,source:&str)->Result {
    project.relations.push(serde_json::from_value(json!({"id":id,"kind":kind,"from":source,"to":"other"}))?);
    Ok(())
}
fn object(project:&mut Project,id:&str,source:Option<Value>)->Result {
    let properties=source.map(|source|json!({"source":source})).unwrap_or(json!({}));
    project.elements.push(serde_json::from_value(json!({"id":id,"kind":"uml.transition","name":id,"properties":properties}))?);
    Ok(())
}
fn outcome(project:&Project)->Vec<(StructuralSeverity,&'static str)> {
    Model::from_project(project).validate().into_iter().filter(|issue|issue.subject_id=="point"&&matches!(issue.code,ERROR|UNKNOWN))
        .map(|issue|(issue.severity,issue.code)).collect()
}
fn invalid()->Vec<(StructuralSeverity,&'static str)>{vec![(StructuralSeverity::Error,ERROR)]}
fn unknown()->Vec<(StructuralSeverity,&'static str)>{vec![(StructuralSeverity::Warning,UNKNOWN)]}

#[test]
fn initial_default_and_explicit_kind_allow_zero_or_one_but_not_two_outgoing_transitions()->Result {
    for kind in [None,Some(json!("initial"))] {
        for count in 0..=3 {
            let mut project=fixture(kind.clone())?;
            for index in 0..count { edge(&mut project,&format!("edge-{index}"),if index%2==0{"uml.transition"}else{"uml.protocol_transition"},"point")?; }
            for format in [Format::Json,Format::Protobuf] {
                assert_eq!(outcome(&decode(&encode(&project,format)?,format)?),if count>1{invalid()}else{vec![]});
            }
        }
    }
    Ok(())
}

#[test]
fn finite_unknown_candidates_use_cardinality_bounds_not_blanket_unknown()->Result {
    for known in 0..=2 {
        for possible in 0..=2 {
            for missing in [false,true] {
                let mut project=fixture(None)?;
                for index in 0..known {edge(&mut project,&format!("known-{index}"),"uml.transition","point")?;}
                for index in 0..possible {object(&mut project,&format!("possible-{index}"),if missing{None}else{Some(json!({"href":"external.uml#Vertex"}))})?;}
                let expected=if known>1{invalid()}else if known+possible>1{unknown()}else{vec![]};
                assert_eq!(outcome(&project),expected,"known={known}, possible={possible}, missing={missing}");
            }
        }
    }
    Ok(())
}

#[test]
fn other_pseudostate_kinds_are_exempt_without_hiding_invalid_kind_values()->Result {
    for kind in ["choice","junction","fork","join","deepHistory","shallowHistory","entryPoint","exitPoint","terminate"] {
        let mut project=fixture(Some(json!(kind)))?;
        edge(&mut project,"one","uml.transition","point")?;
        edge(&mut project,"two","uml.transition","point")?;
        object(&mut project,"external",Some(json!({"href":"external.uml#Vertex"})))?;
        assert!(outcome(&project).is_empty(),"{kind}");
    }
    for kind in [Value::Null,json!(42),json!([]),json!("futureKind")] {
        assert_eq!(outcome(&fixture(Some(kind))?),invalid());
    }
    Ok(())
}

#[test]
fn invalid_sources_dominate_even_when_the_upper_count_would_be_small()->Result {
    for value in [Value::Null,json!(42),json!([]),json!({"$ref":"missing"}),json!({"$ref":"wrong"}),json!({"href":""})] {
        let mut project=fixture(None)?;
        object(&mut project,"bad",Some(value))?;
        assert_eq!(outcome(&project),invalid());
        object(&mut project,"external",Some(json!({"href":"external.uml#Vertex"})))?;
        assert_eq!(outcome(&project),invalid());
    }
    Ok(())
}

#[test]
fn known_other_source_identities_do_not_consume_uncertain_capacity()->Result {
    let mut project=fixture(None)?;
    edge(&mut project,"outgoing","uml.transition","point")?;
    object(&mut project,"other-transition",Some(json!({"$ref":"unknown"})))?;
    edge(&mut project,"incoming","uml.transition","other")?;
    project.relations.last_mut().ok_or("Transition")?.to="point".into();
    assert!(outcome(&project).is_empty());
    assert!(Model::from_project(&project).validate().iter().any(|issue|issue.subject_id=="other-transition"&&issue.code=="MDE_PROPERTY_CONTRACT_UNVERIFIED"));
    Ok(())
}

#[test]
fn derived_outgoing_cache_cannot_invent_or_hide_a_count()->Result {
    let mut project=fixture(None)?;
    project.elements.iter_mut().find(|element|element.id=="point").ok_or("Pseudostate")?.properties.insert("outgoing".into(),json!([{"$ref":"fake1"},{"$ref":"fake2"}]));
    assert!(outcome(&project).is_empty());
    edge(&mut project,"one","uml.transition","point")?;
    object(&mut project,"two",Some(json!({"$ref":"point"})))?;
    project.elements.iter_mut().find(|element|element.id=="point").ok_or("Pseudostate")?.properties.insert("outgoing".into(),json!([]));
    assert_eq!(outcome(&project),invalid());
    Ok(())
}

#[test]
fn changing_kind_rejects_new_violation_atomically_and_delete_repairs_import()->Result {
    let mut project=fixture(Some(json!("choice")))?;
    edge(&mut project,"one","uml.transition","point")?;
    edge(&mut project,"two","uml.transition","point")?;
    let before=project.clone();
    let failure=apply(&project,&Command::SetProperty{element_id:"point".into(),path:"properties.kind".into(),value:json!("initial")},0)
        .expect_err("switching to initial must enforce the cardinality");
    assert!(failure.starts_with(ERROR),"{failure}");
    assert_eq!(project,before);
    project.elements.iter_mut().find(|element|element.id=="point").ok_or("Pseudostate")?.properties.insert("kind".into(),json!("initial"));
    let repaired=apply(&project,&Command::DeleteElement{element_id:"two".into()},0)?;
    assert!(outcome(&repaired).is_empty());
    assert_eq!(outcome(&project),invalid());
    Ok(())
}

#[test]
fn public_api_reports_the_same_known_count_violation()->Result {
    let mut project=fixture(None)?;
    edge(&mut project,"one","uml.transition","point")?;
    edge(&mut project,"two","uml.protocol_transition","point")?;
    let response:Value=serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":project}).to_string())?)?;
    assert!(response["diagnostics"].as_array().ok_or("diagnostics")?.iter().any(|issue|issue["subject_id"]=="point"&&issue["code"]==ERROR));
    Ok(())
}
