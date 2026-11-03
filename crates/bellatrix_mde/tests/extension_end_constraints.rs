use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project,
    uml::{references::ReferenceIndex, root::Model, types::{Integer, UnlimitedNatural}, value_specifications}};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const MULTIPLICITY: &str = "UML_EXTENSION_END_MULTIPLICITY";
const UNKNOWN: &str = "MDE_EXTENSION_END_MULTIPLICITY_UNVERIFIED";
const AGGREGATION: &str = "UML_EXTENSION_END_AGGREGATION";

fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Extension bounds","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"profile","kind":"uml.profile","name":"Profile"},
            {"id":"class","kind":"uml.class","name":"Metaclass"},
            {"id":"stereotype","kind":"uml.stereotype","name":"Stereotype"},
            {"id":"extension","kind":"uml.extension","name":"Extension","properties":{"memberEnd":[{"$ref":"base"},{"$ref":"end"}],"ownedEnd":{"$ref":"end"}}},
            {"id":"base","kind":"uml.attribute","name":"base_Class","properties":{"type":{"$ref":"class"}}},
            {"id":"end","kind":"uml.extension_end","name":"extension_Stereotype","properties":{"type":{"$ref":"stereotype"},"aggregation":"composite"}}
        ],"relations":[]});
    for (owner, child) in [("root","profile"),("root","class"),("profile","stereotype"),("profile","extension"),("stereotype","base"),("extension","end")] {
        value["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) {
    model.elements.iter_mut().find(|element| element.id == id).expect("element").properties.insert(key.into(), value);
}
fn has(model: &Project, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(response["diagnostics"].as_array().ok_or("diagnostics")?.iter().any(|issue| issue["subject_id"] == "end" && issue["code"] == code))
}
fn bound(model: &mut Project, id: &str, role: &str, kind: &str, value: Value) -> Result {
    model.elements.push(serde_json::from_value(json!({"id":id,"kind":kind,"name":"","properties":{"value":value}}))?);
    model.relations.push(serde_json::from_value(json!({"id":format!("owns-{id}"),"kind":"owns","from":"end","to":id}))?);
    author(model, "end", role, json!({"$ref":id}));
    Ok(())
}

#[test]
fn given_absent_or_null_bounds_then_extension_default_is_zero_to_one_without_changing_property_default() -> Result {
    let mut model = document()?;
    for null_bounds in [false, true] {
        if null_bounds {
            author(&mut model, "end", "lowerValue", Value::Null);
            author(&mut model, "end", "upperValue", Value::Null);
        }
        let references = ReferenceIndex::new(Model::from_project(&model));
        let end = references.element("end")?;
        assert_eq!(value_specifications::lower_bound(&references, end)?, Integer::from(0));
        assert_eq!(value_specifications::upper_bound(&references, end)?, UnlimitedNatural::from(1));
        assert_eq!(value_specifications::multiplicity(&references, end)?, "0..1".parse()?);
        assert_eq!(value_specifications::lower_bound(&references, references.element("base")?)?, Integer::from(1));
        assert_eq!(value_specifications::multiplicity(&references, references.element("base")?)?, "1".parse()?);
        assert!(!has(&model, MULTIPLICITY)? && !has(&model, UNKNOWN)?);
    }
    bound(&mut model, "upper", "upperValue", "uml.literal_unlimited_natural", json!(1))?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert_eq!(value_specifications::multiplicity(&references, references.element("end")?)?, "0..1".parse()?);
    Ok(())
}

#[test]
fn given_explicit_ranges_then_only_zero_to_one_or_one_are_allowed_and_roundtrip_is_stable() -> Result {
    for (range, valid) in [("0..1",true),("1",true),("0..0",false),("2..2",false),("0..2",false),("0..*",false),("-1..1",false),("2..1",false)] {
        let mut model = document()?;
        author(&mut model, "end", "multiplicity", json!(range));
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert_eq!(has(&reopened, MULTIPLICITY)?, !valid, "{range}");
            assert!(!has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}

#[test]
fn given_aggregation_then_only_composite_is_valid_and_absence_does_not_invent_composite() -> Result {
    for aggregation in [json!("none"),json!("shared"),json!("invalid"),json!(false),Value::Null] {
        let mut model = document()?;
        author(&mut model, "end", "aggregation", aggregation);
        assert!(has(&model, AGGREGATION)?);
    }
    let mut model = document()?;
    assert!(!has(&model, AGGREGATION)?);
    model.elements.iter_mut().find(|element| element.id == "end").ok_or("end")?.properties.remove("aggregation");
    assert!(has(&model, AGGREGATION)?);
    Ok(())
}

#[test]
fn given_identity_lower_values_then_zero_and_one_pass_but_two_and_wrong_type_fail() -> Result {
    for (value, valid) in [(0,true),(1,true),(2,false),(-1,false)] {
        let mut model = document()?;
        bound(&mut model, "lower", "lowerValue", "uml.literal_integer", json!(value))?;
        assert_eq!(has(&model, MULTIPLICITY)?, !valid);
        assert!(!has(&model, UNKNOWN)?);
    }
    let mut model = document()?;
    bound(&mut model, "lower", "lowerValue", "uml.literal_boolean", json!(true))?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert!(matches!(value_specifications::lower_bound(&references, references.element("end")?), Err(value_specifications::EvaluationError::Invalid { .. })));
    assert!(has(&model, MULTIPLICITY)?);
    Ok(())
}

#[test]
fn given_opaque_bound_then_known_opposite_violation_and_invalid_data_dominate_unknown() -> Result {
    for opaque_lower in [false, true] {
        let mut model = document()?;
        bound(&mut model, "lower", "lowerValue", if opaque_lower {"uml.opaque_expression"} else {"uml.literal_integer"}, json!(0))?;
        bound(&mut model, "upper", "upperValue", if opaque_lower {"uml.literal_unlimited_natural"} else {"uml.opaque_expression"}, json!(1))?;
        assert!(has(&model, UNKNOWN)? && !has(&model, MULTIPLICITY)?);
        author(&mut model, if opaque_lower {"upper"} else {"lower"}, "value", json!(2));
        assert!(has(&model, MULTIPLICITY)? && !has(&model, UNKNOWN)?);
        author(&mut model, if opaque_lower {"upper"} else {"lower"}, "value", json!(-1));
        assert!(has(&model, MULTIPLICITY)? && !has(&model, UNKNOWN)?);
    }
    Ok(())
}

#[test]
fn given_external_or_unassigned_bounds_then_no_default_masks_missing_evidence_or_invalid_identity() -> Result {
    let mut model = document()?;
    author(&mut model, "end", "lowerValue", json!({"href":"external.uml#Lower"}));
    assert!(has(&model, UNKNOWN)? && !has(&model, MULTIPLICITY)?);
    for invalid in [json!({"href":false}),json!({"$ref":"missing"}),json!({"$ref":"class"}),json!(false)] {
        author(&mut model, "end", "upperValue", invalid);
        assert!(has(&model, MULTIPLICITY)? && !has(&model, UNKNOWN)?);
    }
    let mut model = document()?;
    bound(&mut model, "lower", "lowerValue", "uml.literal_integer", json!(0))?;
    model.elements.iter_mut().find(|element| element.id == "end").ok_or("end")?.properties.remove("lowerValue");
    assert!(has(&model, UNKNOWN)? && !has(&model, MULTIPLICITY)?);
    Ok(())
}

#[test]
fn given_supported_association_container_when_extension_bound_changes_then_edit_is_atomic() -> Result {
    let mut model = document()?;
    model.elements.iter_mut().find(|element| element.id == "extension").ok_or("association")?.kind = "uml.nary_association".into();
    author(&mut model, "extension", "ownedEnd", json!([{"$ref":"end"}]));
    author(&mut model, "end", "multiplicity", json!("0..1"));
    let before = model.clone();
    let error = apply(&model, &Command::SetProperty {element_id:"end".into(),path:"properties.multiplicity".into(),value:json!("0..2")}, 1)
        .expect_err("ExtensionEnd upper must be one");
    assert!(error.contains("ExtensionEnd-multiplicity"), "{error}");
    assert_eq!(model, before);
    author(&mut model, "end", "multiplicity", json!("0..2"));
    let repaired = apply(&model, &Command::SetProperty {element_id:"end".into(),path:"properties.multiplicity".into(),value:json!("1")}, 2)?;
    assert!(!has(&repaired, MULTIPLICITY)?);
    Ok(())
}

#[test]
fn given_full_extension_container_then_existing_unsupported_gate_is_not_claimed_as_implemented() -> Result {
    let mut model = document()?;
    author(&mut model, "end", "multiplicity", json!("0..1"));
    let before = model.clone();
    let error = apply(&model, &Command::SetProperty {element_id:"end".into(),path:"properties.multiplicity".into(),value:json!("1")}, 1)
        .expect_err("full Extension rules remain unsupported");
    assert!(error.contains("MDE_ASSOCIATION_UNSUPPORTED"), "{error}");
    assert_eq!(model, before);
    Ok(())
}
