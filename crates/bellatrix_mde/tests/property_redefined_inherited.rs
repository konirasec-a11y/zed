use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PROPERTY_REDEFINED_PROPERTY_INHERITED";
const UNKNOWN: &str = "MDE_PROPERTY_REDEFINED_PROPERTY_INHERITED_UNVERIFIED";
fn document(diamond: bool) -> Result<Project> {
    let mut value = json!({
        "id":"project", "name":"Redefinitions", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"base","kind":"uml.class","name":"Base"},
            {"id":"middle","kind":"uml.class","name":"Middle"},
            {"id":"leaf","kind":"uml.class","name":"Leaf"},
            {"id":"other","kind":"uml.class","name":"Other"},
            {"id":"type","kind":"uml.class","name":"Type"},
            {"id":"original","kind":"uml.attribute","name":"original", "properties":{"type":{"$ref":"type"}}},
            {"id":"replacement","kind":"uml.attribute","name":"replacement", "properties":{"type":{"$ref":"type"},"redefinedProperty":[{"$ref":"original"}]}},
            {"id":"last","kind":"uml.attribute","name":"last", "properties":{"type":{"$ref":"type"}}}
        ],
        "relations":[
            {"id":"middle-base","kind":"uml.generalization","from":"middle","to":"base"},
            {"id":"leaf-middle","kind":"uml.generalization","from":"leaf","to":"middle"}
        ]
    });
    for (owner, child) in [
        ("root", "base"),
        ("root", "middle"),
        ("root", "leaf"),
        ("root", "other"),
        ("root", "type"),
        ("base", "original"),
        ("middle", "replacement"),
        ("leaf", "last"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({
                "id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child
            }));
    }
    if diamond {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .extend([
                json!({"id":"other-base","kind":"uml.generalization","from":"other","to":"base"}),
                json!({"id":"leaf-other","kind":"uml.generalization","from":"leaf","to":"other"}),
            ]);
    }
    Ok(decode(&serde_json::to_vec(&value)?, Format::Json)?)
}

fn has(model: &Project, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .any(|d| d["subject_id"] == "replacement" && d["code"] == code))
}
fn pass(model: &Project) -> Result {
    assert!(!has(model, ERROR)?);
    assert!(!has(model, UNKNOWN)?);
    Ok(())
}
#[test]
fn direct_transitive_and_diamond_features_survive_roundtrip() -> Result {
    for diamond in [false, true] {
        let mut model = document(diamond)?;
        pass(&model)?;
        model
            .relations
            .iter_mut()
            .find(|r| r.id == "owns-replacement")
            .ok_or("owner")?
            .from = "leaf".into();
        pass(&model)?;
        for format in [Format::Json, Format::Protobuf] {
            pass(&decode(&encode(&model, format)?, format)?)?;
        }
    }
    Ok(())
}

#[test]
fn moving_redefiner_is_rejected_by_existing_context_gate_and_preserves_snapshot() -> Result {
    let model = document(false)?;
    let before = serde_json::to_value(&model)?;
    let error = apply(
        &model,
        &Command::SetOwner {
            element_id: "replacement".into(),
            owner_id: Some("other".into()),
        },
        1,
    )
    .expect_err("must reject");
    assert!(
        error.contains("UML_NAMESPACE_DISTINGUISHABILITY"),
        "{error}"
    );
    assert_eq!(before, serde_json::to_value(&model)?);
    Ok(())
}
