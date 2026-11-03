use bellatrix_mde::commands::{Command, apply};
use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "TemplateSignature-unique_parameters:";
const ERROR: &str = "UML_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS";
const INVALID: &str = "MDE_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS_INVALID";
const UNKNOWN: &str = "MDE_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Bindings","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"host","kind":"uml.package","name":"Host"},
        {"id":"target","kind":"uml.package","name":"Target"},
        {"id":"signature","kind":"uml.template_signature","name":"Signature","properties":{"parameter":[{"$ref":"formal"},{"$ref":"foreign"}]}},
        {"id":"other-signature","kind":"uml.template_signature","name":"Other signature","properties":{"parameter":[{"$ref":"foreign"}]}},
        {"id":"formal","kind":"uml.template_parameter","name":"Formal"},
        {"id":"foreign","kind":"uml.template_parameter","name":"Foreign"},
        {"id":"formal-type","kind":"uml.class","name":"FormalType"},
        {"id":"foreign-type","kind":"uml.class","name":"ForeignType"},
        {"id":"actual","kind":"uml.class","name":"Actual"},
        {"id":"binding","kind":"uml.template_binding","name":"Binding","properties":{"signature":{"$ref":"signature"},"parameterSubstitution":[{"$ref":"substitution"}]}},
        {"id":"substitution","kind":"uml.template_parameter_substitution","name":"Substitution","properties":{"formal":{"$ref":"formal"},"actual":{"$ref":"actual"}}}
    ],"relations":[
        {"id":"parametered-formal","kind":"uml.parametered_element","from":"formal","to":"formal-type"},
        {"id":"parametered-foreign","kind":"uml.parametered_element","from":"foreign","to":"foreign-type"}
    ]});
    for (owner, child) in [
        ("root", "host"),
        ("root", "target"),
        ("target", "signature"),
        ("host", "other-signature"),
        ("signature", "formal"),
        ("other-signature", "foreign"),
        ("formal", "formal-type"),
        ("foreign", "foreign-type"),
        ("host", "actual"),
        ("host", "binding"),
        ("binding", "substitution"),
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
        .find(|e| e.id == id)
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
            issue["subject_id"] == "signature"
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
fn kind(project: &mut Project, id: &str, value: &str) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .kind = value.into();
    Ok(())
}
fn name(project: &mut Project, id: &str, value: &str) -> Result {
    project
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("element")?
        .name = value.into();
    Ok(())
}
#[test]
fn includes_nonowned_formals_and_uses_exposed_names_not_parameter_labels() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        name(&mut project, "formal", "Same label")?;
        name(&mut project, "foreign", "Same label")?;
        assert!(issues(&project)?.is_empty());
        name(&mut project, "foreign-type", "FormalType")?;
        expect(&project, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format)?, format)?;
            expect(&reopened, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn unnamed_named_elements_compare_as_null_not_as_distinct_defaults() -> Result {
    let mut project = document(true)?;
    name(&mut project, "formal-type", "")?;
    assert!(issues(&project)?.is_empty());
    name(&mut project, "foreign-type", "")?;
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn known_duplicate_survives_external_collection_member() -> Result {
    let mut project = document(true)?;
    name(&mut project, "foreign-type", "FormalType")?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"formal"},{"$ref":"foreign"},{"href":"external.uml#P"}]),
    )?;
    expect(&project, ERROR)?;
    name(&mut project, "foreign-type", "Distinct")?;
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn singleton_unknown_is_vacuous_but_two_unknowns_are_not() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"href":"external.uml#P"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"href":"external.uml#P"},{"href":"external.uml#Q"}]),
    )?;
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"formal"}]),
    )?;
    project.relations.retain(|r| r.id != "parametered-formal");
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn missing_collection_or_exposed_projection_is_unknown() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "signature")
        .ok_or("signature")?
        .properties
        .remove("parameter");
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"formal"},{"$ref":"foreign"}]),
    )?;
    project.relations.retain(|r| r.id != "parametered-formal");
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn malformed_members_and_exposed_projections_remain_invalid() -> Result {
    for value in [
        json!(42),
        json!([]),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"actual"}]),
        json!([{"$ref":"formal"},{"$ref":"formal"}]),
    ] {
        let mut project = document(true)?;
        author(&mut project, "signature", "parameter", value)?;
        expect(&project, INVALID)?;
    }
    let mut project = document(true)?;
    author(&mut project, "formal", "parameteredElement", json!(42))?;
    author(
        &mut project,
        "foreign",
        "parameteredElement",
        json!({"href":"external.uml#E"}),
    )?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn contradictory_local_id_cannot_hide_behind_unresolved_metaclass() -> Result {
    let mut project = document(true)?;
    kind(&mut project, "actual", "uml.unresolved_parameterable")?;
    author(
        &mut project,
        "formal",
        "parameteredElement",
        json!({"$ref":"actual"}),
    )?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn two_distinct_formals_exposing_same_element_still_collide() -> Result {
    let mut project = document(true)?;
    project
        .relations
        .iter_mut()
        .find(|r| r.id == "parametered-foreign")
        .ok_or("reference")?
        .to = "formal-type".into();
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn renaming_nonowned_parameter_target_rolls_back_and_invalid_snapshot_repairs() -> Result {
    let project = document(true)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "foreign-type".into(),
            path: "name".into(),
            value: json!("FormalType"),
        },
        1,
    )
    .expect_err("duplicate name in signature parameter extent");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(before, encode(&project, Format::Json)?);
    let mut invalid = project;
    name(&mut invalid, "foreign-type", "FormalType")?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "foreign-type".into(),
                path: "name".into(),
                value: json!("Repaired"),
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}

#[test]
fn malformed_relationship_name_is_invalid_not_an_absent_name() -> Result {
    let mut project = document(true)?;
    project.relations.push(serde_json::from_value(json!({"id":"binary","kind":"uml.association","from":"actual","to":"formal-type","properties":{"name":42}}))?);
    project
        .relations
        .iter_mut()
        .find(|r| r.id == "parametered-formal")
        .ok_or("reference")?
        .to = "binary".into();
    expect(&project, INVALID)?;
    name(&mut project, "foreign-type", "")?;
    project
        .relations
        .iter_mut()
        .find(|r| r.id == "binary")
        .ok_or("binary")?
        .properties
        .insert("name".into(), Value::Null);
    expect(&project, ERROR)?;
    project
        .relations
        .iter_mut()
        .find(|r| r.id == "binary")
        .ok_or("binary")?
        .properties
        .remove("name");
    expect(&project, ERROR)?;

    Ok(())
}
