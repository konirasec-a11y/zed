use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const KINDS: [&str; 2] = ["uml.duration", "uml.time_expression"];
fn codes(kind: &str) -> (&'static str, &'static str) {
    if kind == "uml.duration" {
        (
            "UML_DURATION_OBSERVATION_REQUIRED",
            "MDE_DURATION_OBSERVATION_REQUIRED_UNVERIFIED",
        )
    } else {
        (
            "UML_TIME_EXPRESSION_OBSERVATION_REQUIRED",
            "MDE_TIME_EXPRESSION_OBSERVATION_REQUIRED_UNVERIFIED",
        )
    }
}
fn document(kind: &str, observations: Option<Value>) -> Result<Project> {
    let observation_kind = if kind == "uml.duration" {
        "uml.duration_observation"
    } else {
        "uml.time_observation"
    };
    let event = if kind == "uml.duration" {
        json!([{"$ref":"target"}])
    } else {
        json!({"$ref":"target"})
    };
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Temporal expressions","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"temporal","kind":kind,"name":"Temporal"},
            {"id":"observation","kind":observation_kind,"name":"Observation","properties":{"event":event}},
            {"id":"other","kind":observation_kind,"name":"Other","properties":{"event":event}},
            {"id":"target","kind":"uml.class","name":"Target"}
        ],"relations":[
            {"id":"owns-temporal","kind":"owns","from":"root","to":"temporal"},
            {"id":"owns-observation","kind":"owns","from":"root","to":"observation"},
            {"id":"owns-other","kind":"owns","from":"root","to":"other"},
            {"id":"owns-target","kind":"owns","from":"root","to":"target"}
        ]}),
    )?;
    if let Some(observations) = observations {
        author(&mut model, "observation", observations)?;
    }
    Ok(model)
}
fn author(model: &mut Project, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "temporal")
        .ok_or("temporal")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn owned_expr(model: &mut Project, kind: &str, explicit: bool) -> Result {
    model.elements.push(serde_json::from_value(
        json!({"id":"expr","kind":kind,"name":"Expr","properties":{"visibility":null}}),
    )?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-expr","kind":"owns","from":"temporal","to":"expr"}),
    )?);
    if explicit {
        author(model, "expr", json!({"$ref":"expr"}))?;
    }
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
        .any(|issue| issue["subject_id"] == "temporal" && issue["code"] == code))
}
#[test]
fn given_no_expr_then_exactly_one_correct_observation_is_required_after_roundtrip() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        for observations in [
            None,
            Some(json!([])),
            Some(json!([{"$ref":"observation"}])),
            Some(json!([{"$ref":"observation"},{"$ref":"other"}])),
        ] {
            let valid = observations
                .as_ref()
                .and_then(Value::as_array)
                .is_some_and(|values| values.len() == 1);
            let mut model = document(kind, observations)?;
            for explicit_null in [false, true] {
                if explicit_null {
                    author(&mut model, "expr", Value::Null)?;
                }
                for format in [Format::Json, Format::Protobuf] {
                    let reopened = decode(&encode(&model, format)?, format)?;
                    assert_eq!(has(&reopened, error)?, !valid);
                    assert!(!has(&reopened, unknown)?);
                }
            }
        }
    }
    Ok(())
}
#[test]
fn given_present_expr_then_observation_subtype_and_count_are_exempt() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        for explicit in [false, true] {
            let mut model = document(kind, None)?;
            owned_expr(&mut model, "uml.literal_integer", explicit)?;
            assert!(!has(&model, error)? && !has(&model, unknown)?);
            author(
                &mut model,
                "observation",
                json!([{"href":"external.uml#Observation"}]),
            )?;
            assert!(!has(&model, error)? && !has(&model, unknown)?);
            model
                .elements
                .iter_mut()
                .find(|element| element.id == "observation")
                .ok_or("observation")?
                .kind = "uml.observation".into();
            author(
                &mut model,
                "observation",
                json!([{"$ref":"observation"},{"$ref":"other"}]),
            )?;
            assert!(!has(&model, error)? && !has(&model, unknown)?);
        }
    }
    Ok(())
}
#[test]
fn given_external_expr_then_valid_consequent_decides_but_false_consequent_stays_unknown() -> Result
{
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        let mut model = document(kind, None)?;
        author(&mut model, "expr", json!({"href":"external.uml#Value"}))?;
        assert!(has(&model, unknown)? && !has(&model, error)?);
        author(&mut model, "observation", json!([{"$ref":"observation"}]))?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, error)? && !has(&reopened, unknown)?);
        }
    }
    Ok(())
}
#[test]
fn given_unknown_observation_then_known_count_violation_wins_in_both_orders() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        let mut model = document(kind, Some(json!([{"href":"external.uml#Observation"}])))?;
        assert!(has(&model, unknown)? && !has(&model, error)?);
        for reverse in [false, true] {
            let mut refs = vec![
                json!({"href":"external.uml#Observation"}),
                json!({"$ref":"observation"}),
            ];
            if reverse {
                refs.reverse();
            }
            author(&mut model, "observation", json!(refs))?;
            assert!(has(&model, error)? && !has(&model, unknown)?);
        }
    }
    Ok(())
}
#[test]
fn given_wrong_or_unknown_observation_metaclass_then_subtype_is_not_assumed() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        for target_kind in ["uml.observation", "future.observation", "uml.class"] {
            let mut model = document(kind, Some(json!([{"$ref":"observation"}])))?;
            model
                .elements
                .iter_mut()
                .find(|element| element.id == "observation")
                .ok_or("observation")?
                .kind = target_kind.into();
            assert_eq!(has(&model, unknown)?, target_kind == "future.observation");
            assert_eq!(has(&model, error)?, target_kind != "future.observation");
        }
    }
    Ok(())
}
#[test]
fn given_malformed_evidence_then_invalid_dominates_either_true_branch() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        for invalid in [
            json!(false),
            json!({"href":false}),
            json!({"$ref":"missing"}),
            json!({"$ref":"target"}),
        ] {
            let mut model = document(kind, Some(json!([{"$ref":"observation"}])))?;
            author(&mut model, "expr", invalid)?;
            assert!(has(&model, error)? && !has(&model, unknown)?);
        }
        for invalid in [
            json!(null),
            json!([false]),
            json!([{"$ref":"missing"}]),
            json!([{"$ref":"observation"},{"$ref":"observation"}]),
        ] {
            let mut model = document(kind, Some(invalid))?;
            owned_expr(&mut model, "uml.literal_integer", true)?;
            assert!(has(&model, error)? && !has(&model, unknown)?);
        }
    }
    Ok(())
}
#[test]
fn given_ambiguous_owned_string_expression_then_role_stays_unknown_until_named() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        let mut model = document(kind, None)?;
        owned_expr(&mut model, "uml.string_expression", false)?;
        assert!(has(&model, unknown)? && !has(&model, error)?);
        author(&mut model, "nameExpression", json!({"$ref":"expr"}))?;
        assert!(has(&model, error)? && !has(&model, unknown)?);
        author(&mut model, "expr", json!({"$ref":"expr"}))?;
        assert!(!has(&model, error)? && !has(&model, unknown)?);
    }
    Ok(())
}
#[test]
fn given_expr_owned_elsewhere_then_reference_cannot_override_canonical_ownership() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        let mut model = document(kind, Some(json!([{"$ref":"observation"}])))?;
        owned_expr(&mut model, "uml.literal_integer", true)?;
        model
            .relations
            .iter_mut()
            .find(|relation| relation.id == "owns-expr")
            .ok_or("owner")?
            .from = "root".into();
        assert!(has(&model, error)? && !has(&model, unknown)?);
    }
    Ok(())
}
#[test]
fn given_sole_expr_deleted_then_observation_requirement_rejects_atomically() -> Result {
    for kind in KINDS {
        let mut model = document(kind, None)?;
        owned_expr(&mut model, "uml.literal_integer", false)?;
        let before = encode(&model, Format::Json)?;
        let error = apply(
            &model,
            &Command::DeleteElement {
                element_id: "expr".into(),
            },
            1,
        )
        .expect_err("observation required when expr disappears");
        assert!(error.to_string().contains(codes(kind).0), "{error}");
        assert_eq!(before, encode(&model, Format::Json)?);
    }
    Ok(())
}

