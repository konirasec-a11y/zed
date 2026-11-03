use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_CONNECTOR_TYPES";
const UNKNOWN: &str = "MDE_CONNECTOR_TYPES_UNVERIFIED";
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
fn has(issues: &[Value], code: &str) -> bool { issues.iter().any(|d| d["subject_id"] == "connector" && d["code"] == code) }

#[test]
fn connector_type_flow_checks_position_conformance_rollback_and_reopening() -> Result {
    let model = document()?;
    assert!(!diagnostics(&model)?.iter().any(|d| d["severity"] == "error"), "{:?}", diagnostics(&model)?);
    let before = model.clone();
    let error = apply(&model, &Command::SetProperty { element_id:"r1".into(), path:"properties.type".into(), value:json!({"$ref":"right"}) }, 1).expect_err("wrong role Type must reject");
    assert!(error.contains("Connector-types"), "{error}");
    assert_eq!(model, before);
    let mut invalid = model;
    author(&mut invalid, "connector", "end", json!([{"$ref":"e2"},{"$ref":"e1"}]));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        assert!(has(&diagnostics(&reopened)?, ERROR));
        let mut repaired = reopened;
        author(&mut repaired, "connector", "end", json!([{"$ref":"e1"},{"$ref":"e2"}]));
        assert!(!has(&diagnostics(&repaired)?, ERROR));
    }
    Ok(())
}

#[test]
fn connector_type_flow_obeys_optional_type_and_end_count_without_inventing_order() -> Result {
    let mut model = document()?;
    model.elements.push(serde_json::from_value(json!({"id":"e3","kind":"uml.connector_end","name":"","properties":{"role":{"$ref":"r1"}}}))?);
    model.relations.push(serde_json::from_value(json!({"id":"owns-e3","kind":"owns","from":"connector","to":"e3"}))?);
    author(&mut model, "connector", "end", json!([{"$ref":"e1"},{"$ref":"e2"},{"$ref":"e3"}]));
    assert!(has(&diagnostics(&model)?, ERROR));
    author(&mut model, "connector", "type", Value::Null);
    assert!(!has(&diagnostics(&model)?, ERROR));
    model.elements.iter_mut().find(|e| e.id == "connector").ok_or("Connector")?.properties.remove("type");
    assert!(!has(&diagnostics(&model)?, ERROR));
    author(&mut model, "connector", "type", json!({"$ref":"association"}));
    model.elements.iter_mut().find(|e| e.id == "connector").ok_or("Connector")?.properties.remove("end");
    assert!(has(&diagnostics(&model)?, UNKNOWN), "owns alone does not invent ordered end role");
    Ok(())
}

#[test]
fn connector_type_flow_preserves_unknown_but_decides_other_known_invalid_positions() -> Result {
    let mut model = document()?;
    author(&mut model, "r2", "type", json!({"href":"external.uml#Type"}));
    assert!(has(&diagnostics(&model)?, UNKNOWN));
    assert!(!has(&diagnostics(&model)?, ERROR));
    author(&mut model, "r1", "type", json!({"$ref":"right"}));
    assert!(has(&diagnostics(&model)?, ERROR));
    for ends in [json!([{"href":"external.uml#End"},{"$ref":"e1"}]), json!([{"$ref":"e1"},{"href":"external.uml#End"}])] {
        // Force the known local comparison to be false in either position.
        author(&mut model, "r1", "type", json!({"$ref":"owner"}));
        author(&mut model, "connector", "end", ends);
        assert!(has(&diagnostics(&model)?, ERROR));
    }
    for ends in [json!([{"href":"external.uml#End"},false]),json!([false,{"href":"external.uml#End"}]),json!([{"$ref":"e1"},{"$ref":"e1"}])] {
        author(&mut model, "connector", "end", ends);
        assert!(has(&diagnostics(&model)?, ERROR));
    }
    Ok(())
}

#[test]
fn connector_type_flow_rejects_bad_role_type_ownership_and_association_reference() -> Result {
    for (id, property, value) in [("connector","type",json!({"$ref":"left"})),("e1","role",json!({"$ref":"owner"})),("e1","role",Value::Null)] {
        let mut model = document()?;
        author(&mut model, id, property, value);
        assert!(has(&diagnostics(&model)?, ERROR));
    }
    for (unknown_id, invalid_id) in [("r1", "a1"), ("a1", "r1")] {
        let mut model = document()?;
        author(&mut model, unknown_id, "type", json!({"href":"external.uml#Type"}));
        author(&mut model, invalid_id, "type", json!(false));
        assert!(has(&diagnostics(&model)?, ERROR), "invalid comparison side cannot be hidden by external counterpart");
    }
    let mut model = document()?;
    model.relations.iter_mut().find(|r| r.id == "owns-e1").ok_or("ownership")?.from = "owner".into();
    assert!(has(&diagnostics(&model)?, ERROR), "explicit end must agree with composite ownership");
    Ok(())
}

#[test]
fn connector_type_flow_checks_both_collections_before_returning_unknown() -> Result {
    for (missing_id, missing_role, invalid_id, invalid_role, valid_refs) in [
        ("connector", "end", "association", "memberEnd", ["a1", "a2"]),
        ("association", "memberEnd", "connector", "end", ["e1", "e2"]),
    ] {
        for invalid in [
            json!(false),
            Value::Null,
            json!([{"$ref": valid_refs[0]}, false]),
            json!([{"$ref": valid_refs[0]}, {"$ref": valid_refs[0]}]),
            json!([{"href": "external.uml#End"}, false]),
            json!([false, {"href": "external.uml#End"}]),
        ] {
            let mut model = document()?;
            model.elements.iter_mut().find(|e| e.id == missing_id).ok_or("missing side")?
                .properties.remove(missing_role);
            author(&mut model, invalid_id, invalid_role, invalid);
            let issues = diagnostics(&model)?;
            assert!(has(&issues, ERROR), "{missing_role} absent must not hide invalid {invalid_role}: {issues:?}");
            assert!(!has(&issues, UNKNOWN), "Invalid must dominate Unsupported for this rule");
        }
        let mut model = document()?;
        model.elements.iter_mut().find(|e| e.id == missing_id).ok_or("missing side")?
            .properties.remove(missing_role);
        assert!(has(&diagnostics(&model)?, UNKNOWN), "missing order plus valid opposite remains unknown");
        assert!(!has(&diagnostics(&model)?, ERROR));
    }
    Ok(())
}
