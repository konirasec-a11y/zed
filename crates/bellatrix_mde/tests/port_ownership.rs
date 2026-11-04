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

fn document(owner_kind: &str) -> Result<Value> {
    import_document(&json!({
        "id":"project","name":"Port ownership","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":owner_kind,"name":"Owner"},
            {"id":"valid","kind":"uml.class","name":"Valid"},
            {"id":"package","kind":"uml.package","name":"Package"},
            {"id":"port","kind":"uml.port","name":"Endpoint","properties":{"aggregation":"composite","visibility":"public"}}
        ],
        "relations":[
            {"id":"root-owner","kind":"owns","from":"root","to":"owner"},
            {"id":"root-valid","kind":"owns","from":"root","to":"valid"},
            {"id":"root-package","kind":"owns","from":"root","to":"package"},
            {"id":"owns-port","kind":"owns","from":"owner","to":"port"}
        ]
    }))
}
fn diagnostics(model: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics missing")?
        .clone())
}
fn rule(d: &Value) -> bool {
    d["subject_id"] == "port"
        && d["message"]
            .as_str()
            .is_some_and(|s| s.starts_with("Port-encapsulated_owner:"))
}

#[test]
fn port_owner_flow_validates_inherited_metaclass_roles_and_reopens() -> Result {
    for kind in [
        "uml.class",
        "uml.component",
        "uml.node",
        "uml.device",
        "uml.execution_environment",
    ] {
        let model = document(kind)?;
        let issues = diagnostics(&model)?;
        assert!(!issues.iter().any(rule), "{kind}: {issues:?}");
    }
    for kind in ["uml.package", "uml.interface", "uml.data_type"] {
        let model = document(kind)?;
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&model, format)?, format)?;
            let issues = diagnostics(&reopened)?;
            assert!(
                issues.iter().any(|d| rule(d) && d["severity"] == "error"),
                "{kind}: {issues:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn port_owner_flow_rejects_invalid_reparent_atomically_and_repairs_draft() -> Result {
    let mut valid = document("uml.class")?;
    // Interface owns Properties, so the generic ownership adapter accepts the
    // pair. The Port-specific rule, not an earlier adapter error, must reject it.
    element_mut(&mut valid, "package")?["kind"] = json!("uml.interface");
    let before = valid.clone();
    let error = apply(
        &valid,
        &Command::SetOwner {
            element_id: "port".into(),
            owner_id: Some("package".into()),
        },
        1,
    )
    .expect_err("Interface cannot be the encapsulated owner");
    assert!(error.contains("Port-encapsulated_owner"), "{error}");
    assert_eq!(before, valid);
    let invalid = document("uml.interface")?;
    let repaired = apply(
        &invalid,
        &Command::SetOwner {
            element_id: "port".into(),
            owner_id: Some("valid".into()),
        },
        2,
    )?;
    assert!(!diagnostics(&repaired)?.iter().any(rule));
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&repaired, format)?, format)?;
        assert_eq!(reopened, repaired);
        assert!(!diagnostics(&reopened)?.iter().any(rule));
    }
    Ok(())
}

#[test]
fn port_owner_membership_is_decisive_beside_external_roles_but_not_invalid_members() -> Result {
    for entries in [
        json!([{"$ref":"port"},{"href":"external.uml#Property"}]),
        json!([{"href":"external.uml#Property"},{"$ref":"port"}]),
    ] {
        let mut model = document("uml.class")?;
        element_mut(&mut model, "owner")?["properties"]["ownedAttribute"] = entries;
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&model, format)?, format)?;
            let issues = diagnostics(&reopened)?;
            assert!(
                !issues.iter().any(rule),
                "local Port proves membership: {issues:?}"
            );
            assert!(
                issues.iter().any(|d| d["severity"] == "warning"),
                "unresolved neighbor must remain diagnosed"
            );
        }
    }
    for entries in [
        json!([{"$ref":"port"},{"href":"external.uml#Property"},false]),
        json!([false,{"href":"external.uml#Property"},{"$ref":"port"}]),
        json!([{"$ref":"port"},{"$ref":"port"}]),
        json!([{"$ref":"port"},{"href":"external.uml#Property"},{"href":"external.uml#Property"}]),
    ] {
        let mut model = document("uml.class")?;
        element_mut(&mut model, "owner")?["properties"]["ownedAttribute"] = entries;
        let issues = diagnostics(&model)?;
        assert!(
            issues.iter().any(|d| rule(d) && d["severity"] == "error"),
            "invalid entries must prevail: {issues:?}"
        );
    }
    Ok(())
}
