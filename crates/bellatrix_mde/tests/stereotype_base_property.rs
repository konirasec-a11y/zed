use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_STEREOTYPE_BASE_PROPERTY_UPPER_BOUND";
const UNKNOWN: &str = "MDE_STEREOTYPE_BASE_PROPERTY_UPPER_BOUND_UNVERIFIED";
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
fn author(model: &mut Project, id: &str, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "stereotype"
                && (issue["code"] == ERROR || issue["code"] == UNKNOWN)
        })
        .cloned()
        .collect())
}
fn expect(model: &Project, code: &str) -> Result {
    let found = issues(model)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    assert_eq!(
        found[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
#[test]
fn base_property_upper_is_exactly_one_in_rooted_and_rootless_models() -> Result {
    for rooted in [true, false] {
        let mut model = document()?;
        if !rooted {
            model.root_model = None;
        }
        assert!(issues(&model)?.is_empty());
        for multiplicity in ["0..1", "1"] {
            author(&mut model, "base", "multiplicity", json!(multiplicity))?;
            assert!(issues(&model)?.is_empty());
        }
        for multiplicity in ["0", "0..2", "1..*", "0..184467440737095516160"] {
            author(&mut model, "base", "multiplicity", json!(multiplicity))?;
            expect(&model, ERROR)?;
            for format in [Format::Json, Format::Protobuf] {
                expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
            }
        }
    }
    Ok(())
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
