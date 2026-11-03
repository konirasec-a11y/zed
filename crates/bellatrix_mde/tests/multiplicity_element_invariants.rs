use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const INVALID: &str = "UML_MULTIPLICITY_VALUE";
const UNKNOWN: &str = "MDE_MULTIPLICITY_UNSUPPORTED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Multiplicity invariants","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"property","kind":"uml.property","name":"Property","properties":{"lowerValue":{"$ref":"lower"},"upperValue":{"$ref":"upper"}}},
        {"id":"lower","kind":"uml.literal_integer","name":"","properties":{"value":0}},
        {"id":"upper","kind":"uml.literal_unlimited_natural","name":"","properties":{"value":1}}
    ],"relations":[
        {"id":"owns-class","kind":"owns","from":"root","to":"class"},
        {"id":"owns-property","kind":"owns","from":"class","to":"property"},
        {"id":"owns-lower","kind":"owns","from":"property","to":"lower"},
        {"id":"owns-upper","kind":"owns","from":"property","to":"upper"}
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
fn kind(p: &mut Project, id: &str, kind: &str) -> Result {
    p.elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .kind = kind.into();
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
            issue["subject_id"] == "property"
                && [
                    INVALID,
                    UNKNOWN,
                    "UML_PARAMETER_MULTIPLICITY",
                    "MDE_PARAMETER_MULTIPLICITY_UNSUPPORTED",
                ]
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
fn upper_ge_lower_compares_exact_bounds_including_arbitrary_precision_and_unlimited() -> Result {
    for rooted in [false, true] {
        let mut p = document(rooted)?;
        for (lower, upper) in [
            (json!(0), json!(0)),
            (json!(1), json!(1)),
            (
                json!("184467440737095516160"),
                json!("184467440737095516160"),
            ),
            (json!("184467440737095516160"), json!("*")),
        ] {
            author(&mut p, "lower", "value", lower)?;
            author(&mut p, "upper", "value", upper)?;
            assert!(issues(&p)?.is_empty());
        }
        author(&mut p, "upper", "value", json!("184467440737095516159"))?;
        expect(&p, INVALID)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&p, format)?, format)?, INVALID)?;
        }
    }
    Ok(())
}
#[test]
fn lower_ge_zero_rejects_negative_integer_even_with_unlimited_or_unknown_upper() -> Result {
    let mut p = document(false)?;
    author(&mut p, "lower", "value", json!(-1))?;
    author(&mut p, "upper", "value", json!("*"))?;
    expect(&p, INVALID)?;
    author(
        &mut p,
        "property",
        "upperValue",
        json!({"href":"external.uml#Upper"}),
    )?;
    expect(&p, INVALID)?;
    author(&mut p, "lower", "value", json!(0))?;
    expect(&p, UNKNOWN)?;
    Ok(())
}
#[test]
fn lower_is_integer_requires_an_integer_value_not_merely_a_numeric_literal() -> Result {
    for (literal, value) in [
        ("uml.literal_boolean", json!(true)),
        ("uml.literal_real", json!(1.0)),
        ("uml.literal_unlimited_natural", json!(1)),
        ("uml.literal_string", json!("1")),
        ("uml.literal_null", Value::Null),
    ] {
        let mut p = document(false)?;
        kind(&mut p, "lower", literal)?;
        author(&mut p, "lower", "value", value)?;
        expect(&p, INVALID)?;
        let found = issues(&p)?;
        assert!(
            found[0]["message"]
                .as_str()
                .ok_or("message")?
                .contains("lower_is_integer"),
            "{found:#?}"
        );
    }
    Ok(())
}
#[test]
fn upper_is_unlimited_natural_does_not_accept_integer_or_null_literals() -> Result {
    for (literal, value) in [
        ("uml.literal_integer", json!(1)),
        ("uml.literal_boolean", json!(true)),
        ("uml.literal_real", json!(1.0)),
        ("uml.literal_string", json!("*")),
        ("uml.literal_null", Value::Null),
    ] {
        let mut p = document(false)?;
        kind(&mut p, "upper", literal)?;
        author(&mut p, "upper", "value", value)?;
        expect(&p, INVALID)?;
        let found = issues(&p)?;
        assert!(
            found[0]["message"]
                .as_str()
                .ok_or("message")?
                .contains("upper_is_unlimitedNatural"),
            "{found:#?}"
        );
    }
    Ok(())
}
#[test]
fn absent_bound_and_null_reference_use_defaults_but_literal_null_is_a_value() -> Result {
    let mut p = document(false)?;
    for bound in ["lower", "upper"] {
        p.elements.retain(|element| element.id != bound);
        p.relations.retain(|relation| relation.to != bound);
        author(
            &mut p,
            "property",
            if bound == "lower" {
                "lowerValue"
            } else {
                "upperValue"
            },
            Value::Null,
        )?;
    }
    assert!(issues(&p)?.is_empty());
    p.elements
        .iter_mut()
        .find(|element| element.id == "property")
        .ok_or("property")?
        .properties
        .clear();
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn invalid_opposite_dominates_opaque_or_external_bounds_in_both_directions() -> Result {
    for unknown_lower in [false, true] {
        let mut p = document(false)?;
        let (unknown, invalid) = if unknown_lower {
            ("lower", "upper")
        } else {
            ("upper", "lower")
        };
        kind(&mut p, unknown, "uml.opaque_expression")?;
        kind(&mut p, invalid, "uml.literal_boolean")?;
        author(&mut p, invalid, "value", json!(true))?;
        expect(&p, INVALID)?;
        author(
            &mut p,
            "property",
            if unknown_lower {
                "lowerValue"
            } else {
                "upperValue"
            },
            json!({"href":"external.uml#Bound"}),
        )?;
        expect(&p, INVALID)?;
    }
    Ok(())
}
#[test]
fn malformed_bound_references_and_wrong_ownership_never_become_unknown() -> Result {
    for value in [
        json!(42),
        json!({"href":42}),
        json!({"$ref":"missing"}),
        json!({"$ref":"class"}),
    ] {
        let mut p = document(false)?;
        author(&mut p, "property", "lowerValue", value)?;
        author(
            &mut p,
            "property",
            "upperValue",
            json!({"href":"external.uml#Upper"}),
        )?;
        expect(&p, INVALID)?;
    }
    let mut p = document(false)?;
    p.relations
        .iter_mut()
        .find(|relation| relation.id == "owns-lower")
        .ok_or("ownership")?
        .from = "class".into();
    expect(&p, INVALID)?;
    Ok(())
}
#[test]
fn opaque_evaluation_remains_unverified_and_does_not_claim_constancy_or_purity() -> Result {
    let mut p = document(false)?;
    kind(&mut p, "lower", "uml.opaque_expression")?;
    author(&mut p, "lower", "body", json!(["0"]))?;
    author(&mut p, "lower", "language", json!(["OCL"]))?;
    expect(&p, UNKNOWN)?;
    Ok(())
}
#[test]
fn invalid_draft_roundtrips_and_repair_uses_same_literal_records() -> Result {
    let valid = document(true)?;
    let mut invalid = valid.clone();
    author(&mut invalid, "lower", "value", json!(2))?;
    assert!(issues(&valid)?.is_empty());
    expect(&invalid, INVALID)?;
    assert!(issues(&valid)?.is_empty());
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, INVALID)?;
        author(&mut reopened, "lower", "value", json!(0))?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened.elements, valid.elements);
        assert_eq!(reopened.relations, valid.relations);
    }
    Ok(())
}

