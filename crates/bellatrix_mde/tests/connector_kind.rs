use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project,
    uml::{connectors::{self, ConnectorKind}, classifiers::QueryError, references::ReferenceIndex, root::Model}};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn document() -> Result<Project> {
    let mut value = json!({"id":"model","name":"Roles","root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":"uml.class","name":"Owner"},
            {"id":"parent","kind":"uml.class","name":"Parent"},
            {"id":"foreign","kind":"uml.class","name":"Foreign"},
            {"id":"type","kind":"uml.class","name":"Type"},
            {"id":"r1","kind":"uml.attribute","name":"inherited","properties":{"type":{"$ref":"type"},"visibility":"public"}},
            {"id":"r2","kind":"uml.attribute","name":"direct","properties":{"type":{"$ref":"type"}}},
            {"id":"outside","kind":"uml.attribute","name":"outside","properties":{"type":{"$ref":"type"}}},
            {"id":"port","kind":"uml.port","name":"Port","properties":{"type":{"$ref":"type"}}},
            {"id":"connector","kind":"uml.connector","name":"Connector","properties":{"end":[{"$ref":"e1"},{"$ref":"e2"}]}},
            {"id":"e1","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r1"}}},
            {"id":"e2","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r2"}}}
        ],
        "relations":[{"id":"general","kind":"uml.generalization","from":"owner","to":"parent"}]
    });
    for (owner, child) in [("root","owner"),("root","parent"),("root","foreign"),("root","type"),("parent","r1"),("owner","r2"),("foreign","outside"),("type","port"),("owner","connector"),("connector","e1"),("connector","e2")] {
        value["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    value["elements"].as_array_mut().ok_or("elements")?.iter_mut()
        .find(|element| element["id"] == "e1").ok_or("e1")?["properties"]["role"] = json!({"$ref":"port"});
    value["relations"].as_array_mut().ok_or("relations")?.iter_mut()
        .find(|relation| relation["id"] == "owns-port").ok_or("port ownership")?["from"] = json!("owner");
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, property: &str, value: Value) {
    model.elements.iter_mut().find(|e| e.id == id).expect("element").properties.insert(property.into(), value);
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    let value: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(value["diagnostics"].as_array().ok_or("diagnostics")?.clone())
}
fn kind(model: &Project) -> std::result::Result<ConnectorKind, QueryError> {
    let references = ReferenceIndex::new(Model::from_project(model));
    connectors::kind(&references, references.element("connector").expect("connector"))
}
fn diagnostic(model: &Project, code: &str) -> Result<bool> {
    Ok(issues(model)?.iter().any(|issue| issue["subject_id"] == "connector" && issue["code"] == code))
}

#[test]
fn given_non_behavior_port_without_part_then_kind_is_delegation_from_normative_default() -> Result {
    let mut model = document()?;
    assert_eq!(kind(&model)?, ConnectorKind::Delegation);
    assert!(!diagnostic(&model, "UML_CONNECTOR_KIND_DERIVATION")? && !diagnostic(&model, "MDE_CONNECTOR_KIND_UNVERIFIED")?);
    for cache in [json!("assembly"), json!(false), json!({"href":"external.uml#Kind"})] {
        author(&mut model, "connector", "kind", cache);
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(kind(&decode(&encode(&model, format)?, format)?)?, ConnectorKind::Delegation);
        }
    }
    Ok(())
}

#[test]
fn when_behavior_flag_changes_then_query_rederives_without_mutating_inputs_or_cache() -> Result {
    let mut model = document()?;
    author(&mut model, "port", "isBehavior", json!(false));
    let before = model.clone();
    let changed = apply(&model, &Command::SetProperty { element_id:"port".into(), path:"properties.isBehavior".into(), value:json!(true) }, 1)?;
    assert_eq!(kind(&changed)?, ConnectorKind::Assembly);
    assert_eq!(model, before);
    assert_eq!(kind(&model)?, ConnectorKind::Delegation);
    let before_changed = changed.clone();
    assert!(apply(&changed, &Command::SetProperty { element_id:"port".into(), path:"properties.isBehavior".into(), value:json!("true") }, 2).is_err());
    assert_eq!(changed, before_changed);
    assert!(!changed.elements.iter().find(|element| element.id == "connector").ok_or("connector")?.properties.contains_key("kind"));
    Ok(())
}
