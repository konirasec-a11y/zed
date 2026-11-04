use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STRING_EXPRESSION_OPERANDS";
const UNKNOWN: &str = "MDE_STRING_EXPRESSION_OPERANDS_UNVERIFIED";
fn document(kind: &str, explicit: bool) -> Result<Project> {
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Strings","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root","properties":{"nameExpression":{"$ref":"expression"}}},
            {"id":"expression","kind":"uml.string_expression","name":"Expression","properties":{"visibility":null}},
            {"id":"operand","kind":kind,"name":"Operand","properties":{"visibility":null}}
        ],"relations":[
            {"id":"owns-expression","kind":"owns","from":"root","to":"expression"},
            {"id":"owns-operand","kind":"owns","from":"expression","to":"operand"}
        ]}),
    )?;
    if explicit {
        author(&mut model, "operand", json!([{"$ref":"operand"}]))?;
    }
    Ok(model)
}
fn author(model: &mut Project, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "expression")
        .ok_or("expression")?
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
        .any(|issue| issue["subject_id"] == "expression" && issue["code"] == code))
}
#[test]
fn given_literal_string_operand_then_explicit_and_inferred_roles_pass_after_roundtrip() -> Result {
    for explicit in [false, true] {
        let model = document("uml.literal_string", explicit)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}
#[test]
fn given_known_non_string_operand_then_literal_text_or_name_does_not_establish_literal_string()
-> Result {
    for kind in [
        "uml.literal_integer",
        "uml.opaque_expression",
        "uml.expression",
    ] {
        for explicit in [false, true] {
            let model = document(kind, explicit)?;
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&model, format)?, format)?;
                assert!(
                    has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?,
                    "{kind}, explicit={explicit}"
                );
            }
        }
    }
    Ok(())
}
#[test]
fn given_unknown_operand_then_warning_survives_roundtrip() -> Result {
    for kind in ["uml.literal_string", "future.value"] {
        let mut model = document(kind, true)?;
        if kind == "uml.literal_string" {
            author(
                &mut model,
                "operand",
                json!([{"$ref":"operand"},{"href":"external.uml#Value"}]),
            )?;
        }
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&reopened, UNKNOWN)? && !has(&reopened, ERROR)?);
        }
    }
    Ok(())
}

#[test]
fn given_non_string_reparented_as_operand_then_command_rejects_atomically() -> Result {
    let mut model = document("uml.literal_integer", false)?;
    model.elements.push(serde_json::from_value(
        json!({"id":"source","kind":"uml.expression","name":"Source"}),
    )?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-source","kind":"owns","from":"root","to":"source"}),
    )?);
    model
        .relations
        .iter_mut()
        .find(|relation| relation.id == "owns-operand")
        .ok_or("owner")?
        .from = "source".into();
    assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetOwner {
            element_id: "operand".into(),
            owner_id: Some("expression".into()),
        },
        1,
    )
    .expect_err("LiteralInteger is not a LiteralString");
    assert!(error.to_string().contains(ERROR), "{error}");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}
