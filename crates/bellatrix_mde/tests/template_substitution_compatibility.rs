use bellatrix_mde::commands::{Command, apply};
use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "TemplateParameterSubstitution-must_be_compatible:";
const ERROR: &str = "UML_TEMPLATE_SUBSTITUTION_COMPATIBLE";
const INVALID: &str = "MDE_TEMPLATE_SUBSTITUTION_COMPATIBLE_INVALID";
const UNKNOWN: &str = "MDE_TEMPLATE_SUBSTITUTION_COMPATIBLE_UNVERIFIED";
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
fn base_compatibility_uses_metaclass_subtyping_not_object_identity() -> Result {
    for rooted in [true, false] {
        for (actual, exposed, valid) in [
            ("uml.class", "uml.class", true),
            ("uml.device", "uml.node", true),
            ("uml.node", "uml.device", false),
            ("uml.model", "uml.package", true),
            ("uml.package", "uml.model", false),
            ("uml.interface", "uml.class", false),
        ] {
            let mut project = document(rooted)?;
            kind(&mut project, "actual", actual)?;
            kind(&mut project, "formal-type", exposed)?;
            if valid {
                assert!(issues(&project)?.is_empty());
            } else {
                expect(&project, ERROR)?;
            }
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(&project, format)?, format)?;
                assert_eq!(issues(&project)?, issues(&reopened)?);
            }
        }
    }
    Ok(())
}
#[test]
fn model_generalization_does_not_replace_metaclass_subtyping() -> Result {
    let mut project = document(true)?;
    kind(&mut project, "actual", "uml.class")?;
    kind(&mut project, "formal-type", "uml.interface")?;
    project.relations.push(serde_json::from_value(
        json!({"id":"general","kind":"uml.generalization","from":"actual","to":"formal-type"}),
    )?);
    expect(&project, ERROR)?;
    Ok(())
}
#[test]
fn explicit_and_canonical_exposed_targets_must_agree_even_with_unknown_type() -> Result {
    let mut project = document(true)?;
    author(
        &mut project,
        "formal",
        "parameteredElement",
        json!({"$ref":"formal-type"}),
    )?;
    assert!(issues(&project)?.is_empty());
    project.relations.retain(|r| r.id != "parametered-formal");
    assert!(issues(&project)?.is_empty());
    project.relations.push(serde_json::from_value(json!({"id":"parametered-formal","kind":"uml.parametered_element","from":"formal","to":"foreign-type"}))?);
    kind(&mut project, "foreign-type", "uml.unresolved_parameterable")?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn missing_projection_is_unknown_and_owned_element_does_not_supply_a_role() -> Result {
    let mut project = document(true)?;
    project.relations.retain(|r| r.id != "parametered-formal");
    expect(&project, UNKNOWN)?;
    author(
        &mut project,
        "formal",
        "ownedParameteredElement",
        json!({"$ref":"formal-type"}),
    )?;
    expect(&project, UNKNOWN)?;
    Ok(())
}
#[test]
fn external_references_and_unknown_metaclasses_stay_unknown() -> Result {
    for (id, key) in [
        ("substitution", "actual"),
        ("substitution", "formal"),
        ("formal", "parameteredElement"),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, key, json!({"href":"external.uml#E"}))?;
        expect(&project, UNKNOWN)?;
    }
    for id in ["actual", "formal", "formal-type"] {
        let mut project = document(true)?;
        kind(&mut project, id, "uml.unresolved_parameterable")?;
        expect(&project, UNKNOWN)?;
    }
    Ok(())
}
#[test]
fn malformed_independent_operands_dominate_unknown() -> Result {
    for (id, key, value) in [
        ("substitution", "actual", json!(42)),
        ("substitution", "actual", Value::Null),
        ("substitution", "actual", json!({"$ref":"missing"})),
        ("substitution", "actual", json!({"$ref":"substitution"})),
        ("substitution", "formal", json!(42)),
        (
            "formal",
            "parameteredElement",
            json!({"$ref":"substitution"}),
        ),
    ] {
        let mut project = document(true)?;
        author(&mut project, id, key, value)?;
        expect(&project, INVALID)?;
    }
    let mut project = document(true)?;
    author(
        &mut project,
        "substitution",
        "actual",
        json!({"href":"external.uml#A"}),
    )?;
    author(&mut project, "formal", "parameteredElement", json!(42))?;
    expect(&project, INVALID)?;
    author(
        &mut project,
        "substitution",
        "formal",
        json!({"href":"external.uml#F"}),
    )?;
    author(&mut project, "substitution", "actual", json!(42))?;
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn canonical_wrong_type_and_ambiguous_references_are_invalid() -> Result {
    let mut project = document(true)?;
    project.relations.push(serde_json::from_value(json!({"id":"duplicate","kind":"uml.parametered_element","from":"formal","to":"formal-type"}))?);
    expect(&project, INVALID)?;
    project.relations.retain(|r| r.id != "duplicate");
    project
        .relations
        .iter_mut()
        .find(|r| r.id == "parametered-formal")
        .ok_or("relation")?
        .to = "substitution".into();
    expect(&project, INVALID)?;
    Ok(())
}
#[test]
fn property_and_value_specification_overrides_are_explicitly_unsupported() -> Result {
    for actual in [
        "uml.property",
        "uml.port",
        "uml.literal_integer",
        "uml.expression",
    ] {
        let mut project = document(true)?;
        kind(&mut project, "actual", actual)?;
        kind(&mut project, "formal-type", actual)?;
        expect(&project, UNKNOWN)?;
        assert!(
            issues(&project)?[0]["message"]
                .as_str()
                .ok_or("message")?
                .contains("override")
        );
    }
    Ok(())
}
#[test]
fn changing_reference_remains_blocked_by_existing_authoring_adapter_without_mutation() -> Result {
    let project = document(true)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "substitution".into(),
            path: "properties.actual".into(),
            value: json!({"$ref":"host"}),
        },
        1,
    )
    .expect_err("reference authoring has no adapter");
    assert!(error.contains("read-only") || error.contains("not declared"), "{error}");
    assert_eq!(before, encode(&project, Format::Json)?);
    let mut invalid = project;
    author(
        &mut invalid,
        "substitution",
        "actual",
        json!({"$ref":"host"}),
    )?;
    expect(&invalid, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "substitution",
            "actual",
            json!({"$ref":"actual"}),
        )?;
        assert!(issues(&reopened)?.is_empty());
    }
    Ok(())
}

#[test]
fn canonical_parametered_element_obeys_specialized_formal_range() -> Result {
    for (formal, exposed) in [
        ("uml.classifier_template_parameter", "uml.operation"),
        ("uml.operation_template_parameter", "uml.class"),
    ] {
        let mut project = document(true)?;
        kind(&mut project, "formal", formal)?;
        kind(&mut project, "formal-type", exposed)?;
        kind(&mut project, "actual", exposed)?;
        expect(&project, INVALID)?;
        author(
            &mut project,
            "formal",
            "parameteredElement",
            json!({"$ref":"formal-type"}),
        )?;
        expect(&project, INVALID)?;
    }
    Ok(())
}
