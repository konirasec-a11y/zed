use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project,
    uml::{classifiers::QueryError, profiles, references::ReferenceIndex, root::Model}};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
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
    Ok(response["diagnostics"].as_array().ok_or("diagnostics")?.iter().any(|issue| issue["subject_id"] == "extension" && issue["code"] == code))
}
fn required(model: &Project) -> std::result::Result<bool, QueryError> {
    let references = ReferenceIndex::new(Model::from_project(model));
    profiles::is_required(&references, references.element("extension").expect("extension"))
}

#[test]
fn given_extension_member_count_then_binary_is_decided_without_resolving_external_identities() -> Result {
    let mut model = document()?;
    assert!(!has(&model, "UML_EXTENSION_IS_BINARY")? && !has(&model, "MDE_EXTENSION_IS_BINARY_UNVERIFIED")?);
    author(&mut model, "extension", "memberEnd", json!([{"href":"external.uml#A"},{"href":"external.uml#B"}]));
    assert!(!has(&model, "UML_EXTENSION_IS_BINARY")? && !has(&model, "MDE_EXTENSION_IS_BINARY_UNVERIFIED")?);
    author(&mut model, "extension", "memberEnd", json!([{"href":"external.uml#A"},{"href":"external.uml#B"},{"href":"external.uml#C"}]));
    assert!(has(&model, "UML_EXTENSION_IS_BINARY")?);
    for invalid in [json!(false), json!([{"href":"external.uml#A"},false]), json!([false,{"href":"external.uml#A"}]), json!([{"$ref":"base"},{"$ref":"base"}])] {
        author(&mut model, "extension", "memberEnd", invalid);
        assert!(has(&model, "UML_EXTENSION_IS_BINARY")?);
    }
    model.elements.iter_mut().find(|element| element.id == "extension").ok_or("extension")?.properties.remove("memberEnd");
    assert!(has(&model, "MDE_EXTENSION_IS_BINARY_UNVERIFIED")?);
    Ok(())
}

#[test]
fn given_owned_extension_end_then_required_follows_lower_bound_and_ignores_derived_caches() -> Result {
    let mut model = document()?;
    assert_eq!(required(&model)?, false);
    for (range, expected) in [("0..1",false),("1",true),("2..2",false)] {
        author(&mut model, "end", "multiplicity", json!(range));
        author(&mut model, "extension", "isRequired", json!(!expected));
        author(&mut model, "extension", "metaclass", json!({"$ref":"stereotype"}));
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert_eq!(required(&reopened)?, expected);
        }
    }
    Ok(())
}

#[test]
fn given_owned_end_contract_then_single_typed_canonical_reference_is_required() -> Result {
    let mut model = document()?;
    for invalid in [Value::Null,json!([]),json!([{"$ref":"end"}]),json!({"$ref":"base"}),json!({"$ref":"missing"}),json!({"href":false})] {
        author(&mut model, "extension", "ownedEnd", invalid);
        assert!(has(&model, "UML_EXTENSION_OWNED_END")?);
        assert!(matches!(required(&model), Err(QueryError::Invalid { .. })));
    }
    author(&mut model, "extension", "ownedEnd", json!({"href":"external.uml#End"}));
    assert!(has(&model, "MDE_EXTENSION_OWNED_END_UNVERIFIED")?);
    assert!(matches!(required(&model), Err(QueryError::Unsupported { .. })));
    model.elements.iter_mut().find(|element| element.id == "extension").ok_or("extension")?.properties.remove("ownedEnd");
    assert!(matches!(required(&model), Err(QueryError::Unsupported { .. })), "owns alone does not replace the materialized contract in this slice");
    Ok(())
}

#[test]
fn given_unknown_owned_contract_then_extra_canonical_extension_end_remains_invalid() -> Result {
    let mut model = document()?;
    model.elements.push(serde_json::from_value(json!({"id":"extra","kind":"uml.extension_end","name":"Extra","properties":{"type":{"$ref":"stereotype"},"aggregation":"composite"}}))?);
    model.relations.push(serde_json::from_value(json!({"id":"owns-extra","kind":"owns","from":"extension","to":"extra"}))?);
    for owned in [json!({"href":"external.uml#End"}),json!({"$ref":"end"})] {
        author(&mut model, "extension", "ownedEnd", owned);
        author(&mut model, "end", "lowerValue", json!({"href":"external.uml#Lower"}));
        assert!(has(&model, "UML_EXTENSION_OWNED_END")?);
        assert!(matches!(required(&model), Err(QueryError::Invalid { .. })));
    }
    Ok(())
}

#[test]
fn given_lower_identity_then_is_required_uses_lower_only_and_retains_invalid_or_unknown_values() -> Result {
    let mut model = document()?;
    model.elements.push(serde_json::from_value(json!({"id":"lower","kind":"uml.literal_integer","name":"","properties":{"value":1}}))?);
    model.relations.push(serde_json::from_value(json!({"id":"owns-lower","kind":"owns","from":"end","to":"lower"}))?);
    author(&mut model, "end", "lowerValue", json!({"$ref":"lower"}));
    author(&mut model, "end", "upperValue", json!({"href":"external.uml#Upper"}));
    assert_eq!(required(&model)?, true);
    author(&mut model, "end", "lowerValue", json!({"href":"external.uml#Lower"}));
    assert!(matches!(required(&model), Err(QueryError::Unsupported { .. })));
    author(&mut model, "end", "lowerValue", json!({"href":false}));
    assert!(matches!(required(&model), Err(QueryError::Invalid { .. })));
    Ok(())
}

#[test]
fn given_partial_extension_support_then_ambiguous_metaclass_end_still_blocks_full_edit_gate() -> Result {
    let mut model = document()?;
    author(&mut model, "end", "multiplicity", json!("0..1"));
    let before = model.clone();
    let error = apply(&model, &Command::SetProperty {element_id:"end".into(),path:"properties.multiplicity".into(),value:json!("1")}, 1)
        .expect_err("non_owned_end still needs normative clarification");
    assert!(error.contains("Unsupported Extension-non_owned_end"), "{error}");
    assert_eq!(model, before);
    Ok(())
}