#[test]
fn unknown_value_metaclass_is_unsupported_but_known_non_value_is_invalid() -> Result {
    use bellatrix_mde::uml::{
        references::ReferenceIndex,
        root::Model,
        value_specifications::{self, EvaluationError},
    };
    let mut p = document(false)?;
    kind(&mut p, "lower", "uml.future_value")?;
    let references = ReferenceIndex::new(Model::from_project(&p));
    assert!(matches!(
        value_specifications::evaluate(&references, "lower"),
        Err(EvaluationError::Unsupported { .. })
    ));
    assert!(matches!(
        value_specifications::is_computable(&references, "lower"),
        Err(EvaluationError::Unsupported { .. })
    ));
    assert!(matches!(
        value_specifications::evaluate(&references, "class"),
        Err(EvaluationError::Invalid { .. })
    ));
    Ok(())
}

#[test]
fn unknown_local_bound_is_unverified_without_hiding_invalid_opposite_or_owner() -> Result {
    for unknown_lower in [false, true] {
        let mut p = document(false)?;
        let (unknown, known) = if unknown_lower {
            ("lower", "upper")
        } else {
            ("upper", "lower")
        };
        kind(&mut p, unknown, "uml.future_value")?;
        expect(&p, UNKNOWN)?;
        kind(&mut p, known, "uml.literal_boolean")?;
        author(&mut p, known, "value", json!(true))?;
        expect(&p, INVALID)?;
        let mut p = document(false)?;
        kind(&mut p, unknown, "uml.future_value")?;
        p.relations
            .iter_mut()
            .find(|relation| relation.to == unknown)
            .ok_or("bound ownership")?
            .from = "class".into();
        expect(&p, INVALID)?;
    }
    Ok(())
}

