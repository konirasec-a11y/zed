use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_protocol::Command;
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> Result<Value> {
    let response = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&response)?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> Result<Vec<u8>> {
    Ok(encode_model_document(&envelope(model), format)?)
}

fn import_document(model: &Value) -> Result<Value> {
    decode(&serde_json::to_vec(model)?, "json")
}

fn apply(model: &Value, command: &Command, revision: u64) -> std::result::Result<Value, String> {
    let document = prepare_model_document(&envelope(model))?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
        false,
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json").map_err(|error| error.to_string())
}

fn element_mut<'a>(model: &'a mut Value, id: &str) -> Result<&'a mut Value> {
    model["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or_else(|| format!("missing element {id}").into())
}

const RULE: &str = "ClassifierTemplateParameter-constraining_classifiers_constrain_args:";
const ERROR: &str = "UML_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL";
const UNKNOWN: &str = "MDE_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL_UNVERIFIED";
fn document(rooted: bool) -> Result<Value> {
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
    import_document(&value)
}
fn author(project: &mut Value, id: &str, key: &str, value: Value) -> Result {
    element_mut(project, id)?["properties"][key] = value;
    Ok(())
}
fn issues(project: &Value) -> Result<Vec<Value>> {
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
fn expect(project: &Value, code: &str) -> Result {
    let issues = issues(project)?;
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0]["code"], code, "{issues:#?}");
    assert_eq!(
        issues[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
fn relation(project: &mut Value, id: &str, kind: &str, from: &str, to: &str) -> Result {
    project["relations"]
        .as_array_mut()
        .ok_or("relations")?
        .push(json!({"id":id,"kind":kind,"from":from,"to":to}));
    *project = import_document(project)?;
    Ok(())
}
fn constrain(project: &mut Value, target: &str) -> Result {
    author(
        project,
        "formal",
        "constrainingClassifier",
        json!([{"$ref":target}]),
    )
}
fn substitution(project: &mut Value) -> Result {
    project["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .push(json!({"id":"class-substitution","kind":"uml.substitution","name":"Substitute","properties":{"contract":{"$ref":"formal-type"}}}));
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
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&project, format)?, format)?;
            assert!(issues(&reopened)?.is_empty());
        }
    }
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
    let before = encode(&project, "json")?;
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
    assert_eq!(before, encode(&project, "json")?);
    author(&mut project, "formal", "allowSubstitutable", json!(false))?;
    for format in ["json", "protobuf"] {
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
