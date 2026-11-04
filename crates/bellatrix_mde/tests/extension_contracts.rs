use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode,
    uml::{classifiers::QueryError, profiles, references::ReferenceIndex, root::Model},
};
use serde_json::{Value, json};
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
fn author(model: &mut Project, id: &str, key: &str, value: Value) {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .expect("element")
        .properties
        .insert(key.into(), value);
}

fn required(model: &Project) -> std::result::Result<bool, QueryError> {
    let references = ReferenceIndex::new(Model::from_project(model));
    profiles::is_required(
        &references,
        references.element("extension").expect("extension"),
    )
}

#[test]
fn given_owned_extension_end_then_required_follows_lower_bound_and_ignores_derived_caches() -> Result
{
    let mut model = document()?;
    assert_eq!(required(&model)?, false);
    for (range, expected) in [("0..1", false), ("1", true), ("2..2", false)] {
        author(&mut model, "end", "multiplicity", json!(range));
        author(&mut model, "extension", "isRequired", json!(!expected));
        author(
            &mut model,
            "extension",
            "metaclass",
            json!({"$ref":"stereotype"}),
        );
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert_eq!(required(&reopened)?, expected);
        }
    }
    Ok(())
}

#[test]
fn given_partial_extension_support_then_ambiguous_metaclass_end_still_blocks_full_edit_gate()
-> Result {
    let mut model = document()?;
    author(&mut model, "end", "multiplicity", json!("0..1"));
    let before = model.clone();
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "end".into(),
            path: "properties.multiplicity".into(),
            value: json!("1"),
        },
        1,
    )
    .expect_err("non_owned_end still needs normative clarification");
    assert!(
        error.contains("Unsupported Extension-non_owned_end"),
        "{error}"
    );
    assert_eq!(model, before);
    Ok(())
}
