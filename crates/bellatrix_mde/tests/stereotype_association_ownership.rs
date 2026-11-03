use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STEREOTYPE_ASSOCIATION_END_OWNERSHIP";
const UNKNOWN: &str = "MDE_STEREOTYPE_ASSOCIATION_END_OWNERSHIP_UNVERIFIED";

fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Stereotype associations","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"profile","kind":"uml.profile","name":"Profile"},
            {"id":"stereotype","kind":"uml.stereotype","name":"Stereotype"},
            {"id":"other","kind":"uml.class","name":"Other"},
            {"id":"third","kind":"uml.class","name":"Third"},
            {"id":"association","kind":"uml.nary_association","name":"Association","properties":{"memberEnd":[{"$ref":"a"},{"$ref":"b"}],"ownedEnd":[{"$ref":"b"}]}},
            {"id":"a","kind":"uml.attribute","name":"toOther","properties":{"type":{"$ref":"other"}}},
            {"id":"b","kind":"uml.attribute","name":"toStereotype","properties":{"type":{"$ref":"stereotype"}}},
            {"id":"c","kind":"uml.attribute","name":"extra","properties":{"type":{"$ref":"stereotype"}}}
        ],"relations":[]});
    for (owner, child) in [("root","profile"),("profile","stereotype"),("root","other"),("root","third"),("root","association"),("stereotype","a"),("association","b"),("third","c")] {
        value["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) {
    model.elements.iter_mut().find(|element| element.id == id).expect("element").properties.insert(key.into(), value);
}
fn has(model: &Project, subject: &str, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(response["diagnostics"].as_array().ok_or("diagnostics")?.iter().any(|issue| issue["subject_id"] == subject && issue["code"] == code))
}

fn move_end(model: &mut Project, owner: &str) {
    model.relations.iter_mut().find(|relation| relation.id == "owns-b").expect("owner").from = owner.into();
    model.elements.iter_mut().find(|element| element.id == "association").expect("association").properties.remove("ownedEnd");
}
#[test]
fn given_non_stereotype_type_then_opposite_requires_association_ownership_after_roundtrip() -> Result {
    let mut model = document()?;
    assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    move_end(&mut model, "other");
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert!(has(&reopened, "stereotype", ERROR)?);
    }
    Ok(())
}
#[test]
fn given_stereotype_type_then_classifier_owned_opposite_is_permitted() -> Result {
    let mut model = document()?;
    move_end(&mut model, "other");
    author(&mut model, "a", "type", json!({"$ref":"stereotype"}));
    assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    Ok(())
}
#[test]
fn given_external_type_then_known_true_consequent_decides_and_false_remains_unknown() -> Result {
    let mut model = document()?;
    author(&mut model, "a", "type", json!({"href":"external.uml#Type"}));
    assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    move_end(&mut model, "other");
    assert!(has(&model, "stereotype", UNKNOWN)? && !has(&model, "stereotype", ERROR)?);
    for invalid in [json!(false),json!({"href":false}),json!({"$ref":"missing"})] {
        author(&mut model, "a", "type", invalid);
        assert!(has(&model, "stereotype", ERROR)?);
    }
    Ok(())
}
#[test]
fn given_external_member_then_type_exemption_decides_but_invalid_members_dominate() -> Result {
    let mut model = document()?;
    author(&mut model, "association", "memberEnd", json!([{"$ref":"a"},{"href":"external.uml#End"}]));
    assert!(has(&model, "stereotype", UNKNOWN)?);
    author(&mut model, "a", "type", json!({"$ref":"stereotype"}));
    assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    for members in [json!([{"$ref":"a"},false]),json!([false,{"$ref":"a"}])] {
        author(&mut model, "association", "memberEnd", members);
        assert!(has(&model, "stereotype", ERROR)?);
    }
    Ok(())
}
#[test]
fn given_opposite_cache_then_it_cannot_replace_member_end_identity() -> Result {
    let mut model = document()?;
    for value in [json!({"$ref":"c"}),json!(null),json!(false)] {
        author(&mut model, "a", "opposite", value.clone());
        assert!(has(&model, "stereotype", ERROR)?, "opposite={value}: {}", validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?);
    }
    author(&mut model, "a", "opposite", json!({"$ref":"b"}));
    assert!(!has(&model, "stereotype", ERROR)?);
    Ok(())
}
#[test]
fn given_unknown_neighbor_then_known_bad_owner_wins_in_both_orders() -> Result {
    for reverse in [false,true] {
        let mut model = document()?;
        move_end(&mut model,"other");
        model.elements.push(serde_json::from_value(json!({"id":"unknown","kind":"uml.unknown_member","name":"Unknown"}))?);
        model.relations.push(serde_json::from_value(json!({"id":"owns-unknown","kind":"owns","from":"stereotype","to":"unknown"}))?);
        if reverse { model.relations.reverse(); }
        assert!(has(&model,"stereotype",ERROR)? && !has(&model,"stereotype",UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn given_extension_then_association_end_ownership_is_exempt() -> Result {
    let mut model = document()?;
    model.elements.iter_mut().find(|element|element.id=="association").expect("association").kind="uml.extension".into();
    move_end(&mut model,"other");
    assert!(!has(&model,"stereotype",ERROR)? && !has(&model,"stereotype",UNKNOWN)?);
    Ok(())
}
#[test]
fn given_reparent_opposite_then_command_rejects_atomically() -> Result {
    let mut model = document()?;
    model.elements.iter_mut().find(|element|element.id=="association").expect("association").properties.remove("ownedEnd");
    let before = encode(&model,Format::Json)?;
    let error = apply(&model,&Command::SetOwner { element_id:"b".into(), owner_id:Some("other".into()) },1).expect_err("bad owner");
    assert!(error.to_string().contains(ERROR), "{error}");
    assert_eq!(before,encode(&model,Format::Json)?);
    Ok(())
}

#[test]
fn given_absent_or_null_type_then_non_stereotype_branch_checks_owner() -> Result {
    for explicit in [false,true] {
        let mut model = document()?;
        model.elements.iter_mut().find(|element|element.id=="a").expect("attribute").properties.remove("type");
        if explicit { author(&mut model,"a","type",Value::Null); }
        assert!(!has(&model,"stereotype",ERROR)? && !has(&model,"stereotype",UNKNOWN)?);
        move_end(&mut model,"other");
        assert!(has(&model,"stereotype",ERROR)?);
    }
    Ok(())
}

#[test]
fn given_external_opposite_cache_then_known_bad_owner_remains_a_violation_in_both_orders() -> Result {
    for reverse in [false,true] {
        let mut model = document()?;
        author(&mut model,"a","opposite",json!({"href":"external.uml#Opposite"}));
        if reverse {
            author(&mut model,"association","memberEnd",json!([{"$ref":"b"},{"$ref":"a"}]));
            model.relations.reverse();
        }
        assert!(has(&model,"stereotype",UNKNOWN)?);
        move_end(&mut model,"other");
        assert!(has(&model,"stereotype",ERROR)? && !has(&model,"stereotype",UNKNOWN)?);
        author(&mut model,"a","opposite",json!({"href":false}));
        author(&mut model,"a","type",json!({"href":"external.uml#Type"}));
        assert!(has(&model,"stereotype",ERROR)?);
    }
    Ok(())
}
