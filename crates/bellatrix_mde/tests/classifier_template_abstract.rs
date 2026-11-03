use bellatrix_mde::commands::{Command, apply};
use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "ClassifierTemplateParameter-matching_abstract:";
const ERROR: &str = "UML_CLASSIFIER_TEMPLATE_MATCHING_ABSTRACT";
const INVALID: &str = "MDE_CLASSIFIER_TEMPLATE_MATCHING_ABSTRACT_INVALID";
const UNKNOWN: &str = "MDE_CLASSIFIER_TEMPLATE_MATCHING_ABSTRACT_UNVERIFIED";
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
fn abstraction_implication_uses_normative_defaults_and_all_boolean_combinations() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        for exposed in [false, true] {
            for actual in [false, true] {
                author(&mut project, "formal-type", "isAbstract", json!(exposed))?;
                author(&mut project, "actual", "isAbstract", json!(actual))?;
                for format in [Format::Json, Format::Protobuf] {
                    let reopened = decode(&encode(&project, format)?, format)?;
                    if !exposed && actual {
                        expect(&reopened, ERROR)?;
                    } else {
                        assert!(issues(&reopened)?.is_empty());
                    }
                }
            }
        }
    }
    Ok(())
}
#[test]
fn explicit_parametered_projection_is_supported_with_or_without_canonical_adapter() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "formal",
        "parameteredElement",
        json!({"$ref":"formal-type"}),
    )?;
    author(&mut project, "actual", "isAbstract", json!(true))?;
    expect(&project, ERROR)?;
    project
        .relations
        .retain(|relation| relation.id != "parametered-formal");
    expect(&project, ERROR)?;
    author(&mut project, "formal-type", "isAbstract", json!(true))?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn decisive_operand_can_prove_implication_despite_external_other_operand() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"href":"external.uml#Formal"}),
    )?;
    assert!(issues(&project)?.is_empty());
    author(&mut project, "actual", "isAbstract", json!(true))?;
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
    author(&mut project, "formal-type", "isAbstract", json!(true))?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn unrelated_formal_kind_does_not_cast_nonclassifier_actual_to_classifier() -> Result {
    let mut project = document(true)?;
    kind(&mut project, "formal", "uml.template_parameter")?;
    kind(&mut project, "actual", "uml.operation")?;
    assert!(issues(&project)?.is_empty());
    kind(&mut project, "formal", "uml.unresolved_parameter")?;
    expect(&project, UNKNOWN)?;
    kind(&mut project, "formal", "uml.classifier_template_parameter")?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn malformed_boolean_operands_dominate_a_decisive_other_operand() -> Result {
    for id in ["formal-type", "actual"] {
        for value in [Value::Null, json!(42), json!("true")] {
            let mut project = document(true)?;
            author(&mut project, "formal-type", "isAbstract", json!(true))?;
            author(&mut project, "actual", "isAbstract", json!(false))?;
            author(&mut project, id, "isAbstract", value)?;
            expect(&project, INVALID)?;
        }
    }
    Ok(())
}
#[test]
fn unresolved_metaclasses_remain_unknown_unless_other_operand_decides() -> Result {
    let mut project = document(true)?;
    kind(&mut project, "formal-type", "uml.unresolved_classifier")?;
    assert!(issues(&project)?.is_empty());
    author(&mut project, "actual", "isAbstract", json!(true))?;
    expect(&project, UNKNOWN)?;
    kind(&mut project, "formal-type", "uml.class")?;
    kind(&mut project, "actual", "uml.unresolved_classifier")?;
    expect(&project, UNKNOWN)?;
    author(&mut project, "formal-type", "isAbstract", json!(true))?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn canonical_ambiguity_and_projection_identity_conflict_are_invalid_before_type_resolution()
-> Result {
    let mut project = document(true)?;
    project.relations.push(serde_json::from_value(json!({"id":"duplicate","kind":"uml.parametered_element","from":"formal","to":"formal-type"}))?);
    expect(&project, INVALID)?;
    let mut project = document(true)?;
    kind(&mut project, "foreign-type", "uml.unresolved_classifier")?;
    author(
        &mut project,
        "formal",
        "parameteredElement",
        json!({"$ref":"foreign-type"}),
    )?;
    expect(&project, INVALID)?;
    let mut project = document(true)?;
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == "parametered-formal")
        .ok_or("relation")?
        .to = "root".into();
    author(
        &mut project,
        "formal",
        "parameteredElement",
        json!({"href":"external.uml#Classifier"}),
    )?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn absent_parametered_projection_is_not_replaced_by_owned_role_guess() -> Result {
    let mut project = document(true)?;
    project
        .relations
        .retain(|relation| relation.id != "parametered-formal");
    author(
        &mut project,
        "formal",
        "ownedParameteredElement",
        json!({"$ref":"formal-type"}),
    )?;
    author(&mut project, "actual", "isAbstract", json!(true))?;
    expect(&project, UNKNOWN)?;
    author(&mut project, "actual", "isAbstract", json!(false))?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn malformed_references_are_invalid_even_when_actual_is_nonabstract() -> Result {
    for (id, property, value) in [
        ("substitution", "formal", json!(42)),
        ("formal", "parameteredElement", Value::Null),
        ("formal", "parameteredElement", json!({"$ref":"missing"})),
        ("formal", "parameteredElement", json!({"$ref":"root"})),
        ("substitution", "actual", json!(42)),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, property, value)?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
#[test]
fn authored_inverse_is_kept_unverified_instead_of_certifying_the_whole_formal() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "formal",
        "templateParameterSubstitution",
        json!([{"href":"external.uml#Substitution"}]),
    )?;
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    assert!(
        response["diagnostics"]
            .as_array()
            .ok_or("diagnostics")?
            .iter()
            .any(|issue| issue["subject_id"] == "formal"
                && issue["code"] == UNKNOWN
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with(RULE)))
    );
    Ok(())
}
#[test]
fn making_actual_abstract_rolls_back_and_reopened_invalid_snapshot_can_be_repaired() -> Result {
    let project = document(true)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "actual".into(),
            path: "properties.isAbstract".into(),
            value: json!(true),
        },
        1,
    )
    .expect_err("abstract actual cannot replace nonabstract formal");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(before, encode(&project, Format::Json)?);
    let mut invalid = project;
    author(&mut invalid, "actual", "isAbstract", json!(true))?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "actual".into(),
                path: "properties.isAbstract".into(),
                value: json!(false),
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}