#[test]
fn given_two_inferred_exprs_then_scalar_role_is_invalid_even_with_correct_observation() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        for observations in [None, Some(json!([{"$ref":"observation"}]))] {
            let mut model = document(kind, observations)?;
            owned_expr(&mut model, "uml.literal_integer", false)?;
            model.elements.push(serde_json::from_value(json!({"id":"expr2","kind":"uml.literal_real","name":"Second expr","properties":{"visibility":null}}))?);
            model.relations.push(serde_json::from_value(
                json!({"id":"owns-expr2","kind":"owns","from":"temporal","to":"expr2"}),
            )?);
            assert!(has(&model, error)? && !has(&model, unknown)?);
        }
    }
    Ok(())
}

#[test]
fn given_other_temporal_observation_subtype_then_the_required_subtype_is_enforced() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        let mut model = document(kind, Some(json!([{"$ref":"observation"}])))?;
        let observation = model
            .elements
            .iter_mut()
            .find(|element| element.id == "observation")
            .ok_or("observation")?;
        if kind == "uml.duration" {
            observation.kind = "uml.time_observation".into();
            observation
                .properties
                .insert("event".into(), json!({"$ref":"target"}));
        } else {
            observation.kind = "uml.duration_observation".into();
            observation
                .properties
                .insert("event".into(), json!([{"$ref":"target"}]));
        }
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&reopened, error)? && !has(&reopened, unknown)?);
        }
    }
    Ok(())
}

#[test]
fn given_unknown_local_expr_then_consequent_can_resolve_the_implication() -> Result {
    for kind in KINDS {
        let (error, unknown) = codes(kind);
        let mut model = document(kind, None)?;
        owned_expr(&mut model, "future.value", true)?;
        assert!(has(&model, unknown)? && !has(&model, error)?);
        author(&mut model, "observation", json!([{"$ref":"observation"}]))?;
        assert!(!has(&model, error)? && !has(&model, unknown)?);
    }
    Ok(())
}
