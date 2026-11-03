use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "ClassifierTemplateParameter-actual_is_classifier:";
const ERROR: &str = "UML_CLASSIFIER_TEMPLATE_ACTUAL";
const INVALID: &str = "MDE_CLASSIFIER_TEMPLATE_ACTUAL_INVALID";
const UNKNOWN: &str = "MDE_CLASSIFIER_TEMPLATE_ACTUAL_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Bindings","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"host","kind":"uml.package","name":"Host"},
        {"id":"target","kind":"uml.package","name":"Target"},
        {"id":"signature","kind":"uml.template_signature","name":"Signature","properties":{"parameter":[{"$ref":"formal"}]}},
        {"id":"other-signature","kind":"uml.template_signature","name":"Other signature","properties":{"parameter":[{"$ref":"foreign"}]}},
        {"id":"formal","kind":"uml.classifier_template_parameter","name":"Formal","properties":{"allowSubstitutable":false}},
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
            issue["subject_id"] == "substitution"
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
#[test]
fn classifier_kinds_satisfy_actual_constraint_in_rooted_and_draft_snapshots() -> Result {
    for rooted in [true, false] {
        for actual_kind in [
            "uml.class",
            "uml.interface",
            "uml.data_type",
            "uml.primitive_type",
            "uml.enumeration",
            "uml.component",
            "uml.association",
            "uml.stereotype",
        ] {
            let mut project = document(rooted)?;
            kind(&mut project, "actual", actual_kind)?;
            assert!(issues(&project)?.is_empty());
            for format in [Format::Json, Format::Protobuf] {
                assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
            }
        }
    }
    Ok(())
}
#[test]
fn parameterable_nonclassifier_actual_is_a_violation_not_a_type_representation_error() -> Result {
    for actual_kind in [
        "uml.operation",
        "uml.property",
        "uml.literal_integer",
        "uml.model",
        "uml.package",
    ] {
        let mut project = document(true)?;
        kind(&mut project, "actual", actual_kind)?;
        expect(&project, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn generic_and_operation_template_parameters_do_not_receive_classifier_restriction() -> Result {
    for formal_kind in ["uml.template_parameter", "uml.operation_template_parameter"] {
        let mut project = document(true)?;
        kind(&mut project, "formal", formal_kind)?;
        kind(&mut project, "actual", "uml.operation")?;
        assert!(issues(&project)?.is_empty());
        author(
            &mut project,
            "substitution",
            "actual",
            json!({"href":"external.uml#Argument"}),
        )?;
        assert!(issues(&project)?.is_empty());
    }
    Ok(())
}
#[test]
fn unknown_formal_or_actual_is_resolved_only_by_a_decisive_implication_operand() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"href":"external.uml#Formal"}),
    )?;
    assert!(issues(&project)?.is_empty());
    kind(&mut project, "actual", "uml.operation")?;
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"formal"}),
    )?;
    author(
        &mut project,
        "substitution",
        "actual",
        json!({"href":"external.uml#Actual"}),
    )?;
    expect(&project, UNKNOWN)?;
    kind(&mut project, "formal", "uml.template_parameter")?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn unresolved_metaclasses_are_unknown_not_a_nonclassifier_proof() -> Result {
    let mut project = document(true)?;
    kind(&mut project, "actual", "uml.unresolved_actual")?;
    expect(&project, UNKNOWN)?;
    kind(&mut project, "actual", "uml.operation")?;
    kind(&mut project, "formal", "uml.unresolved_parameter")?;
    expect(&project, UNKNOWN)?;
    kind(&mut project, "actual", "uml.class")?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn malformed_references_remain_invalid_despite_a_decisive_other_operand() -> Result {
    for (property, value) in [
        ("formal", json!(42)),
        ("formal", json!({"$ref":"actual"})),
        ("formal", json!({"$ref":"missing"})),
        ("actual", json!(42)),
        ("actual", json!({"$ref":"substitution"})),
        ("actual", json!({"$ref":"missing"})),
        ("actual", json!({"href":42})),
    ] {
        let mut project = document(true)?;
        if property == "actual" {
            kind(&mut project, "formal", "uml.template_parameter")?;
        }
        author(&mut project, "substitution", property, value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn missing_actual_projection_remains_unknown_even_with_owned_actual_role() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "substitution")
        .ok_or("substitution")?
        .properties
        .remove("actual");
    author(
        &mut project,
        "substitution",
        "ownedActual",
        json!({"$ref":"actual"}),
    )?;
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "owns-actual")
        .ok_or("owns-actual")?
        .from = "substitution".into();
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn missing_formal_can_be_decided_by_a_classifier_actual_but_not_a_nonclassifier() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "substitution")
        .ok_or("substitution")?
        .properties
        .remove("formal");
    assert!(issues(&project)?.is_empty());
    kind(&mut project, "actual", "uml.operation")?;
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn nonclassifier_snapshot_can_be_repaired_after_reopening() -> Result {
    let mut project = document(true)?;
    kind(&mut project, "actual", "uml.operation")?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&project, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "substitution",
            "actual",
            json!({"$ref":"formal-type"}),
        )?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened.relations, project.relations);
    }
    Ok(())
}

#[test]
fn authored_inverse_projection_is_explicitly_unverified_and_preserved() -> Result {
    for inverse in [
        json!([{"href":"external.uml#Substitution"}]),
        json!([{"$ref":"substitution"}]),
    ] {
        let mut project = document(true)?;
        author(
            &mut project,
            "substitution",
            "formal",
            json!({"$ref":"foreign"}),
        )?;
        author(
            &mut project,
            "formal",
            "templateParameterSubstitution",
            inverse.clone(),
        )?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format)?, format)?;
            let response: Value = serde_json::from_str(&validate_uml_model(
                &json!({"api_version":1,"model":reopened}).to_string(),
            )?)?;
            let found = response["diagnostics"]
                .as_array()
                .ok_or("diagnostics")?
                .iter()
                .filter(|issue| {
                    issue["subject_id"] == "formal"
                        && issue["message"]
                            .as_str()
                            .is_some_and(|message| message.starts_with(RULE))
                })
                .collect::<Vec<_>>();
            assert_eq!(found.len(), 1, "{found:#?}");
            assert_eq!(found[0]["code"], UNKNOWN);
            assert_eq!(found[0]["severity"], "warning");
            assert_eq!(
                reopened
                    .elements
                    .iter()
                    .find(|element| element.id == "formal")
                    .ok_or("formal")?
                    .properties
                    .get("templateParameterSubstitution"),
                Some(&inverse)
            );
            assert!(issues(&reopened)?.is_empty());
        }
    }
    Ok(())
}