#[test]
fn exact_default_bounds_are_one_to_one_except_extension_end_zero_to_one() -> Result {
    use bellatrix_mde::uml::{references::ReferenceIndex, root::Model, value_specifications};
    for (element_kind, expected_lower) in [
        ("uml.property", "1"),
        ("uml.parameter", "1"),
        ("uml.extension_end", "0"),
    ] {
        for nulls in [false, true] {
            let properties = if nulls {
                json!({"lowerValue":null,"upperValue":null})
            } else {
                json!({})
            };
            let p: Project = serde_json::from_value(json!({"id":"defaults","name":"Defaults",
                "elements":[{"id":"element","kind":element_kind,"name":"","properties":properties}]}))?;
            let references = ReferenceIndex::new(Model::from_project(&p));
            let element = references.element("element")?;
            let range = value_specifications::multiplicity(&references, element)?;
            assert_eq!(
                range.lower_bound().to_string(),
                expected_lower,
                "{element_kind}"
            );
            assert_eq!(range.upper_bound().to_string(), "1", "{element_kind}");
            assert_eq!(
                value_specifications::lower_bound(&references, element)?.to_string(),
                expected_lower
            );
            assert_eq!(
                value_specifications::upper_bound(&references, element)?.to_string(),
                "1"
            );
        }
    }
    Ok(())
}

#[test]
fn parameter_uses_inherited_invariants_and_parameter_diagnostic_codes() -> Result {
    let mut p = document(true)?;
    kind(&mut p, "property", "uml.parameter")?;
    p.elements.push(serde_json::from_value(
        json!({"id":"operation","kind":"uml.operation","name":"Operation"}),
    )?);
    p.relations.push(serde_json::from_value(
        json!({"id":"owns-operation","kind":"owns","from":"class","to":"operation"}),
    )?);
    p.relations
        .iter_mut()
        .find(|relation| relation.id == "owns-property")
        .ok_or("parameter ownership")?
        .from = "operation".into();
    assert!(issues(&p)?.is_empty());
    author(&mut p, "lower", "value", json!(2))?;
    expect(&p, "UML_PARAMETER_MULTIPLICITY")?;
    author(&mut p, "lower", "value", json!(0))?;
    kind(&mut p, "upper", "uml.future_value")?;
    expect(&p, "MDE_PARAMETER_MULTIPLICITY_UNSUPPORTED")?;
    author(&mut p, "lower", "value", json!(-1))?;
    expect(&p, "UML_PARAMETER_MULTIPLICITY")?;
    Ok(())
}

#[test]
fn extension_end_explicit_bounds_obey_inherited_value_invariants() -> Result {
    use bellatrix_mde::uml::{
        references::ReferenceIndex,
        root::Model,
        value_specifications::{self, EvaluationError},
    };
    let mut p = document(false)?;
    kind(&mut p, "property", "uml.extension_end")?;
    for (lower, upper, valid) in [(0, 1, true), (1, 1, true), (2, 1, false), (-1, 1, false)] {
        author(&mut p, "lower", "value", json!(lower))?;
        author(&mut p, "upper", "value", json!(upper))?;
        let references = ReferenceIndex::new(Model::from_project(&p));
        let outcome =
            value_specifications::multiplicity(&references, references.element("property")?);
        if valid {
            assert!(outcome.is_ok(), "{outcome:?}");
        } else {
            assert!(
                matches!(outcome, Err(EvaluationError::Invalid { .. })),
                "{outcome:?}"
            );
        }
    }
    Ok(())
}

fn expression_guarantee_issues(project: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"].as_array().ok_or("diagnostics")?.iter()
        .filter(|issue| issue["subject_id"] == "property" && issue["message"].as_str()
            .is_some_and(|message| message.starts_with("MultiplicityElement-value_specification_")))
        .cloned().collect())
}

#[test]
fn multiplicity_constancy_and_purity_are_vacuous_for_absent_or_literal_bounds() -> Result {
    for rooted in [false, true] {
        let mut project = document(rooted)?;
        assert!(expression_guarantee_issues(&project)?.is_empty());
        // LiteralNull is not a legal numeric bound, but is still literal:
        // these two implications must not be confused with numeric constraints.
        kind(&mut project, "lower", "uml.literal_null")?;
        assert!(expression_guarantee_issues(&project)?.is_empty());
        expect(&project, INVALID)?;
        for key in ["lowerValue", "upperValue"] {
            author(&mut project, "property", key, Value::Null)?;
        }
        assert!(expression_guarantee_issues(&project)?.is_empty());
    }
    Ok(())
}

