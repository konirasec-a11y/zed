use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_INSTANCE_DEFINING_FEATURE";
const INVALID: &str = "MDE_INSTANCE_DEFINING_FEATURE_INVALID";
const UNKNOWN: &str = "MDE_INSTANCE_DEFINING_FEATURE_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Slots","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"other","kind":"uml.class","name":"Other"},
        {"id":"feature","kind":"uml.property","name":"Feature"},
        {"id":"second-feature","kind":"uml.property","name":"SecondFeature"},
        {"id":"instance","kind":"uml.instance_specification","name":"Instance","properties":{"classifier":[{"$ref":"class"}]}},
        {"id":"slot-one","kind":"uml.slot","name":"","properties":{"definingFeature":{"$ref":"feature"},"value":[{"$ref":"value-one"}]}},
        {"id":"slot-two","kind":"uml.slot","name":"","properties":{"definingFeature":{"$ref":"second-feature"},"value":[{"$ref":"value-two"}]}},
        {"id":"value-one","kind":"uml.literal_integer","name":"","properties":{"value":1}},
        {"id":"value-two","kind":"uml.literal_integer","name":"","properties":{"value":2}}
    ],"relations":[
        {"id":"owns-class","kind":"owns","from":"root","to":"class"},
        {"id":"owns-other","kind":"owns","from":"root","to":"other"},
        {"id":"owns-instance","kind":"owns","from":"root","to":"instance"},
        {"id":"owns-feature","kind":"owns","from":"class","to":"feature"},
        {"id":"owns-second-feature","kind":"owns","from":"class","to":"second-feature"},
        {"id":"owns-slot-one","kind":"owns","from":"instance","to":"slot-one"},
        {"id":"owns-slot-two","kind":"owns","from":"instance","to":"slot-two"},
        {"id":"owns-value-one","kind":"owns","from":"slot-one","to":"value-one"},
        {"id":"owns-value-two","kind":"owns","from":"slot-two","to":"value-two"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("project")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
}
fn author(p: &mut Project, id: &str, key: &str, value: Value) -> Result {
    p.elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn issues(p: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":p}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "instance"
                && [ERROR, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| issue["code"] == *code)
        })
        .cloned()
        .collect())
}
fn expect(p: &Project, code: &str) -> Result {
    let found = issues(p)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    Ok(())
}

