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
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&repaired, format)?, format)?;
        assert_eq!(reopened, repaired);
        assert!(issues(&reopened)?.is_empty());
    }
    Ok(())
}
