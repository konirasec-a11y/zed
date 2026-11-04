use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STRING_EXPRESSION_SUBEXPRESSIONS";
const UNKNOWN: &str = "MDE_STRING_EXPRESSION_SUBEXPRESSIONS_UNVERIFIED";
fn document() -> Result<Project> {
    Ok(serde_json::from_value(
        json!({"id":"model","name":"String expression roles","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root","properties":{"nameExpression":{"$ref":"expression"}}},
        {"id":"expression","kind":"uml.string_expression","name":"Expression","properties":{"visibility":null}}
    ],"relations":[{"id":"owns-expression","kind":"owns","from":"root","to":"expression"}]}),
    )?)
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
fn child(model: &mut Project, id: &str, kind: &str) -> Result {
    model.elements.push(serde_json::from_value(
        json!({"id":id,"kind":kind,"name":id,"properties":{"visibility":null}}),
    )?);
    model.relations.push(serde_json::from_value(
        json!({"id":format!("owns-{id}"),"kind":"owns","from":"expression","to":id}),
    )?);
    if kind == "uml.string_expression" {
        let literal = format!("{id}-literal");
        model.elements.push(serde_json::from_value(json!({"id":literal,"kind":"uml.literal_string","name":"Leaf","properties":{"visibility":null}}))?);
        model.relations.push(serde_json::from_value(
            json!({"id":format!("owns-{literal}"),"kind":"owns","from":id,"to":literal}),
        )?);
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
        .any(|issue| issue["subject_id"] == "expression" && issue["code"] == code))
}
fn errors(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| issue["severity"] == "error")
        .cloned()
        .collect())
}
#[test]
fn given_both_collections_empty_then_published_else_branch_rejects() -> Result {
    for explicit in [false, true] {
        let mut model = document()?;
        if explicit {
            author(&mut model, "operand", json!([]))?;
            author(&mut model, "subExpression", json!([]))?;
        }
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}
#[test]
fn given_exactly_one_collection_then_explicit_or_canonical_operand_passes() -> Result {
    for operand in [false, true] {
        for explicit in [false, true] {
            let mut model = document()?;
            if operand {
                child(&mut model, "item", "uml.literal_string")?;
                if explicit {
                    author(&mut model, "operand", json!([{"$ref":"item"}]))?;
                }
            } else {
                child(&mut model, "item", "uml.string_expression")?;
                author(&mut model, "subExpression", json!([{"$ref":"item"}]))?;
                if explicit {
                    author(&mut model, "operand", json!([]))?;
                }
            }
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&model, format)?, format)?;
                assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
                let issues = errors(&reopened)?;
                assert!(issues.is_empty(), "{issues:#?}");
            }
        }
    }
    Ok(())
}

#[test]
fn given_only_external_collection_then_unknown_not_empty_or_valid() -> Result {
    for key in ["operand", "subExpression"] {
        let mut model = document()?;
        author(&mut model, key, json!([{"href":"external.uml#Value"}]))?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&reopened, UNKNOWN)? && !has(&reopened, ERROR)?);
        }
    }
    Ok(())
}

#[test]
fn given_sole_operand_deleted_then_empty_expression_is_rejected_atomically() -> Result {
    let mut model = document()?;
    child(&mut model, "literal", "uml.literal_string")?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::DeleteElement {
            element_id: "literal".into(),
        },
        1,
    )
    .expect_err("empty StringExpression");
    assert!(error.to_string().contains(ERROR), "{error}");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}

#[test]
fn given_explicit_empty_collections_then_unknown_owned_child_cannot_hide_the_violation() -> Result {
    let mut model = document()?;
    child(&mut model, "unknown", "future.value")?;
    author(&mut model, "operand", json!([]))?;
    author(&mut model, "subExpression", json!([]))?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert!(has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
    }
    Ok(())
}
