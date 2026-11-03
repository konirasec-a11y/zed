use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_INSTANCE_STRUCTURAL_FEATURE";
const INVALID: &str = "MDE_INSTANCE_STRUCTURAL_FEATURE_INVALID";
const UNKNOWN: &str = "MDE_INSTANCE_STRUCTURAL_FEATURE_UNVERIFIED";
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
fn duplicate(p: &mut Project) -> Result {
    author(p, "slot-two", "definingFeature", json!({"$ref":"feature"}))
}
#[test]
fn duplicate_slottable_feature_is_not_hidden_by_slot_values() -> Result {
    for rooted in [false, true] {
        let mut p = document(rooted)?;
        assert!(issues(&p)?.is_empty());
        duplicate(&mut p)?;
        expect(&p, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&p, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn duplicate_feature_outside_all_slottable_features_is_not_this_invariant() -> Result {
    let mut p = document(false)?;
    duplicate(&mut p)?;
    author(&mut p, "instance", "classifier", json!([{"$ref":"other"}]))?;
    assert!(issues(&p)?.is_empty());
    author(&mut p, "instance", "classifier", json!([]))?;
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn inherited_private_attribute_is_a_slottable_feature() -> Result {
    let mut p = document(false)?;
    duplicate(&mut p)?;
    author(&mut p, "feature", "visibility", json!("private"))?;
    author(&mut p, "instance", "classifier", json!([{"$ref":"other"}]))?;
    p.relations.push(serde_json::from_value(
        json!({"id":"inherit","kind":"uml.generalization","from":"other","to":"class"}),
    )?);
    expect(&p, ERROR)?;
    Ok(())
}
#[test]
fn mixed_external_classifiers_preserve_known_violation_in_both_orders() -> Result {
    for reverse in [false, true] {
        let mut p = document(false)?;
        duplicate(&mut p)?;
        let mut values = vec![
            json!({"$ref":"class"}),
            json!({"href":"external.uml#Class"}),
        ];
        if reverse {
            values.reverse();
        }
        author(&mut p, "instance", "classifier", Value::Array(values))?;
        expect(&p, ERROR)?;
    }
    Ok(())
}
#[test]
fn external_defining_feature_is_unknown_but_malformed_is_invalid() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "slot-two",
        "definingFeature",
        json!({"href":"external.uml#Feature"}),
    )?;
    expect(&p, UNKNOWN)?;
    for value in [
        json!(42),
        Value::Null,
        json!({"href":42}),
        json!({"$ref":"missing"}),
        json!({"$ref":"class"}),
    ] {
        author(&mut p, "slot-one", "definingFeature", value)?;
        expect(&p, INVALID)?;
    }
    Ok(())
}
#[test]
fn singleton_slot_and_empty_classifier_collection_are_vacuous() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "slot-one",
        "definingFeature",
        json!({"href":"external.uml#Feature"}),
    )?;
    author(&mut p, "instance", "classifier", json!([]))?;
    assert!(issues(&p)?.is_empty());
    author(
        &mut p,
        "instance",
        "classifier",
        json!([{"href":"external.uml#Class"}]),
    )?;
    p.relations
        .iter_mut()
        .find(|relation| relation.id == "owns-slot-two")
        .ok_or("owner")?
        .from = "other".into();
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn canonical_slot_duplicates_survive_unknown_stored_slot_projection() -> Result {
    let mut p = document(false)?;
    duplicate(&mut p)?;
    author(
        &mut p,
        "instance",
        "slot",
        json!([{"href":"external.uml#Slot"}]),
    )?;
    expect(&p, ERROR)?;
    author(&mut p, "instance", "slot", json!([]))?;
    expect(&p, INVALID)?;
    Ok(())
}
#[test]
fn direct_feature_evidence_survives_unresolved_ancestry_but_not_cycles() -> Result {
    let mut p = document(false)?;
    duplicate(&mut p)?;
    author(
        &mut p,
        "class",
        "general",
        json!([{"href":"external.uml#Parent"}]),
    )?;
    expect(&p, ERROR)?;
    p.relations.push(serde_json::from_value(
        json!({"id":"cycle","kind":"uml.generalization","from":"class","to":"class"}),
    )?);
    expect(&p, INVALID)?;
    Ok(())
}
#[test]
fn malformed_classifiers_cannot_be_hidden_by_unknown_slot_or_singleton() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "slot-one",
        "definingFeature",
        json!({"href":"external.uml#Feature"}),
    )?;
    for value in [
        json!([{"href":"external.uml#Class"},42]),
        json!([{"$ref":"class"},{"$ref":"class"}]),
        Value::Null,
    ] {
        author(&mut p, "instance", "classifier", value)?;
        expect(&p, INVALID)?;
    }
    Ok(())
}

#[test]
fn feature_count_is_per_owning_instance_not_global() -> Result {
    let mut p = document(false)?;
    duplicate(&mut p)?;
    p.elements.push(serde_json::from_value(json!({"id":"second-instance","kind":"uml.instance_specification","name":"Second","properties":{"classifier":[{"$ref":"class"}]}}))?);
    p.relations.push(serde_json::from_value(
        json!({"id":"owns-second-instance","kind":"owns","from":"root","to":"second-instance"}),
    )?);
    p.relations
        .iter_mut()
        .find(|relation| relation.id == "owns-slot-two")
        .ok_or("owner")?
        .from = "second-instance".into();
    assert!(issues(&p)?.is_empty());
    Ok(())
}

#[test]
fn duplicate_snapshot_repair_preserves_values_and_does_not_cache_results() -> Result {
    let valid = document(true)?;
    let mut invalid = valid.clone();
    duplicate(&mut invalid)?;
    assert!(issues(&valid)?.is_empty());
    expect(&invalid, ERROR)?;
    assert!(issues(&valid)?.is_empty());
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "slot-two",
            "definingFeature",
            json!({"$ref":"second-feature"}),
        )?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened.relations, valid.relations);
        assert_eq!(reopened.elements, valid.elements);
    }
    Ok(())
}

#[test]
fn invalid_defining_feature_dominates_known_duplicate_in_either_slot_order() -> Result {
    for reverse in [false, true] {
        let mut p = document(false)?;
        duplicate(&mut p)?;
        p.elements.push(serde_json::from_value(json!({"id":"third-slot","kind":"uml.slot","name":"","properties":{"definingFeature":42}}))?);
        p.relations.push(serde_json::from_value(
            json!({"id":"owns-third-slot","kind":"owns","from":"instance","to":"third-slot"}),
        )?);
        if reverse {
            p.elements.reverse();
            p.relations.reverse();
        }
        expect(&p, INVALID)?;
    }
    Ok(())
}

#[test]
fn complete_distinct_feature_counts_satisfy_rule_with_external_classifier() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "instance",
        "classifier",
        json!([{"href":"external.uml#Class"}]),
    )?;
    assert!(issues(&p)?.is_empty());
    duplicate(&mut p)?;
    expect(&p, UNKNOWN)?;
    author(
        &mut p,
        "slot-two",
        "definingFeature",
        json!({"$ref":"second-feature"}),
    )?;
    author(
        &mut p,
        "instance",
        "classifier",
        json!([{"href":"external.uml#Class"},42]),
    )?;
    expect(&p, INVALID)?;
    Ok(())
}
