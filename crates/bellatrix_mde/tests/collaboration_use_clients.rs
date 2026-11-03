use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_COLLABORATION_USE_CLIENT_ELEMENTS";
const UNKNOWN: &str = "MDE_COLLABORATION_USE_CLIENT_ELEMENTS_UNVERIFIED";
fn document(explicit: bool) -> Result<Project> {
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Collaboration roles","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":"uml.class","name":"Owner"},
            {"id":"collaboration","kind":"uml.collaboration","name":"Collaboration","properties":{"collaborationRole":[{"$ref":"role1"}]}},
            {"id":"usage","kind":"uml.collaboration_use","name":"Use","properties":{"type":{"$ref":"collaboration"}}},
            {"id":"client","kind":"uml.property","name":"Client"},
            {"id":"role1","kind":"uml.property","name":"First role"},
            {"id":"role2","kind":"uml.property","name":"Second role"}
        ],"relations":[
            {"id":"owns-owner","kind":"owns","from":"root","to":"owner"},
            {"id":"owns-collaboration","kind":"owns","from":"root","to":"collaboration"},
            {"id":"owns-usage","kind":"owns","from":"owner","to":"usage"},
            {"id":"owns-client","kind":"owns","from":"owner","to":"client"},
            {"id":"owns-role1","kind":"owns","from":"collaboration","to":"role1"},
            {"id":"owns-role2","kind":"owns","from":"collaboration","to":"role2"},
            {"id":"binding","kind":"uml.dependency","from":"client","to":"role1","properties":{"visibility":null}},
            {"id":"owns-binding","kind":"owns","from":"usage","to":"binding"}
        ]}),
    )?;
    if explicit {
        author(
            &mut model,
            "usage",
            "roleBinding",
            json!([{"$ref":"binding"}]),
        )?;
    }
    Ok(model)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) -> Result {
    if let Some(element) = model.elements.iter_mut().find(|element| element.id == id) {
        element.properties.insert(key.into(), value);
    } else {
        model
            .relations
            .iter_mut()
            .find(|relation| relation.id == id)
            .ok_or("subject")?
            .properties
            .insert(key.into(), value);
    }
    Ok(())
}
fn has(model: &Project, code: &str) -> Result<bool> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .any(|issue| issue["subject_id"] == "usage" && issue["code"] == code))
}
fn paired() -> Result<Project> {
    let mut model = document(false)?;
    model.elements.push(serde_json::from_value(
        json!({"id":"client2","kind":"uml.property","name":"Other client"}),
    )?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-client2","kind":"owns","from":"owner","to":"client2"}),
    )?);
    author(
        &mut model,
        "binding",
        "client",
        json!([{"$ref":"client"},{"$ref":"client2"}]),
    )?;
    author(
        &mut model,
        "binding",
        "supplier",
        json!([{"$ref":"role1"},{"$ref":"role2"}]),
    )?;
    author(
        &mut model,
        "collaboration",
        "collaborationRole",
        json!([{"$ref":"role1"},{"$ref":"role2"}]),
    )?;
    Ok(model)
}
fn pass(model: &Project) -> Result {
    assert!(!has(model, ERROR)?);
    assert!(!has(model, UNKNOWN)?);
    Ok(())
}
#[test]
fn matching_collections_and_single_endpoint_roundtrip() -> Result {
    for model in [document(false)?, document(true)?, paired()?] {
        pass(&model)?;
        for format in [Format::Json, Format::Protobuf] {
            pass(&decode(&encode(&model, format)?, format)?)?;
        }
    }
    Ok(())
}
#[test]
fn different_client_contexts_violate_even_with_unknown_neighbor_in_both_orders() -> Result {
    for reverse in [false, true] {
        let mut model = paired()?;
        model
            .relations
            .iter_mut()
            .find(|r| r.id == "owns-client2")
            .ok_or("owns")?
            .from = "collaboration".into();
        let mut endpoints = vec![
            json!({"$ref":"client"}),
            json!({"$ref":"client2"}),
            json!({"href":"external#Client"}),
        ];
        if reverse {
            endpoints.reverse();
        }
        author(&mut model, "binding", "client", json!(endpoints))?;
        assert!(has(&model, ERROR)?);
        assert!(!has(&model, UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn supplier_contexts_use_forward_roles_not_ownership_or_usage_type() -> Result {
    let mut model = paired()?;
    author(
        &mut model,
        "collaboration",
        "collaborationRole",
        json!([{"$ref":"role1"}]),
    )?;
    assert!(has(&model, ERROR)?);
    author(&mut model, "collaboration", "collaborationRole", json!([]))?;
    pass(&model)?;
    author(
        &mut model,
        "usage",
        "type",
        json!({"href":"external#Collaboration"}),
    )?;
    pass(&model)?;
    Ok(())
}
#[test]
fn connectable_elements_can_have_equal_multiple_contexts() -> Result {
    let mut model = paired()?;
    model.elements.push(serde_json::from_value(json!({"id":"other","kind":"uml.collaboration","name":"Other","properties":{"collaborationRole":[{"$ref":"role1"},{"$ref":"role2"}]}}))?);
    model.relations.push(serde_json::from_value(
        json!({"id":"owns-other","kind":"owns","from":"root","to":"other"}),
    )?);
    pass(&model)?;
    author(
        &mut model,
        "other",
        "collaborationRole",
        json!([{"$ref":"role2"}]),
    )?;
    assert!(has(&model, ERROR)?);
    Ok(())
}
#[test]
fn external_endpoints_or_unknown_context_membership_stay_unknown() -> Result {
    let mut model = paired()?;
    author(
        &mut model,
        "binding",
        "client",
        json!([{"$ref":"client"},{"href":"external#Client"}]),
    )?;
    assert!(has(&model, UNKNOWN)?);
    assert!(!has(&model, ERROR)?);
    let mut model = paired()?;
    author(
        &mut model,
        "collaboration",
        "collaborationRole",
        json!([{"$ref":"role1"},{"href":"external#Role"}]),
    )?;
    assert!(has(&model, UNKNOWN)?);
    assert!(!has(&model, ERROR)?);
    Ok(())
}
#[test]
fn nonconnectable_single_endpoint_and_malformed_neighbors_are_errors() -> Result {
    for (id, key, value) in [
        ("binding", "client", json!([{"$ref":"owner"}])),
        ("binding", "supplier", json!([{"$ref":"collaboration"}])),
        ("binding", "client", json!([{"href":"external#Client"},42])),
        (
            "binding",
            "supplier",
            json!([{"$ref":"role1"},{"$ref":"role1"}]),
        ),
        (
            "collaboration",
            "collaborationRole",
            json!([{"$ref":"role1"},42]),
        ),
        (
            "usage",
            "roleBinding",
            json!([{"href":"external#Dependency"},42]),
        ),
    ] {
        let mut model = paired()?;
        author(&mut model, id, key, value)?;
        assert!(has(&model, ERROR)?, "{id}.{key}");
    }
    Ok(())
}
#[test]
fn moving_one_client_to_different_context_is_atomic() -> Result {
    let model = paired()?;
    let before = serde_json::to_value(&model)?;
    let error = apply(
        &model,
        &Command::SetOwner {
            element_id: "client2".into(),
            owner_id: Some("collaboration".into()),
        },
        1,
    )
    .expect_err("must reject");
    assert!(
        error.contains("CollaborationUse-client_elements:"),
        "{error}"
    );
    assert_eq!(before, serde_json::to_value(&model)?);
    Ok(())
}
#[test]
fn authored_opposites_are_not_silently_certified() -> Result {
    for (id, key, value) in [
        ("role1", "collaboration", json!([{"$ref":"collaboration"}])),
        ("role1", "collaboration", json!([])),
        ("client", "structuredClassifier", json!([{"$ref":"owner"}])),
    ] {
        let mut model = paired()?;
        author(&mut model, id, key, value)?;
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?, "{id}.{key}");
    }
    for (id, key) in [
        ("role1", "collaboration"),
        ("client", "structuredClassifier"),
    ] {
        let mut model = paired()?;
        author(&mut model, id, key, json!(42))?;
        assert!(has(&model, ERROR)?);
    }
    Ok(())
}
#[test]
fn pairs_across_bindings_and_dependency_objects_use_same_rule() -> Result {
    for object in [false, true] {
        let mut model = paired()?;
        author(&mut model, "binding", "client", json!([{"$ref":"client"}]))?;
        author(&mut model, "binding", "supplier", json!([{"$ref":"role1"}]))?;
        if object {
            model.elements.push(serde_json::from_value(json!({"id":"binding2","kind":"uml.dependency","name":"Binding","properties":{"visibility":null,"client":[{"$ref":"client2"}],"supplier":[{"$ref":"role2"}]}}))?);
        } else {
            model.relations.push(serde_json::from_value(json!({"id":"binding2","kind":"uml.dependency","from":"client2","to":"role2","properties":{"visibility":null}}))?);
        }
        model.relations.push(serde_json::from_value(
            json!({"id":"owns-binding2","kind":"owns","from":"usage","to":"binding2"}),
        )?);
        pass(&model)?;
        let mut draft = serde_json::to_value(&model)?;
        draft.as_object_mut().ok_or("model")?.remove("root_model");
        pass(&serde_json::from_value(draft)?)?;
        model
            .relations
            .iter_mut()
            .find(|r| r.id == "owns-client2")
            .ok_or("owns")?
            .from = "collaboration".into();
        assert!(has(&model, ERROR)?);
    }
    Ok(())
}

#[test]
fn derived_parameter_opposite_still_validates_stored_shape() -> Result {
    let mut model = document(false)?;
    model
        .elements
        .iter_mut()
        .find(|e| e.id == "client")
        .ok_or("client")?
        .kind = "uml.parameter".into();
    author(&mut model, "client", "structuredClassifier", json!(42))?;
    assert!(has(&model, ERROR)?);
    author(&mut model, "client", "structuredClassifier", json!([]))?;
    assert!(has(&model, UNKNOWN)?);
    assert!(!has(&model, ERROR)?);
    Ok(())
}
