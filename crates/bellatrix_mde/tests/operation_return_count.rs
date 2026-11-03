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
fn two_known_returns_dominate_external_projection_in_both_orders() -> Result {
    use bellatrix_mde::uml::{operations, references::ReferenceIndex, root::Model};
    for reverse in [false, true] {
        let mut model = document(true)?;
        let mut values = vec![
            json!({"$ref":"first"}),
            json!({"$ref":"second"}),
            json!({"href":"external.uml#Parameter"}),
        ];
        if reverse {
            values.reverse();
            model.relations.reverse();
        }
        author(&mut model, "operation", "ownedParameter", json!(values))?;
        expect(&model, UNKNOWN)?;
        let references = ReferenceIndex::new(Model::from_project(&model));
        assert!(operations::owned_parameters(&references, "operation").is_err());
        assert!(operations::return_parameter(&references, "operation").is_err());
        author(&mut model, "second", "direction", json!("return"))?;
        expect(&model, ERROR)?;
        let references = ReferenceIndex::new(Model::from_project(&model));
        let error =
            operations::return_parameter(&references, "operation").expect_err("two known returns");
        assert!(error.contains("at_most_one_return"), "{error}");
    }
    Ok(())
}
#[test]
fn unknown_owned_metaclass_is_not_silently_omitted_from_complete_parameter_query() -> Result {
    use bellatrix_mde::uml::{operations, references::ReferenceIndex, root::Model};
    let mut model = document(true)?;
    model.elements.push(serde_json::from_value(json!({"id":"unknown","kind":"uml.unknown_parameter","name":"Unknown","properties":{"direction":"return"}}))?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-unknown","kind":"owns","from":"operation","to":"unknown"}),
    )?);
    expect(&model, UNKNOWN)?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert!(operations::owned_parameters(&references, "operation").is_err());
    author(&mut model, "second", "direction", json!("return"))?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn malformed_direction_is_not_hidden_by_two_earlier_returns_or_unknown_roles() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        author(&mut model, "second", "direction", json!("return"))?;
        model.elements.push(serde_json::from_value(json!({"id":"bad","kind":"uml.parameter","name":"Bad","properties":{"direction":false}}))?);
        model.relations.push(serde_json::from_value(
            json!({"id":"owns-bad","kind":"owns","from":"operation","to":"bad"}),
        )?);
        if reverse {
            model.relations.reverse();
        }
        expect(&model, INVALID)?;
        author(
            &mut model,
            "operation",
            "ownedParameter",
            json!([{"href":"external.uml#Parameter1"},{"href":"external.uml#Parameter2"},{"href":"external.uml#Parameter3"}]),
        )?;
        expect(&model, INVALID)?;
        let found = issues(&model)?;
        assert!(
            !found[0]["message"]
                .as_str()
                .ok_or("message")?
                .contains("projection"),
            "{found:#?}"
        );
    }
    Ok(())
}
#[test]
fn explicit_projection_must_preserve_order_membership_and_contract() -> Result {
    for value in [
        json!([]),
        json!([{"$ref":"second"},{"$ref":"first"}]),
        json!([{"$ref":"first"},{"$ref":"first"}]),
        json!([{"$ref":"first"},{"$ref":"missing"}]),
        json!([{"$ref":"class"}]),
        json!([{"href":"external.uml#Parameter"},false]),
        Value::Null,
    ] {
        let mut model = document(true)?;
        author(&mut model, "operation", "ownedParameter", value)?;
        expect(&model, INVALID)?;
    }
    let mut model = document(true)?;
    author(
        &mut model,
        "operation",
        "ownedParameter",
        json!([{"$ref":"first"},{"$ref":"second"}]),
    )?;
    assert!(issues(&model)?.is_empty());
    Ok(())
}
#[test]
fn foreign_parameter_projection_is_invalid_not_a_second_local_return() -> Result {
    let mut model = document(true)?;
    model
        .relations
        .iter_mut()
        .find(|relation| relation.id == "owns-second")
        .ok_or("owner")?
        .from = "class".into();
    author(&mut model, "second", "direction", json!("return"))?;
    assert!(issues(&model)?.is_empty());
    author(
        &mut model,
        "operation",
        "ownedParameter",
        json!([{"$ref":"first"},{"$ref":"second"}]),
    )?;
    expect(&model, INVALID)?;
    Ok(())
}
#[test]
fn parameter_type_and_multiplicity_do_not_change_return_count() -> Result {
    let mut model = document(true)?;
    author(
        &mut model,
        "first",
        "type",
        json!({"href":"external.uml#Type"}),
    )?;
    author(
        &mut model,
        "first",
        "upperValue",
        json!({"href":"external.uml#Upper"}),
    )?;
    assert!(issues(&model)?.is_empty());
    author(&mut model, "second", "direction", json!("return"))?;
    expect(&model, ERROR)?;
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
    Ok(())
}

