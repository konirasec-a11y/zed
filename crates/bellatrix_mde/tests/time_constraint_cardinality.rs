use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn codes(kind: &str) -> (&'static str, &'static str) {
    if kind == "uml.time_constraint" {
        (
            "UML_TIME_CONSTRAINT_CONSTRAINED_ELEMENTS",
            "MDE_TIME_CONSTRAINT_CONSTRAINED_ELEMENTS_UNVERIFIED",
        )
    } else {
        (
            "UML_DURATION_CONSTRAINT_CONSTRAINED_ELEMENTS",
            "MDE_DURATION_CONSTRAINT_CONSTRAINED_ELEMENTS_UNVERIFIED",
        )
    }
}
fn document(kind: &str, values: Option<Value>) -> Result<Project> {
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Time constraints","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"constraint","kind":kind,"name":"Constraint"},
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
    if let Some(values) = values {
        author(&mut model, values)?;
    }
    Ok(model)
}
fn author(model: &mut Project, values: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "constraint")
        .ok_or("constraint")?
        .properties
        .insert("constrainedElement".into(), values);
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
fn given_known_counts_then_exact_normative_ranges_survive_roundtrip() -> Result {
    for kind in ["uml.time_constraint", "uml.duration_constraint"] {
        let (error, unknown) = codes(kind);
        for count in 0..=3 {
            let values = ["first", "second", "third"]
                .into_iter()
                .take(count)
                .map(|id| json!({"$ref":id}))
                .collect::<Vec<_>>();
            let model = document(kind, Some(json!(values)))?;
            let valid = count == 1 || (kind == "uml.duration_constraint" && count == 2);
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&model, format)?, format)?;
                assert_eq!(has(&reopened, error)?, !valid, "{kind} count={count}");
                assert!(!has(&reopened, unknown)?);
            }
        }
        let absent = document(kind, None)?;
        assert!(has(&absent, error)? && !has(&absent, unknown)?);
    }
    Ok(())
}
#[test]
fn given_external_identity_at_permitted_count_then_unknown_is_not_a_pass() -> Result {
    for kind in ["uml.time_constraint", "uml.duration_constraint"] {
        let (error, unknown) = codes(kind);
        let model = document(kind, Some(json!([{"href":"external.uml#Element"}])))?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&reopened, unknown)? && !has(&reopened, error)?);
        }
    }
    Ok(())
}

#[test]
fn given_deletion_leaves_dangling_constrained_reference_then_command_rejects_atomically() -> Result
{
    for kind in ["uml.time_constraint", "uml.duration_constraint"] {
        let model = document(kind, Some(json!([{"$ref":"first"}])))?;
        let before = encode(&model, Format::Json)?;
        let error = apply(
            &model,
            &Command::DeleteElement {
                element_id: "first".into(),
            },
            1,
        )
        .expect_err("required constrained element");
        assert!(error.to_string().contains(codes(kind).0), "{error}");
        assert_eq!(before, encode(&model, Format::Json)?);
    }
    Ok(())
}

#[test]
fn given_repaired_reference_collection_then_unreferenced_element_can_be_deleted() -> Result {
    for kind in ["uml.time_constraint", "uml.duration_constraint"] {
        let mut model = document(kind, Some(json!([{"$ref":"first"},{"$ref":"second"}])))?;
        // DeleteElement does not silently rewrite Constraint reference lists.
        // Repair the authored collection before deleting its former target.
        author(&mut model, json!([{"$ref":"second"}]))?;
        let changed = apply(
            &model,
            &Command::DeleteElement {
                element_id: "first".into(),
            },
            1,
        )?;
        let (error, unknown) = codes(kind);
        assert!(!has(&changed, error)? && !has(&changed, unknown)?);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&changed, format)?, format)?;
            assert!(!has(&reopened, error)? && !has(&reopened, unknown)?);
        }
    }
    Ok(())
}

#[test]
fn given_one_local_and_one_external_then_time_is_invalid_but_duration_is_unknown() -> Result {
    for kind in ["uml.time_constraint", "uml.duration_constraint"] {
        let (error, unknown) = codes(kind);
        for reverse in [false, true] {
            let mut values = vec![
                json!({"$ref":"first"}),
                json!({"href":"external.uml#Element"}),
            ];
            if reverse {
                values.reverse();
            }
            let model = document(kind, Some(json!(values)))?;
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&model, format)?, format)?;
                assert_eq!(has(&reopened, error)?, kind == "uml.time_constraint");
                assert_eq!(has(&reopened, unknown)?, kind == "uml.duration_constraint");
            }
        }
    }
    Ok(())
}
