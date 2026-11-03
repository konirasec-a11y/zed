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
fn given_no_delegating_end_then_non_ports_behavior_ports_and_parts_derive_assembly() -> Result {
    let mut model = document()?;
    author(&mut model, "port", "isBehavior", json!(true));
    assert_eq!(kind(&model)?, ConnectorKind::Assembly);
    author(&mut model, "port", "isBehavior", json!(false));
    author(&mut model, "e1", "partWithPort", json!({"$ref":"r2"}));
    assert_eq!(kind(&model)?, ConnectorKind::Assembly);
    author(&mut model, "e1", "partWithPort", Value::Null);
    author(&mut model, "e1", "role", json!({"$ref":"r1"}));
    assert_eq!(kind(&model)?, ConnectorKind::Assembly);
    Ok(())
}

#[test]
fn given_external_sibling_then_known_delegation_decides_exists_in_both_orders() -> Result {
    for ends in [json!([{"$ref":"e1"},{"href":"external.uml#End"}]), json!([{"href":"external.uml#End"},{"$ref":"e1"}])] {
        let mut model = document()?;
        author(&mut model, "connector", "end", ends);
        assert_eq!(kind(&model)?, ConnectorKind::Delegation);
        assert!(!diagnostic(&model, "MDE_CONNECTOR_KIND_UNVERIFIED")?);
        author(&mut model, "port", "isBehavior", json!(true));
        assert!(matches!(kind(&model), Err(QueryError::Unsupported { .. })));
        assert!(diagnostic(&model, "MDE_CONNECTOR_KIND_UNVERIFIED")?);
    }
    Ok(())
}

#[test]
fn given_unknown_endpoint_input_then_false_conjunct_can_decide_without_fabricating_delegation() -> Result {
    let mut model = document()?;
    author(&mut model, "e1", "role", json!({"href":"external.uml#Role"}));
    assert!(matches!(kind(&model), Err(QueryError::Unsupported { .. })));
    author(&mut model, "e1", "partWithPort", json!({"href":"external.uml#Part"}));
    assert_eq!(kind(&model)?, ConnectorKind::Assembly);
    author(&mut model, "e1", "role", json!({"$ref":"r1"}));
    assert_eq!(kind(&model)?, ConnectorKind::Assembly);
    Ok(())
}

#[test]
fn given_known_delegation_then_invalid_sibling_or_input_still_prevents_success() -> Result {
    for (known, malformed) in [("e1", "e2"), ("e2", "e1")] {
        for property in ["role", "partWithPort"] {
            let mut model = document()?;
            author(&mut model, known, "role", json!({"$ref":"port"}));
            author(&mut model, malformed, property, json!(false));
            assert!(matches!(kind(&model), Err(QueryError::Invalid { .. })));
            assert!(diagnostic(&model, "UML_CONNECTOR_KIND_DERIVATION")?);
        }
    }
    for ends in [json!([{"$ref":"e1"},false]), json!([false,{"$ref":"e1"}]), json!([{"$ref":"e1"},{"$ref":"e1"}])] {
        let mut model = document()?;
        author(&mut model, "connector", "end", ends);
        assert!(matches!(kind(&model), Err(QueryError::Invalid { .. })));
    }
    Ok(())
}

#[test]
fn given_port_behavior_value_then_invalid_boolean_is_not_a_default_or_unknown() -> Result {
    for invalid in [Value::Null, json!("false"), json!(0), json!({"href":"external.uml#Behavior"})] {
        let mut model = document()?;
        author(&mut model, "port", "isBehavior", invalid);
        author(&mut model, "e2", "role", json!({"href":"external.uml#Role"}));
        assert!(matches!(kind(&model), Err(QueryError::Invalid { .. })));
        author(&mut model, "e1", "partWithPort", json!({"$ref":"r2"}));
        assert!(matches!(kind(&model), Err(QueryError::Invalid { .. })), "false conjunct cannot hide malformed Boolean");
    }
    Ok(())
}

#[test]
fn given_missing_end_role_then_ownership_and_cache_cannot_prove_assembly() -> Result {
    let mut model = document()?;
    model.elements.iter_mut().find(|element| element.id == "connector").ok_or("connector")?.properties.remove("end");
    author(&mut model, "connector", "kind", json!("assembly"));
    assert!(matches!(kind(&model), Err(QueryError::Unsupported { .. })));
    for malformed in [json!([]), Value::Null, json!(false), json!([{"$ref":"e1"}])] {
        author(&mut model, "connector", "end", malformed);
        assert!(matches!(kind(&model), Err(QueryError::Invalid { .. })));
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
