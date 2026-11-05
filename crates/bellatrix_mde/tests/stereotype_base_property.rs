use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    encode,
};
use serde_json::json;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Extension bounds","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"profile","kind":"uml.profile","name":"Profile"},
            {"id":"class","kind":"uml.class","name":"Metaclass"},
            {"id":"stereotype","kind":"uml.stereotype","name":"Stereotype"},
            {"id":"extension","kind":"uml.extension","name":"Extension","properties":{"memberEnd":[{"$ref":"base"},{"$ref":"end"}],"ownedEnd":{"$ref":"end"}}},
            {"id":"base","kind":"uml.attribute","name":"base_Class","properties":{"type":{"$ref":"class"}}},
            {"id":"end","kind":"uml.extension_end","name":"extension_Stereotype","properties":{"type":{"$ref":"stereotype"},"aggregation":"composite"}}
        ],"relations":[]});
    for (owner, child) in [
        ("root", "profile"),
        ("root", "class"),
        ("profile", "stereotype"),
        ("profile", "extension"),
        ("stereotype", "base"),
        ("extension", "end"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}

#[test]
fn unsupported_generic_extension_edit_preserves_the_model() -> Result {
    let model = document()?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "base".into(),
            path: "properties.multiplicity".into(),
            value: json!("0"),
        },
        1,
    )
    .expect_err("base upper must remain one");
    assert!(error.contains("MDE_ASSOCIATION_UNSUPPORTED"), "{error}");
    assert_eq!(encode(&model, Format::Json)?, before);
    Ok(())
}
