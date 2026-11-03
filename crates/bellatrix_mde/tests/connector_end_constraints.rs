use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project};
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
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, property: &str, value: Value) {
    model.elements.iter_mut().find(|e| e.id == id).expect("element").properties.insert(property.into(), value);
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    let value: Value = serde_json::from_str(&validate_uml_model(&json!({"api_version":1,"model":model}).to_string())?)?;
    Ok(value["diagnostics"].as_array().ok_or("diagnostics")?.clone())
}
fn has(model: &Project, rule: &str, severity: &str) -> Result<bool> {
    Ok(issues(model)?.iter().any(|diagnostic| diagnostic["subject_id"] == "e1"
        && diagnostic["severity"] == severity
        && diagnostic["message"].as_str().is_some_and(|message| message.starts_with(&format!("ConnectorEnd-{rule}:")))))
}
fn attach_to_part(model: &mut Project) {
    author(model, "e1", "role", json!({"$ref":"port"}));
    author(model, "e1", "partWithPort", json!({"$ref":"r2"}));
}

#[test]
fn given_port_on_typed_part_when_validated_then_namespace_membership_is_required() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format)?, format)?;
        for rule in ["role_and_part_with_port", "part_with_port_empty", "self_part_with_port"] {
            assert!(!has(&reopened, rule, "error")? && !has(&reopened, rule, "warning")?, "{:?}", issues(&reopened)?);
        }
    }
    author(&mut model, "r2", "type", json!({"$ref":"foreign"}));
    assert!(has(&model, "role_and_part_with_port", "error")?);
    author(&mut model, "r2", "type", json!({"href":"external.uml#Type"}));
    assert!(has(&model, "role_and_part_with_port", "warning")?);
    author(&mut model, "e1", "role", json!({"$ref":"outside"}));
    assert!(has(&model, "role_and_part_with_port", "error")?, "non-Port is decisive beside unknown Type");
    Ok(())
}

#[test]
fn given_inherited_port_when_visibility_changes_then_membership_obeys_namespace_graph() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    model.relations.iter_mut().find(|relation| relation.id == "owns-port").ok_or("port owner")?.from = "parent".into();
    model.relations.push(serde_json::from_value(json!({"id":"type-parent","kind":"uml.generalization","from":"type","to":"parent"}))?);
    author(&mut model, "port", "visibility", json!("public"));
    assert!(!has(&model, "role_and_part_with_port", "error")? && !has(&model, "role_and_part_with_port", "warning")?);
    author(&mut model, "port", "visibility", json!("private"));
    assert!(has(&model, "role_and_part_with_port", "error")?);
    author(&mut model, "port", "visibility", json!("protected"));
    assert!(!has(&model, "role_and_part_with_port", "error")?);
    model.relations.push(serde_json::from_value(json!({"id":"cycle","kind":"uml.generalization","from":"parent","to":"type"}))?);
    assert!(has(&model, "role_and_part_with_port", "error")?);
    Ok(())
}

#[test]
fn given_containing_classifier_port_then_part_must_be_empty_and_inverse_cannot_override_owner() -> Result {
    let mut model = document()?;
    model.relations.iter_mut().find(|relation| relation.id == "owns-port").ok_or("port owner")?.from = "owner".into();
    author(&mut model, "e1", "role", json!({"$ref":"port"}));
    assert!(!has(&model, "part_with_port_empty", "error")?);
    author(&mut model, "e1", "partWithPort", json!({"$ref":"r2"}));
    assert!(has(&model, "part_with_port_empty", "error")?);
    author(&mut model, "e1", "connector", json!({"href":"external.uml#Connector"}));
    assert!(has(&model, "part_with_port_empty", "error")?);
    author(&mut model, "e1", "partWithPort", json!({"href":"external.uml#Part"}));
    assert!(has(&model, "part_with_port_empty", "error")?, "known nonempty part cannot hide behind unresolved reference");
    author(&mut model, "e1", "partWithPort", Value::Null);
    assert!(!has(&model, "part_with_port_empty", "error")?);
    Ok(())
}

#[test]
fn given_part_with_port_then_port_parts_are_rejected_and_unknown_parts_stay_unknown() -> Result {
    let mut model = document()?;
    for absent in [Value::Null, json!({"$ref":"r2"})] {
        author(&mut model, "e1", "partWithPort", absent);
        assert!(!has(&model, "self_part_with_port", "error")?);
    }
    author(&mut model, "e1", "partWithPort", json!({"$ref":"port"}));
    assert!(has(&model, "self_part_with_port", "error")?);
    author(&mut model, "e1", "partWithPort", json!({"href":"external.uml#Part"}));
    assert!(has(&model, "self_part_with_port", "warning")? && !has(&model, "self_part_with_port", "error")?);
    for malformed in [json!(false), json!({"href":false}), json!({"$ref":"missing"}), json!({"$ref":"connector"})] {
        author(&mut model, "e1", "partWithPort", malformed);
        assert!(has(&model, "self_part_with_port", "error")?);
    }
    Ok(())
}

