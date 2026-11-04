use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_uml_model,
};
use bellatrix_mde_contracts::protocol::Command;
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

const RULE: &str = "TemplateSignature-unique_parameters:";
const ERROR: &str = "UML_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS";
const UNKNOWN: &str = "MDE_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS_UNVERIFIED";
fn document(rooted: bool) -> Result<Value> {
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
    import_document(&value)
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
            issue["subject_id"] == "signature"
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
fn kind(project: &mut Value, id: &str, value: &str) -> Result {
    element_mut(project, id)?["kind"] = json!(value);
    Ok(())
}
fn name(project: &mut Value, id: &str, value: &str) -> Result {
    element_mut(project, id)?["name"] = json!(value);
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
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&project, format)?, format)?;
            expect(&reopened, ERROR)?;
        }
    }
    Ok(())
}

#[test]
fn renaming_nonowned_parameter_target_rolls_back_and_invalid_snapshot_repairs() -> Result {
    let project = document(true)?;
    let before = encode(&project, "json")?;
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
    assert_eq!(before, encode(&project, "json")?);
    let mut invalid = project;
    name(&mut invalid, "foreign-type", "FormalType")?;
    for format in ["json", "protobuf"] {
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
