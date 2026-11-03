use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STEREOTYPE_BINARY_ASSOCIATIONS";
const UNKNOWN: &str = "MDE_STEREOTYPE_BINARY_ASSOCIATIONS_UNVERIFIED";

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
fn nary(model: &mut Project) {
    author(model, "association", "memberEnd", json!([{"$ref":"a"},{"$ref":"b"},{"$ref":"c"}]));
}

#[test]
fn given_direct_attribute_in_association_then_only_binary_cardinality_passes_after_roundtrip() -> Result {
    let mut model = document()?;
    assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    nary(&mut model);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert!(has(&reopened, "stereotype", ERROR)? && !has(&reopened, "stereotype", UNKNOWN)?);
    }
    Ok(())
}

#[test]
fn given_unknown_neighbor_then_known_nary_association_remains_invalid_in_both_orders() -> Result {
    for reverse in [false, true] {
        let mut model = document()?;
        model.elements.push(serde_json::from_value(json!({"id":"unknown","kind":"uml.attribute","name":"Unknown","properties":{"type":{"$ref":"other"},"association":{"href":"external.uml#Association"}}}))?);
        model.relations.push(serde_json::from_value(json!({"id":"owns-unknown","kind":"owns","from":"stereotype","to":"unknown"}))?);
        if reverse { model.relations.reverse(); }
        assert!(has(&model, "stereotype", UNKNOWN)? && !has(&model, "stereotype", ERROR)?);
        nary(&mut model);
        assert!(has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    }
    Ok(())
}

#[test]
fn given_partial_members_then_known_cardinality_wins_but_malformed_members_still_fail() -> Result {
    let mut model = document()?;
    for members in [json!([{"$ref":"a"},{"href":"external.uml#B"}]),json!([{"href":"external.uml#B"},{"$ref":"a"}])] {
        author(&mut model, "association", "memberEnd", members);
        assert!(!has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    }
    author(&mut model, "association", "memberEnd", json!([{"$ref":"a"},{"href":"external.uml#B"},{"href":"external.uml#C"}]));
    assert!(has(&model, "stereotype", ERROR)?);
    for members in [json!([{"$ref":"a"},false]),json!([false,{"$ref":"a"}]),json!([{"$ref":"a"},{"$ref":"missing"}]),json!({"$ref":"a"})] {
        author(&mut model, "association", "memberEnd", members);
        assert!(has(&model, "stereotype", ERROR)?);
    }
    Ok(())
}

#[test]
fn given_stored_association_inverse_then_it_cannot_override_canonical_membership() -> Result {
    let mut model = document()?;
    model.elements.push(serde_json::from_value(json!({"id":"other-association","kind":"uml.nary_association","name":"Other","properties":{"memberEnd":[{"$ref":"b"},{"$ref":"c"}]}}))?);
    for inverse in [json!({"$ref":"other-association"}),Value::Null,json!(false),json!({"href":false}),json!({"$ref":"missing"})] {
        author(&mut model, "a", "association", inverse);
        assert!(has(&model, "stereotype", ERROR)?);
    }
    author(&mut model, "a", "association", json!({"href":"external.uml#Association"}));
    assert!(has(&model, "stereotype", UNKNOWN)?);
    nary(&mut model);
    assert!(has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    Ok(())
}

#[test]
fn given_unmaterialized_members_then_inverse_or_visual_endpoints_do_not_prove_membership() -> Result {
    let mut model = document()?;
    model.elements.iter_mut().find(|element| element.id == "association").ok_or("association")?.properties.remove("memberEnd");
    assert!(has(&model, "stereotype", UNKNOWN)? && !has(&model, "stereotype", ERROR)?);
    author(&mut model, "a", "association", json!({"$ref":"association"}));
    assert!(has(&model, "stereotype", UNKNOWN)?);
    author(&mut model, "association", "memberEnd", json!([{"$ref":"b"},{"$ref":"c"}]));
    assert!(has(&model, "stereotype", ERROR)?, "complete memberEnd disproves stored inverse");
    Ok(())
}

#[test]
fn given_inherited_attribute_then_only_direct_stereotype_owner_receives_obligation() -> Result {
    let mut model = document()?;
    model.elements.push(serde_json::from_value(json!({"id":"child","kind":"uml.stereotype","name":"Child"}))?);
    model.relations.push(serde_json::from_value(json!({"id":"owns-child","kind":"owns","from":"profile","to":"child"}))?);
    model.relations.push(serde_json::from_value(json!({"id":"general","kind":"uml.generalization","from":"child","to":"stereotype"}))?);
    nary(&mut model);
    assert!(has(&model, "stereotype", ERROR)?);
    assert!(!has(&model, "child", ERROR)? && !has(&model, "child", UNKNOWN)?);
    Ok(())
}

#[test]
fn given_unknown_attribute_collection_then_known_violation_wins_without_ignoring_invalid_ownership() -> Result {
    let mut model = document()?;
    author(&mut model, "stereotype", "ownedAttribute", json!([{"$ref":"a"},{"href":"external.uml#Attribute"}]));
    assert!(has(&model, "stereotype", UNKNOWN)?);
    nary(&mut model);
    assert!(has(&model, "stereotype", ERROR)? && !has(&model, "stereotype", UNKNOWN)?);
    for malformed in [json!([false,{"href":"external.uml#Attribute"}]),json!([{"href":"external.uml#Attribute"},false])] {
        author(&mut model, "stereotype", "ownedAttribute", malformed);
        assert!(has(&model, "stereotype", ERROR)?);
    }
    Ok(())
}

#[test]
fn when_nary_end_moves_to_stereotype_then_existing_association_gate_also_rejects_atomically() -> Result {
    let mut model = document()?;
    nary(&mut model);
    for id in ["a","c"] {
        model.relations.iter_mut().find(|relation| relation.id == format!("owns-{id}")).ok_or("ownership")?.from = "association".into();
    }
    author(&mut model, "association", "ownedEnd", json!([{"$ref":"a"},{"$ref":"b"},{"$ref":"c"}]));
    let before = model.clone();
    assert!(apply(&model, &Command::SetOwner {element_id:"a".into(),owner_id:Some("stereotype".into())}, 1).is_err());
    assert_eq!(model, before);
    Ok(())
}
