use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "OperationTemplateParameter-match_default_signature:";
const ERROR: &str = "UML_OPERATION_TEMPLATE_DEFAULT_SIGNATURE";
const INVALID: &str = "MDE_OPERATION_TEMPLATE_DEFAULT_SIGNATURE_INVALID";
const UNKNOWN: &str = "MDE_OPERATION_TEMPLATE_DEFAULT_SIGNATURE_UNVERIFIED";

fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Operation templates","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"class","kind":"uml.class","name":"Class"},
            {"id":"other","kind":"uml.class","name":"Other"},
            {"id":"default","kind":"uml.operation","name":"Default"},
            {"id":"exposed","kind":"uml.operation","name":"Exposed"},
            {"id":"template","kind":"uml.operation_template_parameter","name":"Formal","properties":{"default":{"$ref":"default"},"parameteredElement":{"$ref":"exposed"}}},
            {"id":"first","kind":"uml.parameter","name":"First","properties":{"type":{"$ref":"class"}}},
            {"id":"second","kind":"uml.parameter","name":"Second","properties":{"type":{"$ref":"class"}}}
        ],"relations":[]});
    for (owner, child) in [
        ("root", "class"),
        ("root", "other"),
        ("class", "default"),
        ("class", "exposed"),
        ("default", "first"),
        ("exposed", "second"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
}
fn author(project: &mut Project, id: &str, key: &str, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn issues(project: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "template"
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with(RULE))
        })
        .cloned()
        .collect())
}
fn expect(project: &Project, code: &str) -> Result {
    let issues = issues(project)?;
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0]["code"], code, "{issues:#?}");
    assert_eq!(
        issues[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
#[test]
fn exact_signatures_use_normative_defaults_and_survive_roundtrip() -> Result {
    for rooted in [true, false] {
        let project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        for format in [Format::Json, Format::Protobuf] {
            assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
        }
    }
    Ok(())
}
#[test]
fn signature_checks_each_exact_field_not_type_conformance_or_bound_containment() -> Result {
    for (key, value) in [
        ("type", json!({"$ref":"other"})),
        ("direction", json!("out")),
        ("isOrdered", json!(true)),
        ("isUnique", json!(false)),
        ("multiplicity", json!("0..1")),
        ("multiplicity", json!("1..2")),
    ] {
        let mut project = document(true)?;
        author(&mut project, "first", key, value)?;
        if key == "type" {
            project.relations.push(serde_json::from_value(json!({
                "id":"subtype", "kind":"uml.generalization", "from":"other", "to":"class"
            }))?);
        }
        expect(&project, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn default_must_be_operation_but_absent_default_is_vacuously_satisfied() -> Result {
    let mut project = document(true)?;
    author(&mut project, "template", "default", json!({"$ref":"class"}))?;
    expect(&project, ERROR)?;
    author(&mut project, "template", "default", Value::Null)?;
    author(
        &mut project,
        "template",
        "parameteredElement",
        json!({"href":"external.uml#Operation"}),
    )?;
    assert!(issues(&project)?.is_empty());
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "template")
        .ok_or("template")?
        .properties
        .remove("default");
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "template",
        "ownedDefault",
        json!({"$ref":"default"}),
    )?;
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn missing_projection_external_reference_and_unknown_metaclass_are_unverified() -> Result {
    for (id, key) in [
        ("template", "default"),
        ("template", "parameteredElement"),
        ("first", "type"),
    ] {
        let mut project = document(true)?;
        author(
            &mut project,
            id,
            key,
            json!({"href":"external.uml#Element"}),
        )?;
        expect(&project, UNKNOWN)?;
    }
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "template")
        .ok_or("template")?
        .properties
        .remove("parameteredElement");
    expect(&project, UNKNOWN)?;
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "default")
        .ok_or("default")?
        .kind = "uml.unknown_operation".into();
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn malformed_references_and_scalars_are_invalid_not_unknown() -> Result {
    for (id, key, value) in [
        ("template", "default", json!({"$ref":"missing"})),
        ("template", "parameteredElement", json!({"$ref":"class"})),
        ("first", "type", json!(42)),
        ("first", "direction", json!(42)),
        ("first", "isOrdered", Value::Null),
        ("first", "multiplicity", json!(42)),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, key, value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn invalid_dominates_unknown_and_false_regardless_of_operand_side() -> Result {
    for (first, second) in [("first", "second"), ("second", "first")] {
        let mut project = document(true)?;
        author(
            &mut project,
            first,
            "type",
            json!({"href":"external.uml#Type"}),
        )?;
        author(&mut project, second, "type", json!({"$ref":"missing"}))?;
        author(&mut project, first, "isUnique", json!(false))?;
        expect(&project, INVALID)?;
        author(&mut project, second, "type", json!({"$ref":"class"}))?;
        expect(&project, ERROR)?;
    }
    Ok(())
}
#[test]
fn unequal_parameter_count_is_false_and_order_is_significant() -> Result {
    let mut project = document(true)?;
    project
        .relations
        .retain(|relation| relation.id != "owns-second");
    expect(&project, ERROR)?;
    let mut project = document(true)?;
    for (id, owner) in [("third", "default"), ("fourth", "exposed")] {
        project.elements.push(serde_json::from_value(json!({"id":id,"kind":"uml.parameter","name":id,"properties":{"type":{"$ref":"other"}}}))?);
        project.relations.push(serde_json::from_value(
            json!({"id":format!("owns-{id}"),"kind":"owns","from":owner,"to":id}),
        )?);
    }
    assert!(issues(&project)?.is_empty());
    let first = project
        .relations
        .iter()
        .position(|relation| relation.id == "owns-second")
        .ok_or("second")?;
    let second = project
        .relations
        .iter()
        .position(|relation| relation.id == "owns-fourth")
        .ok_or("fourth")?;
    project.relations.swap(first, second);
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn mismatch_command_rolls_back_and_reopened_snapshot_can_be_repaired() -> Result {
    let project = document(true)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "first".into(),
            path: "properties.isUnique".into(),
            value: json!(false),
        },
        1,
    )
    .expect_err("signature mismatch");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(before, encode(&project, Format::Json)?);
    let mut invalid = project;
    author(&mut invalid, "first", "isUnique", json!(false))?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "first".into(),
                path: "properties.isUnique".into(),
                value: json!(true),
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}

#[test]
fn unevaluated_bounds_stay_unknown_but_known_difference_decides_false() -> Result {
    for bound in ["lowerValue", "upperValue"] {
        let mut project = document(true)?;
        project.elements.push(serde_json::from_value(
            json!({"id":"bound","kind":"uml.opaque_expression","name":"Unevaluated"}),
        )?);
        project.relations.push(serde_json::from_value(
            json!({"id":"owns-bound","kind":"owns","from":"first","to":"bound"}),
        )?);
        author(&mut project, "first", bound, json!({"$ref":"bound"}))?;
        expect(&project, UNKNOWN)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&project, format)?, format)?, UNKNOWN)?;
        }
        author(&mut project, "first", "isUnique", json!(false))?;
        expect(&project, ERROR)?;
        author(&mut project, "second", "direction", json!(42))?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn absent_types_compare_as_null_and_unknown_snapshots_remain_inspectable() -> Result {
    let mut project = document(true)?;
    for id in ["first", "second"] {
        author(&mut project, id, "type", Value::Null)?;
    }
    assert!(issues(&project)?.is_empty());
    author(&mut project, "first", "type", json!({"$ref":"class"}))?;
    expect(&project, ERROR)?;
    author(
        &mut project,
        "first",
        "type",
        json!({"href":"external.uml#Type"}),
    )?;
    for format in [Format::Json, Format::Protobuf] {
        expect(&decode(&encode(&project, format)?, format)?, UNKNOWN)?;
    }
    Ok(())
}

#[test]
fn owned_exposed_operation_is_not_mistaken_for_an_unmaterialized_default() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        author(&mut project, "template", "default", Value::Null)?;
        author(
            &mut project,
            "template",
            "ownedParameteredElement",
            json!({"$ref":"exposed"}),
        )?;
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "owns-exposed")
            .ok_or("owns-exposed")?
            .from = "template".into();
        assert!(issues(&project)?.is_empty());
        for format in [Format::Json, Format::Protobuf] {
            assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
        }
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "template")
            .ok_or("template")?
            .properties
            .remove("ownedParameteredElement");
        expect(&project, UNKNOWN)?;
    }
    Ok(())
}
