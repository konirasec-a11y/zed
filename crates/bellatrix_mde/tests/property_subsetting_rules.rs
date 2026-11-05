use bellatrix_mde::Project;
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn document(rooted: bool) -> Result<Project> {
    let mut model = json!({"id":"project","name":"Subsetting rules","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"owner","kind":"uml.class","name":"Owner"},
        {"id":"super","kind":"uml.class","name":"Super"},
        {"id":"middle","kind":"uml.class","name":"Middle"},
        {"id":"sub","kind":"uml.class","name":"Sub"},
        {"id":"general","kind":"uml.attribute","name":"General","properties":{"type":{"$ref":"super"}}},
        {"id":"specific","kind":"uml.attribute","name":"Specific","properties":{"type":{"$ref":"super"},"subsettedProperty":[{"$ref":"general"}]}}
    ],"relations":[
        {"id":"sub-middle","kind":"uml.generalization","from":"sub","to":"middle"},
        {"id":"middle-super","kind":"uml.generalization","from":"middle","to":"super"}
    ]});
    for (owner, child) in [
        ("root", "owner"),
        ("root", "super"),
        ("root", "middle"),
        ("root", "sub"),
        ("owner", "general"),
        ("owner", "specific"),
    ] {
        model["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    if !rooted {
        model.as_object_mut().ok_or("model")?.remove("root_model");
    }
    Ok(serde_json::from_value(model)?)
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

#[test]
fn canonical_subsetting_mutator_rejects_type_or_upper_violation_without_mutation() -> Result {
    use bellatrix_mde::{ModelingSection, uml::property_subsetting};
    for (key, value) in [
        ("type", json!({"$ref":"owner"})),
        ("multiplicity", json!("0..2")),
    ] {
        let mut model = document(true)?;
        author(&mut model, "specific", "subsettedProperty", json!([]))?;
        author(&mut model, "specific", key, value)?;
        let mut section = ModelingSection::from_model(&model);
        let unchanged = serde_json::to_value(&section)?;
        let error = property_subsetting::set_subsetted_properties(
            &mut section,
            "specific",
            &["general".into()],
        )
        .expect_err("subsetting must restrict type and upper");
        assert!(error.contains("subsetting"), "{error}");
        assert_eq!(serde_json::to_value(&section)?, unchanged);
        let property = section
            .elements
            .iter_mut()
            .find(|element| element.id == "specific")
            .ok_or("specific")?;
        property
            .properties
            .insert("type".into(), json!({"$ref":"super"}));
        property
            .properties
            .insert("multiplicity".into(), json!("1"));
        property_subsetting::set_subsetted_properties(
            &mut section,
            "specific",
            &["general".into()],
        )?;
    }
    Ok(())
}
