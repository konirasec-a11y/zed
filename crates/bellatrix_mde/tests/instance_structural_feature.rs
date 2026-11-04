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