#[test]
fn nonliteral_multiplicity_never_claims_constancy_or_purity_from_expression_text() -> Result {
    for rooted in [false, true] {
        for bound in ["lower", "upper"] {
            for expression_kind in ["uml.opaque_expression", "uml.expression", "uml.future_value"] {
                let mut project = document(rooted)?;
                kind(&mut project, bound, expression_kind)?;
                author(&mut project, bound, "body", json!(["1"]))?;
                author(&mut project, bound, "language", json!(["OCL"]))?;
                let found = expression_guarantee_issues(&project)?;
                assert_eq!(found.len(), 2, "{found:#?}");
                for (issue, code) in found.iter().zip([
                    "MDE_MULTIPLICITY_CONSTANT_UNVERIFIED",
                    "MDE_MULTIPLICITY_NO_SIDE_EFFECTS_UNVERIFIED",
                ]) {
                    assert_eq!(issue["code"], code, "{found:#?}");
                    assert_eq!(issue["severity"], "warning", "{found:#?}");
                }
                for format in [Format::Json, Format::Protobuf] {
                    assert_eq!(expression_guarantee_issues(&decode(&encode(&project, format)?, format)?)?, found);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn invalid_bound_contract_dominates_unknown_expression_guarantees_in_either_order() -> Result {
    for unknown_lower in [false, true] {
        let mut project = document(false)?;
        let (unknown_key, invalid_key) = if unknown_lower {
            ("lowerValue", "upperValue")
        } else {
            ("upperValue", "lowerValue")
        };
        author(&mut project, "property", unknown_key, json!({"href":"external.uml#Bound"}))?;
        let unknown = expression_guarantee_issues(&project)?;
        assert_eq!(unknown.len(), 2);
        assert_eq!(unknown[0]["code"], "MDE_MULTIPLICITY_CONSTANT_UNVERIFIED");
        for malformed in [json!(42), json!({"$ref":"missing"}), json!({"$ref":"class"}), json!({"href":42})] {
            author(&mut project, "property", invalid_key, malformed)?;
            let found = expression_guarantee_issues(&project)?;
            assert_eq!(found.len(), 2, "{found:#?}");
            assert_eq!(found[0]["code"], "MDE_MULTIPLICITY_CONSTANT_INVALID", "{found:#?}");
            assert_eq!(found[1]["code"], "MDE_MULTIPLICITY_NO_SIDE_EFFECTS_INVALID", "{found:#?}");
            assert!(found.iter().all(|issue| issue["severity"] == "error"), "{found:#?}");
        }
    }
    Ok(())
}

#[test]
fn multiplicity_expression_guarantees_recover_after_draft_repair() -> Result {
    let mut project = document(true)?;
    kind(&mut project, "upper", "uml.opaque_expression")?;
    let before = project.clone();
    assert_eq!(expression_guarantee_issues(&project)?.len(), 2);
    assert_eq!(project.elements, before.elements);
    assert_eq!(project.relations, before.relations);
    kind(&mut project, "upper", "uml.literal_unlimited_natural")?;
    assert!(expression_guarantee_issues(&project)?.is_empty());
    for format in [Format::Json, Format::Protobuf] {
        assert!(expression_guarantee_issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
    }
    Ok(())
}

#[test]
fn parameter_and_extension_end_inherit_nonliteral_bound_guarantees() -> Result {
    for element_kind in ["uml.parameter", "uml.extension_end"] {
        for rooted in [false, true] {
            let mut project = document(rooted)?;
            kind(&mut project, "property", element_kind)?;
            assert!(expression_guarantee_issues(&project)?.is_empty());
            kind(&mut project, "lower", "uml.opaque_expression")?;
            let found = expression_guarantee_issues(&project)?;
            assert_eq!(found.len(), 2, "{element_kind}: {found:#?}");
            assert!(found.iter().all(|issue| issue["severity"] == "warning"));
            project.relations.iter_mut().find(|relation| relation.id == "owns-upper")
                .ok_or("upper ownership")?.from = "class".into();
            let invalid = expression_guarantee_issues(&project)?;
            assert_eq!(invalid.len(), 2, "{element_kind}: {invalid:#?}");
            assert!(invalid.iter().all(|issue| issue["severity"] == "error"));
            assert_eq!(invalid[0]["code"], "MDE_MULTIPLICITY_CONSTANT_INVALID");
            assert_eq!(invalid[1]["code"], "MDE_MULTIPLICITY_NO_SIDE_EFFECTS_INVALID");
        }
    }
    Ok(())
}
