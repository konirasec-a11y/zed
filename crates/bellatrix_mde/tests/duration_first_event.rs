use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_DURATION_CONSTRAINT_FIRST_EVENT";
const UNKNOWN: &str = "MDE_DURATION_CONSTRAINT_FIRST_EVENT_UNVERIFIED";
fn document(constrained: Option<Value>, first_event: Option<Value>) -> Result<Project> {
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Duration first events","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"constraint","kind":"uml.duration_constraint","name":"Duration"},
            {"id":"first","kind":"uml.class","name":"First"},
            {"id":"second","kind":"uml.class","name":"Second"},
            {"id":"third","kind":"uml.class","name":"Third"}
        ],"relations":[
            {"id":"owns-constraint","kind":"owns","from":"root","to":"constraint"},
            {"id":"owns-first","kind":"owns","from":"root","to":"first"},
            {"id":"owns-second","kind":"owns","from":"root","to":"second"},
            {"id":"owns-third","kind":"owns","from":"root","to":"third"}
        ]}),
    )?;
    if let Some(value) = constrained {
        author(&mut model, "constrainedElement", value)?;
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
        .find(|element| element.id == "constraint")
        .ok_or("constraint")?
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
        .any(|issue| issue["subject_id"] == "constraint" && issue["code"] == code))
}
#[test]
fn given_known_cardinalities_then_exact_conditional_survives_roundtrip() -> Result {
    for count in 0..=3 {
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
    .expect_err("dangling constrainedElement");
    // The cardinality/reference gate runs before firstEvent; either path must
    // leave the source untouched rather than silently rewrite firstEvent.
    assert!(
        error
            .to_string()
            .contains("UML_DURATION_CONSTRAINT_CONSTRAINED_ELEMENTS"),
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
            element_id: "constraint".into(),
            path: "properties.firstEvent".into(),
            value: json!([true]),
        },
        1,
    )
    .expect_err("one firstEvent cannot describe two constrained elements");
    // This proves the authoring boundary, not rollback by FIRST_EVENT: the
    // array remains read-only and the command never reaches that validator.
    assert_eq!(error.to_string(), "property is read-only");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}
