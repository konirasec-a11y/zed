use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PROPERTY_SUBSETTED_PROPERTY_NAMES";
const INVALID: &str = "MDE_PROPERTY_SUBSETTED_PROPERTY_NAMES_INVALID";
const UNKNOWN: &str = "MDE_PROPERTY_SUBSETTED_PROPERTY_NAMES_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut model = json!({"id":"project","name":"Subsetted property names","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"owner","kind":"uml.class","name":"Owner"},
        {"id":"general","kind":"uml.attribute","name":"General"},
        {"id":"specific","kind":"uml.attribute","name":"Specific","properties":{"subsettedProperty":[{"$ref":"general"}]}}
    ],"relations":[
        {"id":"owns-owner","kind":"owns","from":"root","to":"owner"},
        {"id":"owns-general","kind":"owns","from":"owner","to":"general"},
        {"id":"owns-specific","kind":"owns","from":"owner","to":"specific"}
    ]});
    if !rooted {
        model.as_object_mut().ok_or("model")?.remove("root_model");
    }
    Ok(serde_json::from_value(model)?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn name(model: &mut Project, id: &str, value: &str) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .name = value.into();
    Ok(())
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "specific"
                && [ERROR, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| issue["code"] == *code)
        })
        .cloned()
        .collect())
}
fn expect(model: &Project, code: &str) -> Result {
    let found = issues(model)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    assert_eq!(
        found[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
#[test]
fn names_are_compared_without_type_context_or_upper_bound_dependencies() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        author(
            &mut model,
            "specific",
            "type",
            json!({"href":"external.uml#Type"}),
        )?;
        author(
            &mut model,
            "specific",
            "upperValue",
            json!({"href":"external.uml#Upper"}),
        )?;
        assert!(issues(&model)?.is_empty());
        name(&mut model, "specific", "General")?;
        expect(&model, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn absent_names_compare_as_null_and_equality_is_case_sensitive() -> Result {
    let mut model = document(true)?;
    for (specific, general, equal) in [
        ("", "", true),
        ("", "General", false),
        ("Specific", "", false),
        ("Name", "name", false),
        ("same", "same", true),
    ] {
        name(&mut model, "specific", specific)?;
        name(&mut model, "general", general)?;
        if equal {
            expect(&model, ERROR)?;
        } else {
            assert!(issues(&model)?.is_empty());
        }
    }
    Ok(())
}
#[test]
fn optional_empty_or_missing_subsets_are_vacuously_satisfied() -> Result {
    let mut model = document(true)?;
    author(&mut model, "specific", "subsettedProperty", json!([]))?;
    assert!(issues(&model)?.is_empty());
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "specific")
        .ok_or("specific")?
        .properties
        .remove("subsettedProperty");
    assert!(issues(&model)?.is_empty());
    Ok(())
}
#[test]
fn known_equal_names_dominate_external_members_in_both_orders() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        let mut members = vec![
            json!({"$ref":"general"}),
            json!({"href":"external.uml#Property"}),
        ];
        if reverse {
            members.reverse();
        }
        author(&mut model, "specific", "subsettedProperty", json!(members))?;
        expect(&model, UNKNOWN)?;
        name(&mut model, "specific", "General")?;
        expect(&model, ERROR)?;
    }
    Ok(())
}
#[test]
fn unknown_local_metaclasses_do_not_invent_null_names_or_hide_known_equality() -> Result {
    let mut model = document(true)?;
    model.elements.push(serde_json::from_value(
        json!({"id":"unknown","kind":"uml.unknown_property","name":""}),
    )?);
    name(&mut model, "specific", "")?;
    author(
        &mut model,
        "specific",
        "subsettedProperty",
        json!([{"$ref":"unknown"}]),
    )?;
    expect(&model, UNKNOWN)?;
    author(
        &mut model,
        "specific",
        "subsettedProperty",
        json!([{"$ref":"unknown"},{"$ref":"general"}]),
    )?;
    expect(&model, UNKNOWN)?;
    name(&mut model, "general", "")?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn invalid_reference_contracts_dominate_unknown_and_name_equality() -> Result {
    for value in [
        Value::Null,
        json!(42),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"owner"}]),
        json!([{"$ref":"general"},{"$ref":"general"}]),
        json!([{"href":"external.uml#P"},false]),
        json!([false,{"href":"external.uml#P"}]),
        json!([{"href":""}]),
    ] {
        let mut model = document(true)?;
        name(&mut model, "specific", "General")?;
        author(&mut model, "specific", "subsettedProperty", value)?;
        expect(&model, INVALID)?;
    }
    Ok(())
}
#[test]
fn property_specializations_inherit_the_name_constraint() -> Result {
    for kind in ["uml.attribute", "uml.port", "uml.extension_end"] {
        let mut model = document(true)?;
        model
            .elements
            .iter_mut()
            .find(|element| element.id == "specific")
            .ok_or("specific")?
            .kind = kind.into();
        name(&mut model, "specific", "General")?;
        expect(&model, ERROR)?;
    }
    Ok(())
}
