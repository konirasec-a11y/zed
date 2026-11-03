use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "TemplateBinding-parameter_substitution_formal:";
const ERROR: &str = "UML_TEMPLATE_BINDING_FORMAL";
const INVALID: &str = "MDE_TEMPLATE_BINDING_FORMAL_INVALID";
const UNKNOWN: &str = "MDE_TEMPLATE_BINDING_FORMAL_UNVERIFIED";
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
#[test]
fn substitution_formal_in_target_signature_is_valid_in_rooted_and_draft_models() -> Result {
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
fn foreign_formal_is_violation_and_survives_roundtrip() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    expect(&project, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
    }
    Ok(())
}
#[test]
fn empty_substitutions_make_membership_vacuous_without_resolving_signature() -> Result {
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
    Ok(())
}
#[test]
fn unresolved_references_or_projections_remain_unknown() -> Result {
    for (id, key) in [
        ("binding", "signature"),
        ("signature", "parameter"),
        ("substitution", "formal"),
    ] {
        let mut project = document(true)?;
        project
            .elements
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or("element")?
            .properties
            .remove(key);
        expect(&project, UNKNOWN)?;
    }
    for (id, key, value) in [
        (
            "binding",
            "signature",
            json!({"href":"external.uml#Signature"}),
        ),
        (
            "signature",
            "parameter",
            json!([{"href":"external.uml#Parameter"}]),
        ),
        (
            "substitution",
            "formal",
            json!({"href":"external.uml#Parameter"}),
        ),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, key, value)?;
        expect(&project, UNKNOWN)?;
    }
    Ok(())
}
#[test]
fn known_foreign_formal_dominates_unknown_substitutions_in_either_order() -> Result {
    for reverse in [true, false] {
        let mut project = document(true)?;
        let mut values = vec![
            json!({"href":"external.uml#Substitution"}),
            json!({"$ref":"substitution"}),
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
            "substitution",
            "formal",
            json!({"$ref":"foreign"}),
        )?;
        expect(&project, ERROR)?;
    }
    Ok(())
}
#[test]
fn incomplete_signature_cannot_prove_formal_exclusion() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "signature",
        "parameter",
        json!([{"$ref":"formal"},{"href":"external.uml#Parameter"}]),
    )?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn malformed_values_dominate_unknown_and_known_exclusion() -> Result {
    for (id, key, value) in [
        ("binding", "signature", json!({"$ref":"missing"})),
        ("binding", "signature", json!({"$ref":"actual"})),
        ("signature", "parameter", json!([])),
        (
            "signature",
            "parameter",
            json!([{"$ref":"formal"},{"$ref":"formal"}]),
        ),
        ("binding", "parameterSubstitution", Value::Null),
        ("binding", "parameterSubstitution", json!(42)),
        (
            "binding",
            "parameterSubstitution",
            json!([{"$ref":"substitution"},{"$ref":"substitution"}]),
        ),
        (
            "binding",
            "parameterSubstitution",
            json!([{"$ref":"missing"}]),
        ),
        (
            "binding",
            "parameterSubstitution",
            json!([{"$ref":"actual"}]),
        ),
        ("substitution", "formal", Value::Null),
        ("substitution", "formal", json!(42)),
        ("substitution", "formal", json!({"$ref":"actual"})),
    ] {
        let mut project = document(true)?;
        author(
            &mut project,
            "binding",
            "parameterSubstitution",
            json!([{"href":"external.uml#Substitution"},{"$ref":"substitution"}]),
        )?;
        author(&mut project, id, key, value)?;
        expect(&project, INVALID)?;
    }
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "binding")
        .ok_or("binding")?
        .properties
        .remove("parameterSubstitution");
    author(&mut project, "binding", "signature", json!(42))?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn malformed_formal_is_not_hidden_by_external_signature() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "binding",
        "signature",
        json!({"href":"external.uml#Signature"}),
    )?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"missing"}),
    )?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn unresolved_formal_metaclass_preserves_known_identity_exclusion() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "foreign")
        .ok_or("foreign")?
        .kind = "uml.unresolved_parameter".into();
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn explicit_binding_target_contradiction_remains_invalid_with_unresolved_signature_kind() -> Result
{
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "other-signature")
        .ok_or("signature")?
        .kind = "uml.unresolved_signature".into();
    project.elements.retain(|e| e.id != "binding");
    project.relations.push(serde_json::from_value(json!({"id":"binding","kind":"uml.template_binding","from":"host","to":"signature","properties":{"signature":{"$ref":"other-signature"},"parameterSubstitution":[{"$ref":"substitution"}]}}))?);
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn rejected_snapshot_can_be_repaired_after_json_and_protobuf_reopen() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&project, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "substitution",
            "formal",
            json!({"$ref":"formal"}),
        )?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened.relations, project.relations);
    }
    Ok(())
}

#[test]
fn known_signature_member_proves_inclusion_despite_an_external_neighbor() -> Result {
    for reverse in [true, false] {
        let mut project = document(true)?;
        let mut parameters = vec![
            json!({"$ref":"formal"}),
            json!({"href":"external.uml#Parameter"}),
        ];
        if reverse {
            parameters.reverse();
        }
        author(&mut project, "signature", "parameter", json!(parameters))?;
        assert!(issues(&project)?.is_empty());
        for format in [Format::Json, Format::Protobuf] {
            assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
        }
    }
    Ok(())
}
#[test]
fn unresolved_formal_metaclasses_do_not_erase_known_membership_identities() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "formal")
        .ok_or("formal")?
        .kind = "uml.unresolved_parameter".into();
    assert!(issues(&project)?.is_empty());
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    expect(&project, ERROR)?;
    Ok(())
}
