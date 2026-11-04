use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STEREOTYPE_GENERALIZATION";
const UNKNOWN: &str = "MDE_STEREOTYPE_GENERALIZATION_UNVERIFIED";
fn document() -> Result<Project> {
    Ok(serde_json::from_value(
        json!({"id":"model","name":"Stereotypes","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"specific","kind":"uml.stereotype","name":"Specific"},
            {"id":"middle","kind":"uml.stereotype","name":"Middle"},
            {"id":"general","kind":"uml.stereotype","name":"General"}
        ],"relations":[
            {"id":"owns-specific","kind":"owns","from":"root","to":"specific"},
            {"id":"owns-middle","kind":"owns","from":"root","to":"middle"},
            {"id":"owns-general","kind":"owns","from":"root","to":"general"}
        ]}),
    )?)
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
fn generalize(model: &mut Project, from: &str, to: &str) -> Result {
    model.relations.push(serde_json::from_value(json!({"id":format!("general-{from}-{to}"),"kind":"uml.generalization","from":from,"to":to}))?);
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
        .any(|issue| issue["code"] == code))
}
#[test]
fn given_complete_stereotype_chain_then_valid_after_both_roundtrips() -> Result {
    let mut model = document()?;
    generalize(&mut model, "specific", "middle")?;
    generalize(&mut model, "middle", "general")?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn given_direct_or_transitive_non_stereotype_then_both_directions_are_rejected() -> Result {
    for endpoint in ["specific", "general"] {
        for transitive in [false, true] {
            let mut model = document()?;
            model
                .elements
                .iter_mut()
                .find(|element| element.id == endpoint)
                .ok_or("endpoint")?
                .kind = "uml.class".into();
            if transitive {
                generalize(&mut model, "specific", "middle")?;
                generalize(&mut model, "middle", "general")?;
            } else {
                generalize(&mut model, "specific", "general")?;
            }
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&model, format)?, format)?;
                assert!(has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
            }
        }
    }
    Ok(())
}
#[test]
fn given_unknown_parent_then_unknown_without_inventing_a_non_stereotype() -> Result {
    let mut model = document()?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "general")
        .ok_or("general")?
        .kind = "future.classifier".into();
    generalize(&mut model, "specific", "general")?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert!(has(&reopened, UNKNOWN)? && !has(&reopened, ERROR)?);
    }
    Ok(())
}

#[test]
fn given_invalid_ancestry_edit_then_rejection_is_atomic() -> Result {
    let mut model = document()?;
    generalize(&mut model, "specific", "general")?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "general".into(),
            path: "properties.isFinalSpecialization".into(),
            value: json!(true),
        },
        1,
    )
    .expect_err("malformed ancestry");
    assert!(
        error.to_string().contains("Classifier::non_final_parents"),
        "{error}"
    );
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}

fn has_subject(model: &Project, subject: &str, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .any(|issue| issue["subject_id"] == subject && issue["code"] == code))
}

#[test]
fn given_bad_parents_of_one_stereotype_then_isolated_stereotype_remains_valid() -> Result {
    for reverse in [false, true] {
        let mut model = document()?;
        model
            .elements
            .iter_mut()
            .find(|element| element.id == "general")
            .ok_or("general")?
            .kind = "uml.class".into();
        generalize(&mut model, "specific", "general")?;
        if reverse {
            model.elements.reverse();
            model.relations.reverse();
        }
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has_subject(&reopened, "specific", ERROR)?);
            assert!(!has_subject(&reopened, "middle", ERROR)?);
            assert!(!has_subject(&reopened, "middle", UNKNOWN)?);
        }
    }
    Ok(())
}

#[test]
fn given_unknown_or_invalid_self_ancestry_then_other_stereotypes_do_not_inherit_it() -> Result {
    for value in [json!([{"href":"external.uml#Parent"}]), json!(false)] {
        let mut model = document()?;
        author(&mut model, "specific", "general", value.clone())?;
        let expected = if value.is_array() { UNKNOWN } else { ERROR };
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has_subject(&reopened, "specific", expected)?);
            assert!(!has_subject(&reopened, "middle", ERROR)?);
            assert!(!has_subject(&reopened, "middle", UNKNOWN)?);
        }
    }
    Ok(())
}
