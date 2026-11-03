use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_OPERATION_ONLY_BODY_FOR_QUERY";
const INVALID: &str = "MDE_OPERATION_ONLY_BODY_FOR_QUERY_INVALID";
const UNKNOWN: &str = "MDE_OPERATION_ONLY_BODY_FOR_QUERY_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Operation condition","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"operation","kind":"uml.operation","name":"Operation","properties":{"bodyCondition":{"$ref":"condition"},"isQuery":true}},
        {"id":"condition","kind":"uml.constraint","name":"Condition"}
    ],"relations":[
        {"id":"owns-class","kind":"owns","from":"root","to":"class"},
        {"id":"owns-operation","kind":"owns","from":"class","to":"operation"},
        {"id":"owns-condition","kind":"owns","from":"operation","to":"condition"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("model")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
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
fn diagnostics(model: &Project) -> Result<Vec<Value>> {
    let result: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(result["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| issue["subject_id"] == "operation")
        .cloned()
        .collect())
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    Ok(diagnostics(model)?
        .into_iter()
        .filter(|issue| {
            [ERROR, INVALID, UNKNOWN]
                .iter()
                .any(|code| issue["code"] == *code)
        })
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
fn a_body_condition_requires_query_true_with_false_as_the_default() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        assert!(issues(&model)?.is_empty());
        author(&mut model, "operation", "isQuery", json!(false))?;
        expect(&model, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
        model
            .elements
            .iter_mut()
            .find(|element| element.id == "operation")
            .ok_or("operation")?
            .properties
            .remove("isQuery");
        expect(&model, ERROR)?;
    }
    Ok(())
}
#[test]
fn null_or_absent_body_condition_is_vacuous_and_owned_constraint_does_not_invent_role() -> Result {
    let mut model = document(true)?;
    author(&mut model, "operation", "isQuery", json!(false))?;
    author(&mut model, "operation", "bodyCondition", Value::Null)?;
    assert!(issues(&model)?.is_empty());
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "operation")
        .ok_or("operation")?
        .properties
        .remove("bodyCondition");
    assert!(issues(&model)?.is_empty());
    Ok(())
}
#[test]
fn external_body_presence_requires_query_without_claiming_resolved_contract() -> Result {
    let mut model = document(true)?;
    author(
        &mut model,
        "operation",
        "bodyCondition",
        json!({"href":"external.uml#Condition"}),
    )?;
    assert!(issues(&model)?.is_empty());
    let found = diagnostics(&model)?;
    assert!(
        found.iter().any(
            |issue| issue["code"] == "MDE_OPERATION_BODY_CONDITION_UNVERIFIED"
                && issue["severity"] == "warning"
        ),
        "{found:#?}"
    );
    assert!(
        !found
            .iter()
            .any(|issue| issue["code"] == "UML_OPERATION_BODY_CONDITION"),
        "{found:#?}"
    );
    author(&mut model, "operation", "isQuery", json!(false))?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn unknown_local_constraint_metaclass_does_not_become_invalid_when_query_is_true() -> Result {
    let mut model = document(true)?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "condition")
        .ok_or("condition")?
        .kind = "uml.unknown_constraint".into();
    assert!(issues(&model)?.is_empty());
    let found = diagnostics(&model)?;
    assert!(
        found.iter().any(
            |issue| issue["code"] == "MDE_OPERATION_BODY_CONDITION_UNVERIFIED"
                && issue["severity"] == "warning"
        ),
        "{found:#?}"
    );
    author(&mut model, "operation", "isQuery", json!(false))?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn malformed_reference_or_wrong_ownership_is_invalid_even_for_nonquery() -> Result {
    for query in [false, true] {
        for value in [
            json!(42),
            json!([]),
            json!([{"$ref":"condition"}]),
            json!({"$ref":"missing"}),
            json!({"$ref":"class"}),
            json!({"href":false}),
            json!({"href":""}),
        ] {
            let mut model = document(true)?;
            author(&mut model, "operation", "isQuery", json!(query))?;
            author(&mut model, "operation", "bodyCondition", value)?;
            expect(&model, INVALID)?;
        }
        let mut model = document(true)?;
        author(&mut model, "operation", "isQuery", json!(query))?;
        model
            .relations
            .iter_mut()
            .find(|relation| relation.id == "owns-condition")
            .ok_or("owner")?
            .from = "class".into();
        expect(&model, INVALID)?;
    }
    Ok(())
}
#[test]
fn malformed_query_is_not_hidden_by_absent_null_or_external_body() -> Result {
    for body in [Value::Null, json!({"href":"external.uml#Condition"})] {
        for query in [Value::Null, json!(0), json!("true")] {
            let mut model = document(true)?;
            author(&mut model, "operation", "bodyCondition", body.clone())?;
            author(&mut model, "operation", "isQuery", query)?;
            expect(&model, INVALID)?;
        }
    }
    let mut model = document(true)?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "operation")
        .ok_or("operation")?
        .properties
        .remove("bodyCondition");
    author(&mut model, "operation", "isQuery", Value::Null)?;
    expect(&model, INVALID)?;

    Ok(())
}
#[test]
fn condition_expression_is_not_evaluated_by_the_query_only_invariant() -> Result {
    let mut model = document(true)?;
    author(
        &mut model,
        "condition",
        "specification",
        json!({"href":"external.uml#Expression"}),
    )?;
    assert!(issues(&model)?.is_empty());
    assert!(
        diagnostics(&model)?
            .iter()
            .any(|issue| issue["code"] == "MDE_OPERATION_CONDITION_UNSUPPORTED")
    );
    author(&mut model, "operation", "isQuery", json!(false))?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn nonquery_edit_is_rejected_atomically_and_imported_violation_can_be_repaired() -> Result {
    let model = document(true)?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "operation".into(),
            path: "properties.isQuery".into(),
            value: json!(false),
        },
        1,
    )
    .expect_err("bodyCondition requires query");
    assert!(
        error.contains("UML_OPERATION_BODY_CONDITION") || error.contains(ERROR),
        "{error}"
    );
    assert_eq!(encode(&model, Format::Json)?, before);
    let mut invalid = model;
    author(&mut invalid, "operation", "isQuery", json!(false))?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "operation".into(),
            path: "properties.isQuery".into(),
            value: json!(true),
        },
        2,
    )?;
    assert!(issues(&repaired)?.is_empty());
    Ok(())
}