#[test]
fn slot_values_do_not_hide_missing_classifier_feature() -> Result {
    for rooted in [false, true] {
        let mut p = document(rooted)?;
        assert!(issues(&p)?.is_empty());
        author(&mut p, "instance", "classifier", json!([{"$ref":"other"}]))?;
        expect(&p, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&p, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn one_matching_classifier_is_enough_in_either_order() -> Result {
    for reverse in [false, true] {
        let mut p = document(false)?;
        let mut values = vec![json!({"$ref":"other"}), json!({"$ref":"class"})];
        if reverse {
            values.reverse();
        }
        author(&mut p, "instance", "classifier", Value::Array(values))?;
        assert!(issues(&p)?.is_empty());
    }
    Ok(())
}
#[test]
fn known_membership_wins_external_classifier_but_invalid_remains_invalid() -> Result {
    for reverse in [false, true] {
        let mut p = document(false)?;
        let mut values = vec![
            json!({"href":"external.uml#Class"}),
            json!({"$ref":"class"}),
        ];
        if reverse {
            values.reverse();
        }
        author(&mut p, "instance", "classifier", Value::Array(values))?;
        assert!(issues(&p)?.is_empty());
        author(&mut p, "slot-one", "definingFeature", json!(42))?;
        expect(&p, INVALID)?;
    }
    Ok(())
}
#[test]
fn external_feature_needs_identity_except_against_empty_feature_set() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "slot-one",
        "definingFeature",
        json!({"href":"external.uml#Feature"}),
    )?;
    expect(&p, UNKNOWN)?;
    author(&mut p, "instance", "classifier", json!([{"$ref":"other"}]))?;
    expect(&p, ERROR)?;
    author(&mut p, "instance", "classifier", json!([]))?;
    expect(&p, ERROR)?;
    Ok(())
}
#[test]
fn empty_slots_are_vacuous_even_when_classifier_is_external() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "instance",
        "classifier",
        json!([{"href":"external.uml#Class"}]),
    )?;
    p.relations.retain(|relation| relation.from != "instance");
    assert!(issues(&p)?.is_empty());
    author(
        &mut p,
        "instance",
        "classifier",
        json!([{"href":"external.uml#Class"},42]),
    )?;
    expect(&p, INVALID)?;
    Ok(())
}
#[test]
fn private_inherited_features_are_members_but_local_redefinitions_exclude_targets() -> Result {
    let mut p = document(false)?;
    author(&mut p, "feature", "visibility", json!("private"))?;
    author(&mut p, "instance", "classifier", json!([{"$ref":"other"}]))?;
    p.relations.push(serde_json::from_value(
        json!({"id":"inherit","kind":"uml.generalization","from":"other","to":"class"}),
    )?);
    assert!(issues(&p)?.is_empty());
    p.elements.push(serde_json::from_value(json!({"id":"replacement","kind":"uml.property","name":"Replacement","properties":{"redefinedProperty":[{"$ref":"feature"}]}}))?);
    p.relations.push(serde_json::from_value(
        json!({"id":"owns-replacement","kind":"owns","from":"other","to":"replacement"}),
    )?);
    expect(&p, ERROR)?;
    author(
        &mut p,
        "slot-one",
        "definingFeature",
        json!({"$ref":"replacement"}),
    )?;
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn known_false_slot_wins_unknown_sibling_and_malformed_wins_both() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "slot-one",
        "definingFeature",
        json!({"href":"external.uml#Feature"}),
    )?;
    p.elements.push(serde_json::from_value(
        json!({"id":"unrelated-feature","kind":"uml.property","name":"Unrelated"}),
    )?);
    p.relations.push(serde_json::from_value(
        json!({"id":"owns-unrelated","kind":"owns","from":"other","to":"unrelated-feature"}),
    )?);
    author(
        &mut p,
        "slot-two",
        "definingFeature",
        json!({"$ref":"unrelated-feature"}),
    )?;
    expect(&p, ERROR)?;
    author(&mut p, "slot-two", "definingFeature", Value::Null)?;
    expect(&p, INVALID)?;
    Ok(())
}
#[test]
fn malformed_classifier_hierarchy_beats_another_matching_classifier() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "instance",
        "classifier",
        json!([{"$ref":"class"},{"$ref":"other"}]),
    )?;
    p.relations.push(serde_json::from_value(
        json!({"id":"cycle","kind":"uml.generalization","from":"other","to":"other"}),
    )?);
    expect(&p, INVALID)?;
    Ok(())
}
#[test]
fn canonical_slots_cannot_be_omitted_by_authored_projection() -> Result {
    let mut p = document(false)?;
    author(&mut p, "instance", "slot", json!([]))?;
    expect(&p, INVALID)?;
    author(
        &mut p,
        "instance",
        "slot",
        json!([{"href":"external.uml#Slot"}]),
    )?;
    expect(&p, UNKNOWN)?;
    author(&mut p, "instance", "classifier", json!([]))?;
    expect(&p, ERROR)?;
    Ok(())
}
#[test]
fn repaired_snapshot_keeps_values_and_validation_is_snapshot_local() -> Result {
    let valid = document(true)?;
    let mut invalid = valid.clone();
    author(
        &mut invalid,
        "instance",
        "classifier",
        json!([{"$ref":"other"}]),
    )?;
    assert!(issues(&valid)?.is_empty());
    expect(&invalid, ERROR)?;
    assert!(issues(&valid)?.is_empty());
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "instance",
            "classifier",
            json!([{"$ref":"class"}]),
        )?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened.elements, valid.elements);
        assert_eq!(reopened.relations, valid.relations);
    }
    Ok(())
}

#[test]
fn direct_feature_membership_survives_external_ancestry_and_specification() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "class",
        "general",
        json!([{"href":"external.uml#Parent"}]),
    )?;
    author(
        &mut p,
        "instance",
        "specification",
        json!({"href":"external.uml#Specification"}),
    )?;
    assert!(issues(&p)?.is_empty());
    author(&mut p, "class", "general", json!(42))?;
    expect(&p, INVALID)?;
    Ok(())
}

#[test]
fn each_slot_may_match_a_different_classifier() -> Result {
    let mut p = document(false)?;
    p.relations
        .iter_mut()
        .find(|relation| relation.id == "owns-second-feature")
        .ok_or("feature ownership")?
        .from = "other".into();
    author(
        &mut p,
        "instance",
        "classifier",
        json!([{"$ref":"class"},{"$ref":"other"}]),
    )?;
    assert!(issues(&p)?.is_empty());
    for classifier in ["class", "other"] {
        author(
            &mut p,
            "instance",
            "classifier",
            json!([{"$ref":classifier}]),
        )?;
        expect(&p, ERROR)?;
    }
    Ok(())
}
