use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_DURATION_OBSERVATION_FIRST_EVENT";
const UNKNOWN: &str = "MDE_DURATION_OBSERVATION_FIRST_EVENT_UNVERIFIED";
fn document(events: Option<Value>, first_event: Option<Value>) -> Result<Project> {
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Duration first events","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"observation","kind":"uml.duration_observation","name":"Duration"},
            {"id":"first","kind":"uml.class","name":"First"},
            {"id":"second","kind":"uml.class","name":"Second"},
            {"id":"third","kind":"uml.class","name":"Third"}
        ],"relations":[
            {"id":"owns-constraint","kind":"owns","from":"root","to":"observation"},
            {"id":"owns-first","kind":"owns","from":"root","to":"first"},
            {"id":"owns-second","kind":"owns","from":"root","to":"second"},
            {"id":"owns-third","kind":"owns","from":"root","to":"third"}
        ]}),
    )?;
    if let Some(value) = events {
        author(&mut model, "event", value)?;
    }
    if let Some(value) = first_event {
        author(&mut model, "firstEvent", value)?;
    }
    Ok(model)
}
fn author(model: &mut Project, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "observation")
        .ok_or("observation")?
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
        .any(|issue| issue["subject_id"] == "observation" && issue["code"] == code))
}
#[test]
fn given_known_cardinalities_then_exact_conditional_survives_roundtrip() -> Result {
    for count in 1..=2 {
        for first_event in [json!([]), json!([true]), json!([true, false])] {
            let elements = ["first", "second", "third"]
                .into_iter()
                .take(count)
                .map(|id| json!({"$ref":id}))
                .collect::<Vec<_>>();
            let expected = if count == 2 { 2 } else { 0 };
            let valid = first_event.as_array().ok_or("array")?.len() == expected;
            let model = document(Some(json!(elements)), Some(first_event))?;
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&model, format)?, format)?;
                assert_eq!(has(&reopened, ERROR)?, !valid, "count={count}");
                assert!(!has(&reopened, UNKNOWN)?);
            }
        }
    }
    Ok(())
}
#[test]
fn given_required_event_absent_then_unknown_but_explicit_empty_is_invalid() -> Result {
    for first_event in [None, Some(json!([])), Some(json!([true, false]))] {
        let model = document(None, first_event.clone())?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&reopened, UNKNOWN)? && !has(&reopened, ERROR)?);
        }
        let empty = document(Some(json!([])), first_event)?;
        assert!(has(&empty, ERROR)? && !has(&empty, UNKNOWN)?);
    }
    let model = document(None, Some(json!([true])))?;
    assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    Ok(())
}
#[test]
fn given_external_reference_then_known_mismatch_wins_but_matching_count_stays_unknown() -> Result {
    for count in [1, 2] {
        for reverse in [false, true] {
            let mut elements = vec![json!({"href":"external.uml#Element"})];
            elements.extend(
                ["first", "second"]
                    .into_iter()
                    .take(count - 1)
                    .map(|id| json!({"$ref":id})),
            );
            if reverse {
                elements.reverse();
            }
            let expected = if count == 2 {
                json!([true, false])
            } else {
                json!([])
            };
            let mut model = document(Some(json!(elements)), Some(expected))?;
            assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
            author(&mut model, "firstEvent", json!([true]))?;
            assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
        }
    }
    Ok(())
}
#[test]
fn given_invalid_boolean_shape_or_duplicate_then_invalid_dominates_external_unknown() -> Result {
    for first_event in [
        json!(true),
        json!(null),
        json!(["true", false]),
        json!([null, false]),
        json!([true, true]),
        json!([true, false, true]),
    ] {
        let model = document(
            Some(json!([{"$ref":"first"},{"href":"external.uml#Element"}])),
            Some(first_event),
        )?;
        assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn given_invalid_events_references_then_expected_first_event_count_cannot_hide_invalid() -> Result {
    for events in [
        json!(false),
        json!(null),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"first"},{"$ref":"first"}]),
        json!([{"$ref":"first"},{"$ref":"second"},{"$ref":"third"}]),
        json!([{"href":"external.uml#Element"},{"href":false}]),
    ] {
        for first_event in [json!([]), json!([true, false])] {
            let model = document(Some(events.clone()), Some(first_event))?;
            assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
        }
    }
    Ok(())
}
#[test]
fn given_valid_pair_when_target_is_deleted_then_rejection_preserves_first_event() -> Result {
    let model = document(
        Some(json!([{"$ref":"first"},{"$ref":"second"}])),
        Some(json!([true, false])),
    )?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::DeleteElement {
            element_id: "first".into(),
        },
        1,
    )
    .expect_err("dangling event");
    // A dangling event remains Invalid even though its stored count is unchanged.
    assert!(
        error
            .to_string()
            .contains("UML_DURATION_OBSERVATION_FIRST_EVENT"),
        "{error}"
    );
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}

#[test]
fn given_direct_first_event_edit_then_read_only_adapter_preserves_source() -> Result {
    let model = document(
        Some(json!([{"$ref":"first"},{"$ref":"second"}])),
        Some(json!([true, false])),
    )?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "observation".into(),
            path: "properties.firstEvent".into(),
            value: json!([true]),
        },
        1,
    )
    .expect_err("one firstEvent cannot describe two events elements");
    // This proves the authoring boundary, not rollback by FIRST_EVENT: the
    // array remains read-only and the command never reaches that validator.
    assert_eq!(error.to_string(), "property is read-only");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}

#[test]
fn given_unknown_or_non_named_local_event_then_reference_contract_is_preserved() -> Result {
    for kind in ["future.event", "uml.comment"] {
        let mut model = document(Some(json!([{"$ref":"first"}])), Some(json!([])))?;
        model
            .elements
            .iter_mut()
            .find(|element| element.id == "first")
            .ok_or("event")?
            .kind = kind.into();
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert_eq!(has(&reopened, UNKNOWN)?, kind == "future.event");
            assert_eq!(has(&reopened, ERROR)?, kind == "uml.comment");
        }
        author(&mut model, "firstEvent", json!([true]))?;
        assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    }
    Ok(())
}
