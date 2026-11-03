use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "TemplateBinding-one_parameter_substitution:";
const ERROR: &str = "UML_TEMPLATE_BINDING_UNIQUE_SUBSTITUTION";
const INVALID: &str = "MDE_TEMPLATE_BINDING_UNIQUE_SUBSTITUTION_INVALID";
const UNKNOWN: &str = "MDE_TEMPLATE_BINDING_UNIQUE_SUBSTITUTION_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Bindings","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"host","kind":"uml.package","name":"Host"},
        {"id":"target","kind":"uml.package","name":"Target"},
        {"id":"signature","kind":"uml.template_signature","name":"Signature","properties":{"parameter":[{"$ref":"formal"}]}},
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
            issue["subject_id"] == "binding"
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
fn second_substitution(project: &mut Project, formal: &str) -> Result {
    project.elements.push(serde_json::from_value(json!({"id":"second-substitution","kind":"uml.template_parameter_substitution","name":"Second","properties":{"formal":{"$ref":formal},"actual":{"$ref":"actual"}}}))?);
    project.relations.push(serde_json::from_value(json!({"id":"owns-second-substitution","kind":"owns","from":"binding","to":"second-substitution"}))?);
    author(
        project,
        "binding",
        "parameterSubstitution",
        json!([{"$ref":"substitution"},{"$ref":"second-substitution"}]),
    )
}
#[test]
fn zero_or_one_substitution_per_formal_is_valid_and_roundtrips() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        second_substitution(&mut project, "foreign")?;
        author(
            &mut project,
            "signature",
            "parameter",
            json!([{"$ref":"formal"},{"$ref":"foreign"}]),
        )?;
        assert!(issues(&project)?.is_empty());
        for format in [Format::Json, Format::Protobuf] {
            assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
        }
    }
    Ok(())
}
#[test]
fn two_distinct_substitutions_for_one_target_formal_violate() -> Result {
    let mut project = document(true)?;
    second_substitution(&mut project, "formal")?;
    expect(&project, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
    }
    Ok(())
}
#[test]
fn duplicated_foreign_formal_is_not_counted_by_this_rule() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    second_substitution(&mut project, "foreign")?;
    assert!(issues(&project)?.is_empty());
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    assert!(
        response["diagnostics"]
            .as_array()
            .ok_or("diagnostics")?
            .iter()
            .any(|issue| issue["code"] == "UML_TEMPLATE_BINDING_FORMAL")
    );
    Ok(())
}
#[test]
fn known_duplicate_dominates_unknown_substitutions_in_both_orders() -> Result {
    for reverse in [true, false] {
        let mut project = document(true)?;
        second_substitution(&mut project, "foreign")?;
        let mut values = vec![
            json!({"href":"external.uml#Substitution"}),
            json!({"$ref":"substitution"}),
            json!({"$ref":"second-substitution"}),
        ];
        if reverse {
            values.reverse();
        }
        author(
            &mut project,
            "binding",
            "parameterSubstitution",
            json!(values),
        )?;
        expect(&project, UNKNOWN)?;
        author(
            &mut project,
            "second-substitution",
            "formal",
            json!({"$ref":"formal"}),
        )?;
        expect(&project, ERROR)?;
        author(&mut project, "second-substitution", "formal", json!(42))?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn uncertain_signature_membership_only_matters_for_duplicated_formals() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "binding",
        "signature",
        json!({"href":"external.uml#Signature"}),
    )?;
    assert!(issues(&project)?.is_empty());
    second_substitution(&mut project, "formal")?;
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "binding",
        "signature",
        json!({"$ref":"signature"}),
    )?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"foreign"},{"href":"external.uml#Parameter"}]),
    )?;
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"formal"},{"href":"external.uml#Parameter"}]),
    )?;
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn empty_substitutions_are_safe_with_unknown_signature_but_not_malformed_inputs() -> Result {
    let mut project = document(true)?;
    author(&mut project, "binding", "parameterSubstitution", json!([]))?;
    project.relations.retain(|r| r.id != "owns-substitution");
    author(
        &mut project,
        "binding",
        "signature",
        json!({"href":"external.uml#Signature"}),
    )?;
    assert!(issues(&project)?.is_empty());
    for value in [
        json!(42),
        json!({"$ref":"missing"}),
        json!({"$ref":"actual"}),
    ] {
        author(&mut project, "binding", "signature", value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn malformed_references_and_duplicate_collection_entries_remain_invalid() -> Result {
    for (id, key, value) in [
        (
            "binding",
            "parameterSubstitution",
            json!([{"$ref":"substitution"},{"$ref":"substitution"}]),
        ),
        ("binding", "parameterSubstitution", Value::Null),
        ("signature", "parameter", json!([])),
        (
            "signature",
            "parameter",
            json!([{"$ref":"formal"},{"$ref":"formal"}]),
        ),
        ("substitution", "formal", Value::Null),
        ("substitution", "formal", json!({"$ref":"missing"})),
    ] {
        let mut project = document(true)?;
        second_substitution(&mut project, "formal")?;
        author(&mut project, id, key, value)?;
        expect(&project, INVALID)?;
    }
    let mut project = document(true)?;
    author(
        &mut project,
        "binding",
        "signature",
        json!({"href":"external.uml#Signature"}),
    )?;
    author(&mut project, "substitution", "formal", json!(42))?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn local_formal_identity_can_prove_duplicates_despite_unknown_metaclass() -> Result {
    let mut project = document(true)?;
    second_substitution(&mut project, "formal")?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "formal")
        .ok_or("formal")?
        .kind = "uml.unresolved_parameter".into();
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn missing_collection_is_unknown_but_missing_formal_in_singleton_cannot_duplicate() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "binding")
        .ok_or("binding")?
        .properties
        .remove("parameterSubstitution");
    expect(&project, UNKNOWN)?;
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "substitution")
        .ok_or("substitution")?
        .properties
        .remove("formal");
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn duplicate_snapshot_can_be_repaired_after_reopening() -> Result {
    let mut project = document(true)?;
    second_substitution(&mut project, "formal")?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"formal"},{"$ref":"foreign"}]),
    )?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&project, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "second-substitution",
            "formal",
            json!({"$ref":"foreign"}),
        )?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened.relations, project.relations);
    }
    Ok(())
}

#[test]
fn single_external_substitution_or_formal_has_an_upper_count_of_one() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"href":"external.uml#Formal"}),
    )?;
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "binding",
        "parameterSubstitution",
        json!([{"href":"external.uml#Substitution"}]),
    )?;
    project
        .relations
        .retain(|relation| relation.id != "owns-substitution");
    assert!(issues(&project)?.is_empty());
    for format in [Format::Json, Format::Protobuf] {
        assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
    }
    Ok(())
}
#[test]
fn only_unresolved_substitutions_contribute_to_upper_bound_of_unlisted_formal() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    second_substitution(&mut project, "foreign")?;
    author(
        &mut project,
        "binding",
        "parameterSubstitution",
        json!([{"$ref":"substitution"},{"$ref":"second-substitution"},{"href":"external.uml#Substitution"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "binding",
        "parameterSubstitution",
        json!([{"$ref":"substitution"},{"$ref":"second-substitution"},{"href":"external.uml#Substitution"},{"href":"external.uml#Another"}]),
    )?;
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"formal"},{"$ref":"foreign"}]),
    )?;
    expect(&project, ERROR)?;
    Ok(())
}
