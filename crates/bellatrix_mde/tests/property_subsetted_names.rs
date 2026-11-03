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

fn aggregate(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "specific"
                && (issue["code"] == "UML_PROPERTY_SUBSETTING"
                    || issue["code"] == "UML_PROPERTY_SUBSETTING_UNSUPPORTED")
        })
        .cloned()
        .collect())
}
fn expect_aggregate(model: &Project, warning: bool) -> Result {
    let found = aggregate(model)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0]["code"],
        if warning {
            "UML_PROPERTY_SUBSETTING_UNSUPPORTED"
        } else {
            "UML_PROPERTY_SUBSETTING"
        },
        "{found:#?}"
    );
    assert_eq!(
        found[0]["severity"],
        if warning { "warning" } else { "error" }
    );
    Ok(())
}
fn unknown_member(model: &mut Project) -> Result {
    model.elements.push(serde_json::from_value(
        json!({"id":"unknown","kind":"uml.unknown_property","name":"Unknown"}),
    )?);
    Ok(())
}
#[test]
fn aggregate_retains_unknown_without_exposing_partial_public_collections() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    let mut model = document(true)?;
    unknown_member(&mut model)?;
    author(
        &mut model,
        "specific",
        "subsettedProperty",
        json!([{"$ref":"general"},{"$ref":"unknown"}]),
    )?;
    expect_aggregate(&model, true)?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert!(matches!(
        property_subsetting::subsetted_properties(&references, references.element("specific")?),
        Err(QueryError::Unsupported { .. })
    ));
    assert!(matches!(
        property_subsetting::compare_subsetting(
            &references,
            references.element("specific")?,
            references.element("unknown")?
        ),
        Err(QueryError::Unsupported { .. })
    ));
    for format in [Format::Json, Format::Protobuf] {
        expect_aggregate(&decode(&encode(&model, format)?, format)?, true)?;
    }
    Ok(())
}
#[test]
fn aggregate_known_equality_beats_unknown_in_both_orders() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        unknown_member(&mut model)?;
        name(&mut model, "specific", "General")?;
        let mut values = vec![json!({"$ref":"unknown"}), json!({"$ref":"general"})];
        if reverse {
            values.reverse();
        }
        author(&mut model, "specific", "subsettedProperty", json!(values))?;
        expect_aggregate(&model, false)?;
    }
    Ok(())
}
#[test]
fn aggregate_proven_invalid_contract_dominates_unknown_in_both_orders() -> Result {
    for invalid in [
        json!({"$ref":"missing"}),
        json!({"$ref":"owner"}),
        json!(false),
        json!({"href":""}),
    ] {
        for reverse in [false, true] {
            let mut model = document(true)?;
            unknown_member(&mut model)?;
            let mut values = vec![json!({"$ref":"unknown"}), invalid.clone()];
            if reverse {
                values.reverse();
            }
            author(&mut model, "specific", "subsettedProperty", json!(values))?;
            expect_aggregate(&model, false)?;
        }
    }
    let mut model = document(true)?;
    unknown_member(&mut model)?;
    for values in [
        json!([{"$ref":"unknown"},{"$ref":"unknown"}]),
        json!([{"$ref":"unknown"},{"href":"external.uml#P"},{"href":"external.uml#P"}]),
    ] {
        author(&mut model, "specific", "subsettedProperty", values)?;
        expect_aggregate(&model, false)?;
    }
    Ok(())
}
#[test]
fn unknown_owner_is_not_a_proven_empty_subsetting_context() -> Result {
    let mut model = document(true)?;
    model.elements.push(serde_json::from_value(
        json!({"id":"unknown-owner","kind":"uml.unknown_classifier","name":"Unknown"}),
    )?);
    model
        .relations
        .iter_mut()
        .find(|relation| relation.id == "owns-specific")
        .ok_or("owner")?
        .from = "unknown-owner".into();
    for id in ["specific", "general"] {
        author(&mut model, id, "type", json!({"$ref":"owner"}))?;
    }
    expect_aggregate(&model, true)?;
    name(&mut model, "specific", "General")?;
    expect_aggregate(&model, false)?;
    Ok(())
}
#[test]
fn subsetting_mutator_preserves_unknown_drafts_but_rejects_invalid_targets_atomically() -> Result {
    use bellatrix_mde::{ModelingSection, uml::property_subsetting};
    let mut model = document(true)?;
    unknown_member(&mut model)?;
    let mut section = ModelingSection::from_model(&model);
    property_subsetting::set_subsetted_properties(&mut section, "specific", &["unknown".into()])?;
    let unchanged = serde_json::to_value(&section)?;
    for target in ["owner", "missing"] {
        assert!(
            property_subsetting::set_subsetted_properties(
                &mut section,
                "specific",
                &["unknown".into(), target.into()]
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&section)?, unchanged);
    }
    property_subsetting::set_subsetted_properties(&mut section, "specific", &[])?;
    Ok(())
}