#[test]
fn external_only_projection_cannot_erase_two_canonical_parameter_returns() -> Result {
    use bellatrix_mde::uml::{operations, references::ReferenceIndex, root::Model};
    let mut model = document(true)?;
    author(
        &mut model,
        "operation",
        "ownedParameter",
        json!([{"href":"external.uml#Parameter1"},{"href":"external.uml#Parameter2"}]),
    )?;
    expect(&model, UNKNOWN)?;
    author(&mut model, "second", "direction", json!("return"))?;
    expect(&model, ERROR)?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert!(operations::owned_parameters(&references, "operation").is_err());
    let error = operations::return_parameter(&references, "operation")
        .expect_err("known count dominates incomplete projection");
    assert!(error.contains("at_most_one_return"), "{error}");
    Ok(())
}

#[test]
fn external_neighbor_cannot_hide_known_projection_order_contradiction() -> Result {
    let mut model = document(true)?;
    author(
        &mut model,
        "operation",
        "ownedParameter",
        json!([{"$ref":"second"},{"$ref":"first"},{"href":"external.uml#Parameter"}]),
    )?;
    expect(&model, INVALID)?;
    author(
        &mut model,
        "operation",
        "ownedParameter",
        json!([{"$ref":"first"},{"href":"external.uml#Parameter"},{"$ref":"second"}]),
    )?;
    expect(&model, UNKNOWN)?;
    Ok(())
}

#[test]
fn partial_projection_cannot_have_fewer_entries_than_known_owned_parameters() -> Result {
    use bellatrix_mde::uml::{operations, references::ReferenceIndex, root::Model};
    for rooted in [false, true] {
        for reverse in [false, true] {
            let mut model = document(rooted)?;
            if reverse {
                model.relations.reverse();
            }
            author(
                &mut model,
                "operation",
                "ownedParameter",
                json!([{"href":"external.uml#Parameter"}]),
            )?;
            for direction in ["in", "return"] {
                author(&mut model, "second", "direction", json!(direction))?;
                expect(&model, INVALID)?;
                let references = ReferenceIndex::new(Model::from_project(&model));
                for error in [
                    operations::owned_parameters(&references, "operation")
                        .expect_err("projection is too short"),
                    operations::return_parameter(&references, "operation")
                        .expect_err("projection is too short"),
                ] {
                    assert!(error.contains("too few entries"), "{error}");
                }
            }
        }
    }
    Ok(())
}

#[test]
fn external_projection_slots_must_fit_before_between_and_after_local_anchors() -> Result {
    for values in [
        json!([{"$ref":"second"},{"href":"external.uml#Parameter"}]),
        json!([{"href":"external.uml#Parameter"},{"$ref":"first"}]),
    ] {
        let mut model = document(true)?;
        author(&mut model, "operation", "ownedParameter", values)?;
        expect(&model, INVALID)?;
    }
    let mut model = document(true)?;
    model.elements.push(serde_json::from_value(
        json!({"id":"third","kind":"uml.parameter","name":"Third"}),
    )?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-third","kind":"owns","from":"operation","to":"third"}),
    )?);
    author(
        &mut model,
        "operation",
        "ownedParameter",
        json!([
            {"$ref":"first"},{"$ref":"third"},{"href":"external.uml#Parameter"}
        ]),
    )?;
    expect(&model, INVALID)?;
    author(
        &mut model,
        "operation",
        "ownedParameter",
        json!([
            {"$ref":"first"},{"href":"external.uml#Parameter"},{"$ref":"third"}
        ]),
    )?;
    expect(&model, UNKNOWN)?;
    Ok(())
}

#[test]
fn unknown_local_projection_anchor_is_not_a_free_external_slot() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        model.elements.push(serde_json::from_value(
            json!({"id":"unknown","kind":"uml.unknown_parameter","name":"Unknown"}),
        )?);
        model.relations.insert(
            0,
            serde_json::from_value(
                json!({"id":"owns-unknown","kind":"owns","from":"operation","to":"unknown"}),
            )?,
        );
        let mut values = vec![
            json!({"$ref":"unknown"}),
            json!({"$ref":"second"}),
            json!({"href":"external.uml#Parameter"}),
        ];
        if reverse {
            model.relations.reverse();
            values.reverse();
        }
        author(&mut model, "operation", "ownedParameter", json!(values))?;
        expect(&model, INVALID)?;
        let mut possible = vec![
            json!({"$ref":"unknown"}),
            json!({"href":"external.uml#Parameter"}),
            json!({"$ref":"second"}),
        ];
        if reverse {
            possible.reverse();
        }
        author(&mut model, "operation", "ownedParameter", json!(possible))?;
        expect(&model, UNKNOWN)?;
    }
    Ok(())
}
