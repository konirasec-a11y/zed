use bellatrix_mde::commands::{Command, apply};
use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "ClassifierTemplateParameter-constraining_classifiers_constrain_args:";
const ERROR: &str = "UML_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL";
const INVALID: &str = "MDE_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL_INVALID";
const UNKNOWN: &str = "MDE_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Bindings","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"host","kind":"uml.package","name":"Host"},
        {"id":"target","kind":"uml.package","name":"Target"},
        {"id":"signature","kind":"uml.template_signature","name":"Signature","properties":{"parameter":[{"$ref":"formal"}]}},
        {"id":"other-signature","kind":"uml.template_signature","name":"Other signature","properties":{"parameter":[{"$ref":"foreign"}]}},
        {"id":"formal","kind":"uml.classifier_template_parameter","name":"Formal","properties":{"allowSubstitutable":false,"constrainingClassifier":[]}},
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
fn relation(project: &mut Project, id: &str, kind: &str, from: &str, to: &str) -> Result {
    project.relations.push(serde_json::from_value(
        json!({"id":id,"kind":kind,"from":from,"to":to}),
    )?);
    Ok(())
}
fn constrain(project: &mut Project, target: &str) -> Result {
    author(
        project,
        "formal",
        "constrainingClassifier",
        json!([{"$ref":target}]),
    )
}
fn substitution(project: &mut Project) -> Result {
    project.elements.push(serde_json::from_value(json!({"id":"class-substitution","kind":"uml.substitution","name":"Substitute","properties":{"contract":{"$ref":"formal-type"}}}))?);
    relation(
        project,
        "owns-class-substitution",
        "owns",
        "actual",
        "class-substitution",
    )
}
#[test]
fn identity_conformance_and_unrelated_are_distinct_in_root_and_draft() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        constrain(&mut project, "actual")?;
        assert!(issues(&project)?.is_empty());
        constrain(&mut project, "formal-type")?;
        expect(&project, ERROR)?;
        relation(
            &mut project,
            "general",
            "uml.generalization",
            "actual",
            "formal-type",
        )?;
        assert!(issues(&project)?.is_empty());
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format)?, format)?;
            assert!(issues(&reopened)?.is_empty());
        }
    }
    Ok(())
}
#[test]
fn substitution_requires_allow_flag_and_defaults_to_true() -> Result {
    let mut project = document(true)?;
    constrain(&mut project, "formal-type")?;
    substitution(&mut project)?;
    expect(&project, ERROR)?;
    author(&mut project, "formal", "allowSubstitutable", json!(true))?;
    assert!(issues(&project)?.is_empty());
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "formal")
        .ok_or("formal")?
        .properties
        .remove("allowSubstitutable");
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn all_constraints_are_required_and_false_dominates_external_unknown() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "formal",
        "constrainingClassifier",
        json!([{"$ref":"actual"},{"$ref":"formal-type"},{"href":"external.uml#C"}]),
    )?;
    expect(&project, ERROR)?;
    relation(
        &mut project,
        "general",
        "uml.generalization",
        "actual",
        "formal-type",
    )?;
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn empty_extent_is_true_but_absent_extent_is_unknown() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "actual",
        json!({"href":"external.uml#A"}),
    )?;
    assert!(issues(&project)?.is_empty());
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "formal")
        .ok_or("formal")?
        .properties
        .remove("constrainingClassifier");
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn canonical_constraints_are_supported_and_projection_conflicts_are_invalid() -> Result {
    let mut project = document(true)?;
    project
        .elements
        .iter_mut()
        .find(|e| e.id == "formal")
        .ok_or("formal")?
        .properties
        .remove("constrainingClassifier");
    relation(
        &mut project,
        "constraint",
        "uml.constraining_classifier",
        "formal",
        "actual",
    )?;
    assert!(issues(&project)?.is_empty());
    constrain(&mut project, "actual")?;
    assert!(issues(&project)?.is_empty());
    constrain(&mut project, "formal-type")?;
    kind(&mut project, "formal-type", "uml.unresolved_classifier")?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn malformed_references_flags_and_cycles_remain_invalid() -> Result {
    for (id, key, value) in [
        ("formal", "constrainingClassifier", json!(42)),
        (
            "formal",
            "constrainingClassifier",
            json!([{"$ref":"missing"}]),
        ),
        ("formal", "constrainingClassifier", json!([{"$ref":"root"}])),
        ("formal", "allowSubstitutable", json!(42)),
        ("substitution", "actual", json!(42)),
        ("substitution", "formal", json!(42)),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, key, value)?;
        expect(&project, INVALID)?;
    }
    let mut project = document(true)?;
    constrain(&mut project, "formal-type")?;
    relation(
        &mut project,
        "cycle",
        "uml.generalization",
        "actual",
        "actual",
    )?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn unresolved_types_and_external_references_are_not_false() -> Result {
    for (id, key) in [("substitution", "formal"), ("substitution", "actual")] {
        let mut project = document(true)?;
        constrain(&mut project, "formal-type")?;
        author(&mut project, id, key, json!({"href":"external.uml#E"}))?;
        expect(&project, UNKNOWN)?;
    }
    let mut project = document(true)?;
    constrain(&mut project, "formal-type")?;
    kind(&mut project, "formal-type", "uml.unresolved_classifier")?;
    expect(&project, UNKNOWN)?;
    constrain(&mut project, "actual")?;
    kind(&mut project, "actual", "uml.unresolved_classifier")?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn generic_formals_do_not_acquire_classifier_constraint() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"$ref":"foreign"}),
    )?;
    author(
        &mut project,
        "substitution",
        "actual",
        json!({"$ref":"host"}),
    )?;
    assert!(issues(&project)?.is_empty());
    Ok(())
}
#[test]
fn explicit_inverse_does_not_claim_global_formal_coverage() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "formal",
        "templateParameterSubstitution",
        json!([{"href":"external.uml#S"}]),
    )?;
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    assert!(
        response["diagnostics"]
            .as_array()
            .ok_or("diagnostics")?
            .iter()
            .any(|issue| issue["subject_id"] == "formal" && issue["code"] == UNKNOWN)
    );
    Ok(())
}
#[test]
fn disabling_substitution_rolls_back_and_invalid_snapshot_can_be_repaired() -> Result {
    let mut project = document(true)?;
    constrain(&mut project, "formal-type")?;
    relation(
        &mut project,
        "constraint",
        "uml.constraining_classifier",
        "formal",
        "formal-type",
    )?;
    substitution(&mut project)?;
    author(&mut project, "formal", "allowSubstitutable", json!(true))?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "formal".into(),
            path: "properties.allowSubstitutable".into(),
            value: json!(false),
        },
        1,
    )
    .expect_err("actual needs Substitution alternative");
    assert!(error.contains(RULE), "{error}");
    assert_eq!(before, encode(&project, Format::Json)?);
    author(&mut project, "formal", "allowSubstitutable", json!(false))?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format)?, format)?;
        expect(&reopened, ERROR)?;
        let repaired = apply(
            &reopened,
            &Command::SetProperty {
                element_id: "formal".into(),
                path: "properties.allowSubstitutable".into(),
                value: json!(true),
            },
            2,
        )?;
        assert!(issues(&repaired)?.is_empty());
    }
    Ok(())
}

#[test]
fn known_conformance_path_survives_external_neighbor_but_not_cycle() -> Result {
    let mut project = document(true)?;
    constrain(&mut project, "formal-type")?;
    relation(
        &mut project,
        "general",
        "uml.generalization",
        "actual",
        "formal-type",
    )?;
    author(
        &mut project,
        "actual",
        "general",
        json!([{"href":"external.uml#Base"}]),
    )?;
    assert!(issues(&project)?.is_empty());
    relation(
        &mut project,
        "cycle",
        "uml.generalization",
        "formal-type",
        "actual",
    )?;
    expect(&project, INVALID)?;
    Ok(())
}
