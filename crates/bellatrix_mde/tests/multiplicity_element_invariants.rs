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