#[test]
fn unknown_local_type_remains_unknown_and_known_bad_type_remains_invalid() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, feature_compatibility, references::ReferenceIndex, root::Model,
    };
    for reverse in [false, true] {
        let mut model = document(true)?;
        unknown_member(&mut model)?;
        for (id, target) in if reverse {
            [("specific", "owner"), ("general", "unknown")]
        } else {
            [("specific", "unknown"), ("general", "owner")]
        } {
            author(&mut model, id, "type", json!({"$ref":target}))?;
        }
        expect_aggregate(&model, true)?;
        let other = if reverse { "specific" } else { "general" };
        for value in [
            json!({"$ref":"general"}),
            json!({"$ref":"missing"}),
            json!(false),
        ] {
            author(&mut model, other, "type", value)?;
            expect_aggregate(&model, false)?;
            let references = ReferenceIndex::new(Model::from_project(&model));
            assert!(matches!(
                feature_compatibility::compare_type_conformance(
                    &references,
                    references.element("specific")?,
                    references.element("general")?
                ),
                Err(QueryError::Invalid { .. })
            ));
        }
    }
    Ok(())
}

#[test]
fn unresolved_association_end_metaclass_does_not_become_invalid_context() -> Result {
    let mut model = document(true)?;
    unknown_member(&mut model)?;
    for id in ["specific", "general"] {
        author(&mut model, id, "type", json!({"$ref":"owner"}))?;
    }
    model.elements.push(serde_json::from_value(json!({"id":"association","kind":"uml.association","name":"Association","properties":{"memberEnd":[{"$ref":"specific"},{"$ref":"unknown"}]}}))?);
    expect_aggregate(&model, true)?;
    name(&mut model, "specific", "General")?;
    expect_aggregate(&model, false)?;
    name(&mut model, "specific", "Specific")?;
    author(
        &mut model,
        "association",
        "memberEnd",
        json!([{"$ref":"specific"},{"$ref":"unknown"},{"$ref":"missing"}]),
    )?;
    expect_aggregate(&model, false)?;
    Ok(())
}

#[test]
fn unknown_upper_cannot_hide_invalid_opposite_upper_in_either_order() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    for reverse in [false, true] {
        for invalid in [
            json!(false),
            json!({"$ref":"missing"}),
            json!({"$ref":"owner"}),
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
                "upperValue",
                json!({"href":"external.uml#Upper"}),
            )?;
            author(&mut model, known, "upperValue", invalid)?;
            expect_aggregate(&model, false)?;
            let references = ReferenceIndex::new(Model::from_project(&model));
            assert!(matches!(
                property_subsetting::compare_subsetting(
                    &references,
                    references.element("specific")?,
                    references.element("general")?
                ),
                Err(QueryError::Invalid { .. })
            ));
        }
    }
    Ok(())
}

