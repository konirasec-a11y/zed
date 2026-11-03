use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PROFILE_METACLASS_REFERENCE_SPECIALIZATION";
const UNKNOWN: &str = "MDE_PROFILE_METACLASS_REFERENCE_SPECIALIZATION_UNVERIFIED";
fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Profile metaclass references","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"profile","kind":"uml.profile","name":"Profile","properties":{"metaclassReference":[{"$ref":"import"}]}},
        {"id":"base","kind":"uml.package","name":"Base"},
        {"id":"imported","kind":"uml.class","name":"Imported"},
        {"id":"local","kind":"uml.class","name":"Local"},
        {"id":"middle","kind":"uml.class","name":"Middle"}
    ],"relations":[{"id":"import","kind":"uml.element_import","from":"profile","to":"imported"}]});
    for (owner, child) in [("root","profile"),("root","base"),("base","imported"),("base","middle"),("profile","local")] {
        value["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) {
    model.elements.iter_mut().find(|element|element.id==id).expect("element").properties.insert(key.into(),value);
}
fn generalize(model: &mut Project, from: &str, to: &str) -> Result {
    model.relations.push(serde_json::from_value(json!({"id":format!("general-{from}-{to}"),"kind":"uml.generalization","from":from,"to":to}))?);
    Ok(())
}
fn has(model: &Project, code: &str) -> Result<bool> {
    let response:Value=serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(response["diagnostics"].as_array().ok_or("diagnostics")?.iter().any(|issue|issue["subject_id"]=="profile" && issue["code"]==code))
}
#[test]
fn given_imported_classifier_then_both_generalization_directions_are_rejected_after_roundtrip() -> Result {
    for reverse in [false,true] {
        let mut model=document()?;
        assert!(!has(&model,ERROR)? && !has(&model,UNKNOWN)?);
        let (from,to)=if reverse {("imported","local")} else {("local","imported")};
        generalize(&mut model,from,to)?;
        for format in [Format::Json,Format::Protobuf] {
            let reopened=decode(&encode(&model,format)?,format)?;
            assert!(has(&reopened,ERROR)? && !has(&reopened,UNKNOWN)?);
        }
    }
    Ok(())
}
#[test]
fn given_transitive_path_then_both_conjuncts_use_all_parents() -> Result {
    for reverse in [false,true] {
        let mut model=document()?;
        let (from,to)=if reverse {("imported","local")} else {("local","imported")};
        generalize(&mut model,from,"middle")?;
        generalize(&mut model,"middle",to)?;
        assert!(has(&model,ERROR)?);
    }
    Ok(())
}
#[test]
fn given_unknown_import_neighbor_then_known_violation_wins_in_both_orders() -> Result {
    for reverse in [false,true] {
        let mut model=document()?;
        let mut refs=vec![json!({"$ref":"import"}),json!({"href":"external.uml#Import"})];
        if reverse {refs.reverse();model.relations.reverse();}
        author(&mut model,"profile","metaclassReference",json!(refs));
        assert!(has(&model,UNKNOWN)? && !has(&model,ERROR)?);
        generalize(&mut model,"local","imported")?;
        assert!(has(&model,ERROR)? && !has(&model,UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn given_unresolved_ancestry_or_package_members_then_known_path_still_proves_violation() -> Result {
    let mut model=document()?;
    author(&mut model,"local","general",json!([{"href":"external.uml#General"}]));
    author(&mut model,"profile","packagedElement",json!([{"$ref":"local"},{"href":"external.uml#Packaged"}]));
    assert!(has(&model,UNKNOWN)? && !has(&model,ERROR)?);
    generalize(&mut model,"local","imported")?;
    assert!(has(&model,ERROR)? && !has(&model,UNKNOWN)?);
    Ok(())
}
#[test]
fn given_malformed_reference_then_invalid_dominates_unknown() -> Result {
    for invalid in [json!(false),json!({"href":false}),json!({"$ref":"missing"}),json!({"$ref":"local"})] {
        let mut model=document()?;
        author(&mut model,"profile","metaclassReference",json!([{"href":"external.uml#Import"},invalid]));
        assert!(has(&model,ERROR)?);
    }
    Ok(())
}
#[test]
fn given_import_role_absent_then_ordinary_import_is_not_invented_as_metaclass_reference() -> Result {
    let mut model=document()?;
    model.elements.iter_mut().find(|element|element.id=="profile").expect("profile").properties.remove("metaclassReference");
    generalize(&mut model,"local","imported")?;
    assert!(has(&model,UNKNOWN)? && !has(&model,ERROR)?);
    author(&mut model,"profile","metaclassReference",json!([]));
    assert!(!has(&model,UNKNOWN)? && !has(&model,ERROR)?);
    Ok(())
}
#[test]
fn given_stored_import_endpoint_then_contradiction_is_invalid_and_external_does_not_hide_path() -> Result {
    for value in [json!({"$ref":"middle"}),json!(false)] {
        let mut model=document()?;
        model.relations.iter_mut().find(|relation|relation.id=="import").expect("import").properties.insert("importedElement".into(),value);
        assert!(has(&model,ERROR)?);
    }
    let mut model=document()?;
    model.relations.iter_mut().find(|relation|relation.id=="import").expect("import").properties.insert("importedElement".into(),json!({"href":"external.uml#Classifier"}));
    generalize(&mut model,"imported","local")?;
    assert!(has(&model,ERROR)? && !has(&model,UNKNOWN)?);
    Ok(())
}
#[test]
fn given_classifier_outside_profile_when_reparented_into_profile_then_rejection_is_atomic() -> Result {
    let mut model=document()?;
    model.relations.iter_mut().find(|relation|relation.id=="owns-local").expect("owner").from="base".into();
    generalize(&mut model,"local","imported")?;
    let before=encode(&model,Format::Json)?;
    let error=apply(&model,&Command::SetOwner {element_id:"local".into(),owner_id:Some("profile".into())},1).expect_err("forbidden specialization");
    assert!(error.to_string().contains(ERROR),"{error}");
    assert_eq!(before,encode(&model,Format::Json)?);
    Ok(())
}

#[test]
fn given_unadapted_import_object_then_no_role_is_inferred_from_ownership() -> Result {
    let mut model=document()?;
    model.relations.retain(|relation|relation.id!="import");
    model.elements.push(serde_json::from_value(json!({"id":"import-object","kind":"uml.element_import","name":"Import","properties":{"importedElement":{"$ref":"imported"}}}))?);
    model.relations.push(serde_json::from_value(json!({"id":"owns-import-object","kind":"owns","from":"profile","to":"import-object"}))?);
    model.elements.iter_mut().find(|element|element.id=="profile").expect("profile").properties.remove("metaclassReference");
    assert!(has(&model,UNKNOWN)? && !has(&model,ERROR)?);
    author(&mut model,"profile","metaclassReference",json!([{"$ref":"import-object"}]));
    assert!(has(&model,UNKNOWN)? && !has(&model,ERROR)?);
    Ok(())
}