#[test]
fn given_unknown_input_then_invalid_opposite_input_remains_an_error() -> Result {
    for (unknown, invalid) in [("role", "partWithPort"), ("partWithPort", "role")] {
        let mut model = document()?;
        attach_to_part(&mut model);
        author(&mut model, "e1", unknown, json!({"href":"external.uml#Input"}));
        for malformed in [json!(false), json!({"$ref":"missing"}), json!({"href":false})] {
            author(&mut model, "e1", invalid, malformed);
            assert!(has(&model, "role_and_part_with_port", "error")?);
            assert!(has(&model, "part_with_port_empty", "error")?);
        }
    }
    Ok(())
}

#[test]
fn given_valid_part_port_when_port_moves_outside_type_then_command_rejects_atomically() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    let before = model.clone();
    let error = apply(&model, &Command::SetOwner { element_id:"port".into(), owner_id:Some("foreign".into()) }, 1)
        .expect_err("port must remain a member of the part Type");
    assert!(error.contains("ConnectorEnd-role_and_part_with_port"), "{error}");
    assert_eq!(model, before);
    Ok(())
}

#[test]
fn given_unresolved_port_or_part_type_then_malformed_type_still_wins() -> Result {
    let mut model = document()?;
    attach_to_part(&mut model);
    for unresolved in [Value::Null, json!({"href":"external.uml#Type"})] {
        author(&mut model, "r2", "type", unresolved);
        assert!(has(&model, "role_and_part_with_port", "warning")? && !has(&model, "role_and_part_with_port", "error")?);
    }
    author(&mut model, "e1", "role", json!({"href":"external.uml#Port"}));
    for invalid in [json!(false), json!({"href":false}), json!({"$ref":"missing"}), json!({"$ref":"connector"})] {
        author(&mut model, "r2", "type", invalid);
        assert!(has(&model, "role_and_part_with_port", "error")?);
    }
    Ok(())
}

#[test]
fn given_false_implication_antecedent_then_unknown_other_input_does_not_invent_warning() -> Result {
    let mut model = document()?;
    author(&mut model, "e1", "role", json!({"href":"external.uml#Role"}));
    for absent in [None, Some(Value::Null)] {
        if let Some(absent) = absent {
            author(&mut model, "e1", "partWithPort", absent);
        }
        assert!(!has(&model, "role_and_part_with_port", "warning")? && !has(&model, "role_and_part_with_port", "error")?);
        assert!(!has(&model, "part_with_port_empty", "warning")? && !has(&model, "part_with_port_empty", "error")?);
    }
    author(&mut model, "e1", "role", json!({"$ref":"r1"}));
    author(&mut model, "e1", "partWithPort", json!({"href":"external.uml#Part"}));
    assert!(!has(&model, "part_with_port_empty", "warning")? && !has(&model, "part_with_port_empty", "error")?);
    assert!(has(&model, "role_and_part_with_port", "error")?);
    author(&mut model, "e1", "partWithPort", json!(false));
    assert!(has(&model, "part_with_port_empty", "error")?);
    Ok(())
}

#[test]
fn given_stored_connector_inverse_then_canonical_ownership_cannot_be_overridden() -> Result {
    let mut model = document()?;
    model.elements.push(serde_json::from_value(json!({"id":"other-connector","kind":"uml.connector","name":"Other"}))?);
    for invalid in [json!({"$ref":"other-connector"}), json!(false), json!({"href":false}), json!({"$ref":"missing"}), json!({"$ref":"owner"}), Value::Null] {
        author(&mut model, "e1", "connector", invalid);
        assert!(has(&model, "part_with_port_empty", "error")?);
    }
    for valid in [json!({"$ref":"connector"}), json!({"href":"external.uml#Connector"})] {
        author(&mut model, "e1", "connector", valid);
        assert!(!has(&model, "part_with_port_empty", "error")? && !has(&model, "part_with_port_empty", "warning")?);
    }
    Ok(())
}

#[test]
fn given_invalid_connector_membership_then_external_role_or_empty_part_cannot_hide_it() -> Result {
    for role in [json!({"href":"external.uml#Role"}), json!({"$ref":"r1"})] {
        for part in [Value::Null, json!({"href":"external.uml#Part"})] {
            let mut model = document()?;
            author(&mut model, "e1", "role", role.clone());
            author(&mut model, "e1", "partWithPort", part);
            for membership in [json!(false), json!([{"$ref":"e2"}])] {
                author(&mut model, "connector", "end", membership);
                assert!(has(&model, "part_with_port_empty", "error")? && !has(&model, "part_with_port_empty", "warning")?);
            }
        }
    }
    Ok(())
}
