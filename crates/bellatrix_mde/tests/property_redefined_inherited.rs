use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PROPERTY_REDEFINED_PROPERTY_INHERITED";
const UNKNOWN: &str = "MDE_PROPERTY_REDEFINED_PROPERTY_INHERITED_UNVERIFIED";
fn document(diamond: bool) -> Result<Project> {
    let mut value = json!({
        "id":"project", "name":"Redefinitions", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"base","kind":"uml.class","name":"Base"},
            {"id":"middle","kind":"uml.class","name":"Middle"},
            {"id":"leaf","kind":"uml.class","name":"Leaf"},
            {"id":"other","kind":"uml.class","name":"Other"},
            {"id":"type","kind":"uml.class","name":"Type"},
            {"id":"original","kind":"uml.attribute","name":"original", "properties":{"type":{"$ref":"type"}}},
            {"id":"replacement","kind":"uml.attribute","name":"replacement", "properties":{"type":{"$ref":"type"},"redefinedProperty":[{"$ref":"original"}]}},
            {"id":"last","kind":"uml.attribute","name":"last", "properties":{"type":{"$ref":"type"}}}
        ],
        "relations":[
            {"id":"middle-base","kind":"uml.generalization","from":"middle","to":"base"},
            {"id":"leaf-middle","kind":"uml.generalization","from":"leaf","to":"middle"}
        ]
    });
    for (owner, child) in [
        ("root", "base"),
        ("root", "middle"),
        ("root", "leaf"),
        ("root", "other"),
        ("root", "type"),
        ("base", "original"),
        ("middle", "replacement"),
        ("leaf", "last"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({
                "id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child
            }));
    }
    if diamond {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .extend([
                json!({"id":"other-base","kind":"uml.generalization","from":"other","to":"base"}),
                json!({"id":"leaf-other","kind":"uml.generalization","from":"leaf","to":"other"}),
            ]);
    }
    Ok(decode(&serde_json::to_vec(&value)?, Format::Json)?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn has(model: &Project, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .any(|d| d["subject_id"] == "replacement" && d["code"] == code))
}
fn pass(model: &Project) -> Result {
    assert!(!has(model, ERROR)?);
    assert!(!has(model, UNKNOWN)?);
    Ok(())
}
#[test]
fn direct_transitive_and_diamond_features_survive_roundtrip() -> Result {
    for diamond in [false, true] {
        let mut model = document(diamond)?;
        pass(&model)?;
        model
            .relations
            .iter_mut()
            .find(|r| r.id == "owns-replacement")
            .ok_or("owner")?
            .from = "leaf".into();
        pass(&model)?;
        for format in [Format::Json, Format::Protobuf] {
            pass(&decode(&encode(&model, format)?, format)?)?;
        }
    }
    Ok(())
}
#[test]
fn unrelated_or_same_context_properties_are_not_ancestral_features() -> Result {
    for owner in ["middle", "other"] {
        let mut model = document(false)?;
        model
            .relations
            .iter_mut()
            .find(|r| r.id == "owns-original")
            .ok_or("owner")?
            .from = owner.into();
        assert!(has(&model, ERROR)?);
    }
    Ok(())
}
#[test]
fn absent_or_empty_redefinitions_are_vacuous() -> Result {
    let mut model = document(false)?;
    model
        .elements
        .iter_mut()
        .find(|e| e.id == "replacement")
        .ok_or("element")?
        .properties
        .remove("redefinedProperty");
    pass(&model)?;
    author(&mut model, "replacement", "redefinedProperty", json!([]))?;
    pass(&model)?;
    Ok(())
}
#[test]
fn partial_ancestry_keeps_known_feature_witness_and_unknown_absence() -> Result {
    let mut model = document(false)?;
    author(
        &mut model,
        "middle",
        "general",
        json!([{"$ref":"base"},{"href":"external#Base"}]),
    )?;
    pass(&model)?;
    model
        .relations
        .iter_mut()
        .find(|r| r.id == "owns-original")
        .ok_or("owner")?
        .from = "other".into();
    assert!(has(&model, UNKNOWN)?);
    assert!(!has(&model, ERROR)?);
    Ok(())
}
#[test]
fn unknown_target_and_future_context_are_not_false_absence() -> Result {
    let mut model = document(false)?;
    author(
        &mut model,
        "replacement",
        "redefinedProperty",
        json!([{"href":"external#Property"}]),
    )?;
    assert!(has(&model, UNKNOWN)?);
    assert!(!has(&model, ERROR)?);
    let mut model = document(false)?;
    model
        .elements
        .iter_mut()
        .find(|e| e.id == "middle")
        .ok_or("middle")?
        .kind = "uml.future_classifier".into();
    assert!(has(&model, UNKNOWN)?);
    assert!(!has(&model, ERROR)?);
    Ok(())
}
#[test]
fn malformed_references_context_and_cycles_remain_invalid() -> Result {
    for (key, value) in [
        (
            "redefinedProperty",
            json!([{"$ref":"original"},{"$ref":"original"}]),
        ),
        (
            "redefinedProperty",
            json!([{"href":"external#Property"},42]),
        ),
        ("redefinedProperty", json!([{"$ref":"base"}])),
        ("redefinitionContext", json!(42)),
    ] {
        let mut model = document(false)?;
        author(&mut model, "replacement", key, value)?;
        assert!(has(&model, ERROR)?, "{key}");
    }
    let mut model = document(false)?;
    model.relations.push(serde_json::from_value(
        json!({"id":"cycle","kind":"uml.generalization","from":"base","to":"middle"}),
    )?);
    author(
        &mut model,
        "middle",
        "general",
        json!([{"$ref":"base"},{"href":"external#Base"}]),
    )?;
    assert!(has(&model, ERROR)?);
    Ok(())
}
#[test]
fn moving_redefiner_is_rejected_by_existing_context_gate_and_preserves_snapshot() -> Result {
    let model = document(false)?;
    let before = serde_json::to_value(&model)?;
    let error = apply(
        &model,
        &Command::SetOwner {
            element_id: "replacement".into(),
            owner_id: Some("other".into()),
        },
        1,
    )
    .expect_err("must reject");
    assert!(
        error.contains("UML_NAMESPACE_DISTINGUISHABILITY"),
        "{error}"
    );
    assert_eq!(before, serde_json::to_value(&model)?);
    Ok(())
}
#[test]
fn known_owned_feature_survives_unknown_ancestry_of_its_owner() -> Result {
    let mut model = document(false)?;
    author(
        &mut model,
        "base",
        "general",
        json!([{"href":"external#Ancestor"}]),
    )?;
    pass(&model)?;
    // A malformed sibling still invalidates canonical membership evidence.
    author(
        &mut model,
        "base",
        "ownedAttribute",
        json!([{"$ref":"original"},42]),
    )?;
    assert!(has(&model, ERROR)?);
    Ok(())
}

#[test]
fn external_only_target_does_not_hide_cycle_in_context_hierarchy() -> Result {
    let mut model = document(false)?;
    author(
        &mut model,
        "replacement",
        "redefinedProperty",
        json!([{"href":"external#Property"}]),
    )?;
    model.relations.push(serde_json::from_value(
        json!({"id":"cycle","kind":"uml.generalization","from":"base","to":"middle"}),
    )?);
    author(
        &mut model,
        "middle",
        "general",
        json!([{"href":"external#Base"},{"$ref":"base"}]),
    )?;
    assert!(has(&model, ERROR)?);
    assert!(!has(&model, UNKNOWN)?);
    Ok(())
}
