use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_ASSOCIATION_CLASS_CANNOT_BE_DEFINED";
const UNKNOWN: &str = "MDE_ASSOCIATION_CLASS_UNVERIFIED";

fn document() -> Result<Project> {
    Ok(serde_json::from_value(json!({
        "id":"model", "name":"Association classes", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"left","kind":"uml.class","name":"Left"},
            {"id":"right","kind":"uml.class","name":"Right"},
            {"id":"association","kind":"uml.association_class","name":"Link",
             "properties":{"memberEnd":[{"$ref":"end-left"},{"$ref":"end-right"}]}},
            {"id":"end-left","kind":"uml.attribute","name":"left","properties":{"type":{"$ref":"left"}}},
            {"id":"end-right","kind":"uml.attribute","name":"right","properties":{"type":{"$ref":"right"}}}
        ],
        "relations":[
            {"id":"owns-left","kind":"owns","from":"root","to":"left"},
            {"id":"owns-right","kind":"owns","from":"root","to":"right"},
            {"id":"owns-association","kind":"owns","from":"root","to":"association"},
            {"id":"owns-end-left","kind":"owns","from":"association","to":"end-left"},
            {"id":"owns-end-right","kind":"owns","from":"association","to":"end-right"}
        ]
    }))?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) {
    model.elements.iter_mut().find(|e| e.id == id).expect("element").properties.insert(key.into(), value);
}
fn diagnostics(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(response["diagnostics"].as_array().ok_or("diagnostics missing")?.clone())
}
fn has(issues: &[Value], code: &str) -> bool {
    issues.iter().any(|d| d["subject_id"] == "association" && d["code"] == code)
}

#[test]
fn association_class_flow_rejects_self_end_atomically_reopens_and_recovers() -> Result {
    let model = document()?;
    let issues = diagnostics(&model)?;
    assert!(!has(&issues, ERROR) && !has(&issues, UNKNOWN), "{issues:?}");
    assert!(has(&issues, "MDE_ASSOCIATION_UNSUPPORTED"), "other specialized contracts are not declared complete");
    let before = model.clone();
    let error = apply(&model, &Command::SetProperty { element_id:"end-left".into(), path:"properties.type".into(), value:json!({"$ref":"association"}) }, 1)
        .expect_err("AssociationClass cannot be an end type of itself");
    assert!(error.contains("AssociationClass-cannot_be_defined"), "{error}");
    assert_eq!(before, model);
    let mut invalid = model.clone();
    author(&mut invalid, "end-left", "type", json!({"$ref":"association"}));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        let issues = diagnostics(&reopened)?;
        assert!(has(&issues, ERROR));
        assert_eq!(issues.iter().filter(|d| d["subject_id"] == "association"
            && d["message"].as_str().is_some_and(|m| m.starts_with("AssociationClass-cannot_be_defined:"))).count(), 1, "{issues:?}");
        let mut repaired = reopened;
        author(&mut repaired, "end-left", "type", json!({"$ref":"left"}));
        let repaired = decode(&encode(&repaired, format)?, format)?;
        assert!(!has(&diagnostics(&repaired)?, ERROR));
        let remaining = apply(&repaired, &Command::SetProperty { element_id:"end-left".into(), path:"properties.type".into(), value:json!({"$ref":"right"}) }, 2)
            .expect_err("other AssociationClass specialized contracts remain unsupported");
        assert!(remaining.contains("MDE_ASSOCIATION_UNSUPPORTED"), "{remaining}");
    }
    Ok(())
}

#[test]
fn association_class_flow_keeps_unknowns_without_hiding_proven_violation_or_invalid_members() -> Result {
    let mut model = document()?;
    author(&mut model, "end-right", "type", json!({"href":"external.uml#Type"}));
    let issues = diagnostics(&model)?;
    assert!(has(&issues, UNKNOWN) && !has(&issues, ERROR), "{issues:?}");
    author(&mut model, "end-left", "type", json!({"$ref":"association"}));
    assert!(has(&diagnostics(&model)?, ERROR));
    for members in [
        json!([{"$ref":"end-left"},{"href":"external.uml#End"}]),
        json!([{"href":"external.uml#End"},{"$ref":"end-left"}]),
    ] {
        author(&mut model, "association", "memberEnd", members);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&diagnostics(&reopened)?, ERROR));
        }
    }
    author(&mut model, "end-left", "type", json!({"$ref":"left"}));
    for members in [json!([{"href":"external.uml#End"},false]), json!([false,{"href":"external.uml#End"}])] {
        author(&mut model, "association", "memberEnd", members);
        assert!(has(&diagnostics(&model)?, ERROR));
    }
    Ok(())
}
