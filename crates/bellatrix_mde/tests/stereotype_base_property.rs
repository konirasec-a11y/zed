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
fn base_identity_uses_extension_membership_and_ownership_not_name() -> Result {
    let mut model = document()?;
    author(&mut model, "base", "multiplicity", json!("0..2"))?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "base")
        .ok_or("base")?
        .name = "anything".into();
    expect(&model, ERROR)?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "extension")
        .ok_or("extension")?
        .kind = "uml.association".into();
    assert!(issues(&model)?.is_empty());
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "extension")
        .ok_or("extension")?
        .kind = "uml.extension".into();
    model
        .relations
        .iter_mut()
        .find(|relation| relation.id == "owns-base")
        .ok_or("owner")?
        .from = "class".into();
    assert!(issues(&model)?.is_empty());
    Ok(())
}
#[test]
fn standalone_base_named_attribute_has_no_extension_obligation() -> Result {
    let mut model = document()?;
    model
        .elements
        .retain(|element| element.id != "extension" && element.id != "end");
    model
        .relations
        .retain(|relation| relation.to != "extension" && relation.to != "end");
    author(&mut model, "base", "multiplicity", json!("0..*"))?;
    assert!(issues(&model)?.is_empty());
    Ok(())
}
#[test]
fn external_upper_is_unknown_and_malformed_bounds_are_invalid() -> Result {
    let mut model = document()?;
    author(
        &mut model,
        "base",
        "upperValue",
        json!({"href":"external.uml#Upper"}),
    )?;
    expect(&model, UNKNOWN)?;
    for value in [
        json!(false),
        json!({"$ref":"missing"}),
        json!({"$ref":"class"}),
        json!({"href":false}),
    ] {
        author(&mut model, "base", "upperValue", value)?;
        expect(&model, ERROR)?;
    }
    Ok(())
}
#[test]
fn known_bad_upper_dominates_unknown_member_but_invalid_members_always_win() -> Result {
    for reverse in [false, true] {
        let mut model = document()?;
        let mut members = vec![json!({"$ref":"base"}), json!({"href":"external.uml#End"})];
        if reverse {
            members.reverse();
        }
        author(&mut model, "extension", "memberEnd", json!(members))?;
        expect(&model, UNKNOWN)?;
        author(&mut model, "base", "multiplicity", json!("0..2"))?;
        expect(&model, ERROR)?;
        author(&mut model, "base", "multiplicity", json!("1"))?;
        author(
            &mut model,
            "extension",
            "memberEnd",
            json!([{"$ref":"base"},false]),
        )?;
        expect(&model, ERROR)?;
    }
    Ok(())
}
#[test]
fn unresolved_extension_membership_is_not_invented_by_stored_inverse() -> Result {
    let mut model = document()?;
    author(&mut model, "base", "multiplicity", json!("0..2"))?;
    author(
        &mut model,
        "base",
        "association",
        json!({"$ref":"extension"}),
    )?;
    author(
        &mut model,
        "extension",
        "memberEnd",
        json!([{"href":"external.uml#Base"},{"$ref":"end"}]),
    )?;
    expect(&model, UNKNOWN)?;
    author(
        &mut model,
        "extension",
        "memberEnd",
        json!([{"$ref":"end"},{"$ref":"base"}]),
    )?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn literal_upper_uses_canonical_value_and_a_multiplicity_cache_cannot_override_it() -> Result {
    let mut model = document()?;
    model.elements.push(serde_json::from_value(json!({"id":"upper","kind":"uml.literal_unlimited_natural","name":"","properties":{"value":2}}))?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-upper","kind":"owns","from":"base","to":"upper"}),
    )?);
    author(&mut model, "base", "upperValue", json!({"$ref":"upper"}))?;
    author(&mut model, "base", "upper", json!(1))?;
    expect(&model, ERROR)?;
    author(&mut model, "upper", "value", json!(1))?;
    assert!(issues(&model)?.is_empty());
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

#[test]
fn known_zero_upper_dominates_unknown_owned_attribute() -> Result {
    for reverse in [false, true] {
        let mut model = document()?;
        model.elements.push(serde_json::from_value(
            json!({"id":"unknown","kind":"uml.unknown_attribute","name":"Unknown"}),
        )?);
        model.relations.push(serde_json::from_value(
            json!({"id":"owns-unknown","kind":"owns","from":"stereotype","to":"unknown"}),
        )?);
        if reverse {
            model.relations.reverse();
        }
        expect(&model, UNKNOWN)?;
        author(&mut model, "base", "multiplicity", json!("0"))?;
        expect(&model, ERROR)?;
    }
    Ok(())
}
