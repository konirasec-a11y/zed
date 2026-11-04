use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_OPERATION_AT_MOST_ONE_RETURN";
const INVALID: &str = "MDE_OPERATION_AT_MOST_ONE_RETURN_INVALID";
const UNKNOWN: &str = "MDE_OPERATION_AT_MOST_ONE_RETURN_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Return count","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"operation","kind":"uml.operation","name":"Operation"},
        {"id":"first","kind":"uml.parameter","name":"First","properties":{"type":{"$ref":"class"},"direction":"return"}},
        {"id":"second","kind":"uml.parameter","name":"Second","properties":{"type":{"$ref":"class"}}}
    ],"relations":[]});
    for (owner, child) in [
        ("root", "class"),
        ("class", "operation"),
        ("operation", "first"),
        ("operation", "second"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
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
fn issues(model: &Project) -> Result<Vec<Value>> {
    let result: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(result["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "operation"
                && [ERROR, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| issue["code"] == *code)
        })
        .cloned()
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
fn only_return_direction_counts_and_missing_direction_defaults_to_in() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        assert!(issues(&model)?.is_empty());
        for direction in ["in", "out", "inout"] {
            author(&mut model, "second", "direction", json!(direction))?;
            assert!(issues(&model)?.is_empty());
        }
        author(&mut model, "second", "direction", json!("return"))?;
        expect(&model, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
        author(&mut model, "first", "direction", json!("in"))?;
        assert!(issues(&model)?.is_empty());
        author(&mut model, "second", "direction", json!("in"))?;
        assert!(issues(&model)?.is_empty());
    }
    Ok(())
}

#[test]
fn second_return_edit_is_rejected_atomically_and_repair_succeeds() -> Result {
    let model = document(true)?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "second".into(),
            path: "properties.direction".into(),
            value: json!("return"),
        },
        1,
    )
    .expect_err("only one return");
    assert!(
        error.contains("at_most_one_return") || error.contains(ERROR),
        "{error}"
    );
    assert_eq!(encode(&model, Format::Json)?, before);
    let mut invalid = model;
    author(&mut invalid, "second", "direction", json!("return"))?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "second".into(),
            path: "properties.direction".into(),
            value: json!("out"),
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