#[test]
fn unknown_context_cannot_hide_invalid_opposite_context_in_either_order() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    for reverse in [false, true] {
        let mut model = document(true)?;
        unknown_member(&mut model)?;
        let (unknown, invalid) = if reverse {
            ("general", "specific")
        } else {
            ("specific", "general")
        };
        for (id, property, other) in [
            ("unknown-association", unknown, "unknown"),
            ("invalid-association", invalid, "missing"),
        ] {
            model.elements.push(serde_json::from_value(json!({"id":id,"kind":"uml.association","name":id,"properties":{"memberEnd":[{"$ref":property},{"$ref":other}]}}))?);
        }
        expect_aggregate(&model, false)?;
        let references = ReferenceIndex::new(Model::from_project(&model));
        assert!(matches!(
            property_subsetting::compare_subsetting(
                &references,
                references.element("specific")?,
                references.element("general")?
            ),
            Err(QueryError::Invalid { .. })
        ));
    }
    Ok(())
}

#[test]
fn nary_context_retains_invalid_end_type_beside_unknown_in_both_orders() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    for reverse in [false, true] {
        for invalid_type in [json!({"$ref":"missing"}), json!(false)] {
            let mut model = document(true)?;
            model.elements.push(serde_json::from_value(json!({"id":"unknown-end","kind":"uml.attribute","name":"Unknown","properties":{"type":{"href":"external.uml#Type"}}}))?);
            model.elements.push(serde_json::from_value(json!({"id":"invalid-end","kind":"uml.attribute","name":"Invalid","properties":{"type":invalid_type}}))?);
            let mut members = vec![
                json!({"$ref":"specific"}),
                json!({"$ref":"unknown-end"}),
                json!({"$ref":"invalid-end"}),
            ];
            if reverse {
                members.reverse();
            }
            model.elements.push(serde_json::from_value(json!({"id":"association","kind":"uml.association","name":"Association","properties":{"memberEnd":members}}))?);
            expect_aggregate(&model, false)?;
            let references = ReferenceIndex::new(Model::from_project(&model));
            assert!(matches!(
                property_subsetting::subsetting_context(
                    &references,
                    references.element("specific")?
                ),
                Err(QueryError::Invalid { .. })
            ));
        }
    }
    Ok(())
}

#[test]
fn unknown_member_contract_cannot_hide_known_invalid_context_type() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    for unknown in [
        json!({"href":"external.uml#End"}),
        json!({"$ref":"unknown"}),
    ] {
        for reverse in [false, true] {
            let mut model = document(true)?;
            unknown_member(&mut model)?;
            model.elements.push(serde_json::from_value(json!({"id":"invalid-end","kind":"uml.attribute","name":"Invalid","properties":{"type":{"$ref":"missing"}}}))?);
            let mut members = vec![
                json!({"$ref":"specific"}),
                unknown.clone(),
                json!({"$ref":"invalid-end"}),
            ];
            if reverse {
                members.reverse();
            }
            model.elements.push(serde_json::from_value(json!({"id":"association","kind":"uml.association","name":"Association","properties":{"memberEnd":members}}))?);
            expect_aggregate(&model, false)?;
            let references = ReferenceIndex::new(Model::from_project(&model));
            assert!(matches!(
                property_subsetting::subsetting_context(
                    &references,
                    references.element("specific")?
                ),
                Err(QueryError::Invalid { .. })
            ));
        }
    }
    Ok(())
}

#[test]
fn public_comparison_retains_known_wrong_metaclass_beside_unknown_operand() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    let mut model = document(true)?;
    unknown_member(&mut model)?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    for (specific, general) in [("unknown", "owner"), ("owner", "unknown")] {
        assert!(matches!(
            property_subsetting::compare_subsetting(
                &references,
                references.element(specific)?,
                references.element(general)?
            ),
            Err(QueryError::Invalid { .. })
        ));
    }
    Ok(())
}
