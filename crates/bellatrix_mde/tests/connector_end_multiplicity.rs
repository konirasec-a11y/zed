use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project,
    uml::{connectors, references::ReferenceIndex, root::Model}};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_CONNECTOR_END_MULTIPLICITY";
fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Connectors","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":"uml.class","name":"Owner"},
            {"id":"left","kind":"uml.class","name":"Left"},
            {"id":"right","kind":"uml.class","name":"Right"},
            {"id":"leaf","kind":"uml.class","name":"Leaf"},
            {"id":"middle","kind":"uml.class","name":"Middle"},
            {"id":"association","kind":"uml.nary_association","name":"Association","properties":{"memberEnd":[{"$ref":"a1"},{"$ref":"a2"}]}},
            {"id":"a1","kind":"uml.attribute","name":"leftEnd","properties":{"type":{"$ref":"left"}}},
            {"id":"a2","kind":"uml.attribute","name":"rightEnd","properties":{"type":{"$ref":"right"}}},
            {"id":"r1","kind":"uml.attribute","name":"leftRole","properties":{"type":{"$ref":"leaf"}}},
            {"id":"r2","kind":"uml.attribute","name":"rightRole","properties":{"type":{"$ref":"right"}}},
            {"id":"connector","kind":"uml.connector","name":"Connection","properties":{"type":{"$ref":"association"},"end":[{"$ref":"e1"},{"$ref":"e2"}]}},
            {"id":"e1","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r1"}}},
            {"id":"e2","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r2"}}}
        ],"relations":[{"id":"specialization","kind":"uml.generalization","from":"leaf","to":"middle"},
            {"id":"middle-specialization","kind":"uml.generalization","from":"middle","to":"left"}]
    });
    for (owner, child) in [("root","owner"),("root","left"),("root","right"),("root","leaf"),("root","middle"),("root","association"),("association","a1"),("association","a2"),("owner","r1"),("owner","r2"),("owner","connector"),("connector","e1"),("connector","e2")] {
        value["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) {
    model.elements.iter_mut().find(|e| e.id == id).expect("element").properties.insert(key.into(), value);
}
fn diagnostics(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(response["diagnostics"].as_array().ok_or("diagnostics")?.clone())
}
fn has(model: &Project, end: &str, code: &str) -> Result<bool> {
    Ok(diagnostics(model)?.iter().any(|diagnostic| diagnostic["subject_id"] == end && diagnostic["code"] == code))
}
fn ranges(model: &mut Project) {
    for (id, range) in [("a1", "0..3"), ("a2", "2..5"), ("e1", "1..2"), ("e2", "3..4")] {
        author(model, id, "multiplicity", json!(range));
    }
}
fn target(model: &Project, end: &str) -> Result<Option<String>> {
    let references = ReferenceIndex::new(Model::from_project(model));
    Ok(connectors::defining_end(&references, references.element(end)?)?.map(|element| element.id().to_owned()))
}

#[test]
fn given_ordered_ends_then_defining_end_tracks_positions_not_ownership_order_or_cache() -> Result {
    let mut model = document()?;
    ranges(&mut model);
    assert_eq!(target(&model, "e1")?.as_deref(), Some("a1"));
    assert_eq!(target(&model, "e2")?.as_deref(), Some("a2"));
    model.relations.reverse();
    author(&mut model, "e1", "definingEnd", json!({"$ref":"a2"}));
    assert_eq!(target(&model, "e1")?.as_deref(), Some("a1"));
    assert!(!has(&model, "e1", ERROR)? && !has(&model, "e2", ERROR)?);
    author(&mut model, "connector", "end", json!([{"$ref":"e2"},{"$ref":"e1"}]));
    assert_eq!(target(&model, "e1")?.as_deref(), Some("a2"));
    assert!(has(&model, "e1", ERROR)? && has(&model, "e2", ERROR)?);
    author(&mut model, "association", "memberEnd", json!([{"$ref":"a2"},{"$ref":"a1"}]));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        assert_eq!(target(&reopened, "e1")?.as_deref(), Some("a1"));
        assert!(!has(&reopened, "e1", ERROR)? && !has(&reopened, "e2", ERROR)?);
    }
    Ok(())
}

#[test]
fn when_connector_end_multiplicity_expands_then_command_rolls_back_and_draft_can_be_repaired() -> Result {
    let mut model = document()?;
    ranges(&mut model);
    let before = model.clone();
    let error = apply(&model, &Command::SetProperty { element_id:"e1".into(), path:"properties.multiplicity".into(), value:json!("0..4") }, 1)
        .expect_err("end exceeds corresponding Association end");
    assert!(error.contains("ConnectorEnd-multiplicity"), "{error}");
    assert_eq!(model, before);
    author(&mut model, "e1", "multiplicity", json!("0..4"));
    let repaired = apply(&model, &Command::SetProperty { element_id:"e1".into(), path:"properties.multiplicity".into(), value:json!("1..2") }, 2)?;
    assert!(!has(&repaired, "e1", ERROR)?);
    Ok(())
}
