use bellatrix_mde::{decode_model_document, encode_model_document, validate_uml_model};
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

fn element_mut<'a>(model: &'a mut Value, id: &str) -> Result<&'a mut Value> {
    model["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or_else(|| format!("missing element {id}").into())
}

const RULE: &str = "TemplateBinding-parameter_substitution_formal:";
const ERROR: &str = "UML_TEMPLATE_BINDING_FORMAL";
const UNKNOWN: &str = "MDE_TEMPLATE_BINDING_FORMAL_UNVERIFIED";
fn document(rooted: bool) -> Result<Value> {
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
            issue["subject_id"] == "binding"
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
#[test]
fn substitution_formal_in_target_signature_is_valid_in_rooted_and_draft_models() -> Result {
    for rooted in [true, false] {
        let project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        for format in ["json", "protobuf"] {
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
    for format in ["json", "protobuf"] {
        expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
    }
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
    for format in ["json", "protobuf"] {
        let mut reopened = decode(&encode(&project, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "substitution",
            "formal",
            json!({"$ref":"formal"}),
        )?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened["relations"], project["relations"]);
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
        for format in ["json", "protobuf"] {
            assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
        }
    }
    Ok(())
}
