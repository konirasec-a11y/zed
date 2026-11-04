use bellatrix_mde::commands::{Command, apply};
use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "TemplateParameter-must_be_compatible:";
const ERROR: &str = "UML_TEMPLATE_DEFAULT_COMPATIBLE";
const UNKNOWN: &str = "MDE_TEMPLATE_DEFAULT_COMPATIBLE_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Bindings","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"host","kind":"uml.package","name":"Host"},
        {"id":"target","kind":"uml.package","name":"Target"},
        {"id":"signature","kind":"uml.template_signature","name":"Signature","properties":{"parameter":[{"$ref":"formal"}]}},
        {"id":"other-signature","kind":"uml.template_signature","name":"Other signature","properties":{"parameter":[{"$ref":"foreign"}]}},
        {"id":"formal","kind":"uml.template_parameter","name":"Formal","properties":{"default":{"$ref":"actual"},"ownedParameteredElement":{"$ref":"formal-type"}}},
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
            issue["subject_id"] == "formal"
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
fn default_uses_base_metaclass_compatibility_in_root_and_draft() -> Result {
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
fn editing_default_is_rejected_without_mutation_and_snapshots_can_be_repaired() -> Result {
    let project = document(true)?;
    let before = encode(&project, Format::Json)?;
    let error = apply(
        &project,
        &Command::SetProperty {
            element_id: "formal".into(),
            path: "properties.default".into(),
            value: json!({"$ref":"host"}),
        },
        1,
    )
    .expect_err("default editor lacks reference adapter");
    assert!(
        error.contains("read-only") || error.contains("not declared"),
        "{error}"
    );
    assert_eq!(before, encode(&project, Format::Json)?);
    let mut invalid = project;
    author(&mut invalid, "formal", "default", json!({"$ref":"host"}))?;
    expect(&invalid, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(&mut reopened, "formal", "default", json!({"$ref":"actual"}))?;
        assert!(issues(&reopened)?.is_empty());
    }
    Ok(())
}
