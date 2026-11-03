use bellatrix_mde::{commands::{Command, apply}, decode, encode, validate_uml_model, Format, Project};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn document(owner_kind: &str) -> Result<Project> {
    Ok(serde_json::from_value(json!({
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
    }))?)
}
fn diagnostics(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(response["diagnostics"].as_array().ok_or("diagnostics missing")?.clone())
}
fn rule(d: &Value) -> bool {
    d["subject_id"] == "port" && d["message"].as_str().is_some_and(|s| s.starts_with("Port-encapsulated_owner:"))
}

#[test]
fn port_owner_flow_validates_inherited_metaclass_roles_and_reopens() -> Result {
    for kind in ["uml.class", "uml.component", "uml.node", "uml.device", "uml.execution_environment"] {
        let model = document(kind)?;
        let issues = diagnostics(&model)?;
        assert!(!issues.iter().any(rule), "{kind}: {issues:?}");
    }
    for kind in ["uml.package", "uml.interface", "uml.data_type"] {
        let model = document(kind)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            let issues = diagnostics(&reopened)?;
            assert!(issues.iter().any(|d| rule(d) && d["severity"] == "error"), "{kind}: {issues:?}");
        }
    }
    Ok(())
}

#[test]
fn port_owner_flow_rejects_invalid_reparent_atomically_and_repairs_draft() -> Result {
    let mut valid = document("uml.class")?;
    // Interface owns Properties, so the generic ownership adapter accepts the
    // pair. The Port-specific rule, not an earlier adapter error, must reject it.
    valid.elements.iter_mut().find(|e| e.id == "package").ok_or("target")?.kind = "uml.interface".into();
    let before = valid.clone();
    let error = apply(&valid, &Command::SetOwner { element_id:"port".into(), owner_id:Some("package".into()) }, 1)
        .expect_err("Interface cannot be the encapsulated owner");
    assert!(error.contains("Port-encapsulated_owner"), "{error}");
    assert_eq!(before, valid);
    let invalid = document("uml.interface")?;
    let repaired = apply(&invalid, &Command::SetOwner { element_id:"port".into(), owner_id:Some("valid".into()) }, 2)?;
    assert!(!diagnostics(&repaired)?.iter().any(rule));
    Ok(())
}

#[test]
fn port_owner_flow_distinguishes_unresolved_owner_ownerless_and_stored_inverse() -> Result {
    let unknown = document("uml.unknown_classifier")?;
    let issues = diagnostics(&unknown)?;
    assert!(issues.iter().any(|d| rule(d) && d["severity"] == "warning"), "{issues:?}");
    assert!(!issues.iter().any(|d| rule(d) && d["severity"] == "error"));
    let mut unresolved_role = document("uml.class")?;
    unresolved_role.elements.iter_mut().find(|e| e.id == "owner").ok_or("owner")?.properties.insert("ownedAttribute".into(), json!([{"href":"external.uml#Property"}]));
    let issues = diagnostics(&unresolved_role)?;
    assert!(issues.iter().any(|d| rule(d) && d["severity"] == "warning"), "{issues:?}");
    unresolved_role.elements.iter_mut().find(|e| e.id == "owner").ok_or("owner")?.properties.insert("ownedAttribute".into(), json!([{"$ref":"port"}]));
    assert!(!diagnostics(&unresolved_role)?.iter().any(rule));
    let mut ownerless = document("uml.class")?;
    ownerless.relations.retain(|r| r.id != "owns-port");
    let issues = diagnostics(&ownerless)?;
    assert!(!issues.iter().any(rule), "null equality is not a separate owner requirement");
    assert!(issues.iter().any(|d| d["code"] == "UML_ELEMENT_HAS_OWNER" && d["subject_id"] == "port"));
    let mut forged = document("uml.package")?;
    forged.elements.iter_mut().find(|e| e.id == "port").ok_or("Port")?.properties.insert("encapsulatedClassifier".into(), json!({"$ref":"valid"}));
    let issues = diagnostics(&forged)?;
    assert!(issues.iter().any(|d| rule(d) && d["severity"] == "error"), "stored inverse cannot override owns");
    Ok(())
}

#[test]
fn port_owner_membership_is_decisive_beside_external_roles_but_not_invalid_members() -> Result {
    for entries in [
        json!([{"$ref":"port"},{"href":"external.uml#Property"}]),
        json!([{"href":"external.uml#Property"},{"$ref":"port"}]),
    ] {
        let mut model = document("uml.class")?;
        model.elements.iter_mut().find(|e| e.id == "owner").ok_or("owner")?.properties.insert("ownedAttribute".into(), entries);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            let issues = diagnostics(&reopened)?;
            assert!(!issues.iter().any(rule), "local Port proves membership: {issues:?}");
            assert!(issues.iter().any(|d| d["severity"] == "warning"), "unresolved neighbor must remain diagnosed");
        }
    }
    for entries in [
        json!([{"$ref":"port"},{"href":"external.uml#Property"},false]),
        json!([false,{"href":"external.uml#Property"},{"$ref":"port"}]),
        json!([{"$ref":"port"},{"$ref":"port"}]),
        json!([{"$ref":"port"},{"href":"external.uml#Property"},{"href":"external.uml#Property"}]),
    ] {
        let mut model = document("uml.class")?;
        model.elements.iter_mut().find(|e| e.id == "owner").ok_or("owner")?.properties.insert("ownedAttribute".into(), entries);
        let issues = diagnostics(&model)?;
        assert!(issues.iter().any(|d| rule(d) && d["severity"] == "error"), "invalid entries must prevail: {issues:?}");
    }
    Ok(())
}
