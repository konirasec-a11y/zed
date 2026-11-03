use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PROPERTY_SUBSETTING_RULES";
const INVALID: &str = "MDE_PROPERTY_SUBSETTING_RULES_INVALID";
const UNKNOWN: &str = "MDE_PROPERTY_SUBSETTING_RULES_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut model = json!({"id":"project","name":"Subsetting rules","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"owner","kind":"uml.class","name":"Owner"},
        {"id":"super","kind":"uml.class","name":"Super"},
        {"id":"middle","kind":"uml.class","name":"Middle"},
        {"id":"sub","kind":"uml.class","name":"Sub"},
        {"id":"general","kind":"uml.attribute","name":"General","properties":{"type":{"$ref":"super"}}},
        {"id":"specific","kind":"uml.attribute","name":"Specific","properties":{"type":{"$ref":"super"},"subsettedProperty":[{"$ref":"general"}]}}
    ],"relations":[
        {"id":"sub-middle","kind":"uml.generalization","from":"sub","to":"middle"},
        {"id":"middle-super","kind":"uml.generalization","from":"middle","to":"super"}
    ]});
    for (owner, child) in [
        ("root", "owner"),
        ("root", "super"),
        ("root", "middle"),
        ("root", "sub"),
        ("owner", "general"),
        ("owner", "specific"),
    ] {
        model["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
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
fn type_conformance_supports_identity_and_transitive_specialization_but_not_reverse() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        for target in ["super", "middle", "sub"] {
            author(&mut model, "specific", "type", json!({"$ref":target}))?;
            assert!(issues(&model)?.is_empty());
        }
        author(&mut model, "specific", "type", json!({"$ref":"super"}))?;
        author(&mut model, "general", "type", json!({"$ref":"sub"}))?;
        expect(&model, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn upper_is_restricted_without_imposing_lower_bound_containment() -> Result {
    let mut model = document(true)?;
    assert!(issues(&model)?.is_empty());
    for (specific, general, valid) in [
        ("0", "0", true),
        ("1", "0", false),
        ("0..3", "2..5", true),
        ("0..5", "2..5", true),
        ("0..6", "2..5", false),
        ("0..*", "2..5", false),
        ("0..*", "2..*", true),
        ("0..184467440737095516160", "0..184467440737095516160", true),
        (
            "0..184467440737095516161",
            "0..184467440737095516160",
            false,
        ),
    ] {
        author(&mut model, "specific", "multiplicity", json!(specific))?;
        author(&mut model, "general", "multiplicity", json!(general))?;
        if valid {
            assert!(issues(&model)?.is_empty(), "{specific}/{general}");
        } else {
            expect(&model, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn individual_rule_does_not_depend_on_names_or_subsetting_context() -> Result {
    let mut model = document(true)?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "specific")
        .ok_or("specific")?
        .name = "General".into();
    author(
        &mut model,
        "specific",
        "subsettingContext",
        json!([{"href":"external.uml#Context"}]),
    )?;
    assert!(issues(&model)?.is_empty());
    author(&mut model, "specific", "multiplicity", json!("0..2"))?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn known_false_clause_dominates_unknown_other_clause() -> Result {
    let mut model = document(true)?;
    author(
        &mut model,
        "specific",
        "type",
        json!({"href":"external.uml#Type"}),
    )?;
    expect(&model, UNKNOWN)?;
    author(&mut model, "specific", "multiplicity", json!("0..2"))?;
    expect(&model, ERROR)?;
    author(&mut model, "specific", "type", json!({"$ref":"owner"}))?;
    author(
        &mut model,
        "specific",
        "upperValue",
        json!({"href":"external.uml#Upper"}),
    )?;
    expect(&model, ERROR)?;
    author(&mut model, "specific", "type", json!({"$ref":"super"}))?;
    expect(&model, UNKNOWN)?;
    Ok(())
}
#[test]
fn invalid_type_or_upper_dominates_unknown_in_either_operand_order() -> Result {
    for key in ["type", "upperValue"] {
        for reverse in [false, true] {
            for invalid in [
                json!(false),
                json!({"$ref":"missing"}),
                json!({"$ref":"general"}),
            ] {
                let mut model = document(true)?;
                let (unknown, known) = if reverse {
                    ("general", "specific")
                } else {
                    ("specific", "general")
                };
                author(
                    &mut model,
                    unknown,
                    key,
                    json!({"href":"external.uml#Value"}),
                )?;
                author(&mut model, known, key, invalid)?;
                expect(&model, INVALID)?;
            }
        }
    }
    Ok(())
}
#[test]
fn unknown_subset_neighbors_do_not_hide_known_upper_violation() -> Result {
    for unknown in [
        json!({"href":"external.uml#Property"}),
        json!({"$ref":"unknown"}),
    ] {
        for reverse in [false, true] {
            let mut model = document(true)?;
            model.elements.push(serde_json::from_value(
                json!({"id":"unknown","kind":"uml.unknown_property","name":"Unknown"}),
            )?);
            let mut values = vec![unknown.clone(), json!({"$ref":"general"})];
            if reverse {
                values.reverse();
            }
            author(&mut model, "specific", "subsettedProperty", json!(values))?;
            expect(&model, UNKNOWN)?;
            author(&mut model, "specific", "multiplicity", json!("0..2"))?;
            expect(&model, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn missing_subsets_are_vacuous_while_invalid_subset_contracts_are_not() -> Result {
    let mut model = document(true)?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "specific")
        .ok_or("specific")?
        .properties
        .remove("subsettedProperty");
    author(
        &mut model,
        "specific",
        "type",
        json!({"href":"external.uml#Type"}),
    )?;
    assert!(issues(&model)?.is_empty());
    author(&mut model, "specific", "subsettedProperty", json!([]))?;
    assert!(issues(&model)?.is_empty());
    for value in [
        Value::Null,
        json!(42),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"owner"}]),
        json!([{"$ref":"general"},{"$ref":"general"}]),
        json!([{"href":"external.uml#P"},false]),
    ] {
        author(&mut model, "specific", "subsettedProperty", value)?;
        expect(&model, INVALID)?;
    }
    Ok(())
}
#[test]
fn inherited_property_metaclasses_and_absent_types_keep_normative_boundaries() -> Result {
    for kind in ["uml.attribute", "uml.port", "uml.extension_end"] {
        let mut model = document(true)?;
        model
            .elements
            .iter_mut()
            .find(|element| element.id == "specific")
            .ok_or("specific")?
            .kind = kind.into();
        if kind == "uml.extension_end" {
            model.elements.push(serde_json::from_value(
                json!({"id":"stereotype","kind":"uml.stereotype","name":"Stereotype"}),
            )?);
            for id in ["specific", "general"] {
                author(&mut model, id, "type", json!({"$ref":"stereotype"}))?;
            }
        }
        assert!(issues(&model)?.is_empty(), "{kind}: {:?}", issues(&model)?);
        author(&mut model, "specific", "type", Value::Null)?;
        expect(
            &model,
            if kind == "uml.extension_end" {
                INVALID
            } else {
                UNKNOWN
            },
        )?;
        if kind == "uml.extension_end" {
            author(&mut model, "specific", "type", json!({"$ref":"stereotype"}))?;
        }
        author(&mut model, "specific", "multiplicity", json!("0..2"))?;
        expect(&model, ERROR)?;
    }
    Ok(())
}

#[test]
fn canonical_subsetting_mutator_rejects_type_or_upper_violation_without_mutation() -> Result {
    use bellatrix_mde::{ModelingSection, uml::property_subsetting};
    for (key, value) in [
        ("type", json!({"$ref":"owner"})),
        ("multiplicity", json!("0..2")),
    ] {
        let mut model = document(true)?;
        author(&mut model, "specific", "subsettedProperty", json!([]))?;
        author(&mut model, "specific", key, value)?;
        let mut section = ModelingSection::from_model(&model);
        let unchanged = serde_json::to_value(&section)?;
        let error = property_subsetting::set_subsetted_properties(
            &mut section,
            "specific",
            &["general".into()],
        )
        .expect_err("subsetting must restrict type and upper");
        assert!(error.contains("subsetting"), "{error}");
        assert_eq!(serde_json::to_value(&section)?, unchanged);
        let property = section
            .elements
            .iter_mut()
            .find(|element| element.id == "specific")
            .ok_or("specific")?;
        property
            .properties
            .insert("type".into(), json!({"$ref":"super"}));
        property
            .properties
            .insert("multiplicity".into(), json!("1"));
        property_subsetting::set_subsetted_properties(
            &mut section,
            "specific",
            &["general".into()],
        )?;
    }
    Ok(())
}
