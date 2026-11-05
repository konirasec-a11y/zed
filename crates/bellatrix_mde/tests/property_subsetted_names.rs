use bellatrix_mde::Project;
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn document(rooted: bool) -> Result<Project> {
    let mut model = json!({"id":"project","name":"Subsetted property names","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"owner","kind":"uml.class","name":"Owner"},
        {"id":"general","kind":"uml.attribute","name":"General"},
        {"id":"specific","kind":"uml.attribute","name":"Specific","properties":{"subsettedProperty":[{"$ref":"general"}]}}
    ],"relations":[
        {"id":"owns-owner","kind":"owns","from":"root","to":"owner"},
        {"id":"owns-general","kind":"owns","from":"owner","to":"general"},
        {"id":"owns-specific","kind":"owns","from":"owner","to":"specific"}
    ]});
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

fn unknown_member(model: &mut Project) -> Result {
    model.elements.push(serde_json::from_value(
        json!({"id":"unknown","kind":"uml.unknown_property","name":"Unknown"}),
    )?);
    Ok(())
}
#[test]
fn aggregate_retains_unknown_without_exposing_partial_public_collections() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    let mut model = document(true)?;
    unknown_member(&mut model)?;
    author(
        &mut model,
        "specific",
        "subsettedProperty",
        json!([{"$ref":"general"},{"$ref":"unknown"}]),
    )?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert!(matches!(
        property_subsetting::subsetted_properties(&references, references.element("specific")?),
        Err(QueryError::Unsupported { .. })
    ));
    assert!(matches!(
        property_subsetting::compare_subsetting(
            &references,
            references.element("specific")?,
            references.element("unknown")?
        ),
        Err(QueryError::Unsupported { .. })
    ));
    Ok(())
}

#[test]
fn subsetting_mutator_preserves_unknown_drafts_but_rejects_invalid_targets_atomically() -> Result {
    use bellatrix_mde::{ModelingSection, uml::property_subsetting};
    let mut model = document(true)?;
    unknown_member(&mut model)?;
    let mut section = ModelingSection::from_model(&model);
    property_subsetting::set_subsetted_properties(&mut section, "specific", &["unknown".into()])?;
    let unchanged = serde_json::to_value(&section)?;
    for target in ["owner", "missing"] {
        assert!(
            property_subsetting::set_subsetted_properties(
                &mut section,
                "specific",
                &["unknown".into(), target.into()]
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&section)?, unchanged);
    }
    property_subsetting::set_subsetted_properties(&mut section, "specific", &[])?;
    Ok(())
}
