use bellatrix_mde::{commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project,
    uml::{connectors, classifiers::QueryError, references::ReferenceIndex, root::Model}};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_CONNECTOR_END_MULTIPLICITY";
const UNKNOWN: &str = "MDE_CONNECTOR_END_MULTIPLICITY_UNVERIFIED";
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
fn given_multiplicity_ranges_then_lower_upper_and_unlimited_bounds_obey_containment() -> Result {
    for (specific, general, valid) in [("1..2","0..3",true),("0..2","1..3",false),("1..4","0..3",false),("1..*","0..*",true),("1..*","0..3",false),("0..0","0..1",true)] {
        let mut model = document()?;
        author(&mut model, "e1", "multiplicity", json!(specific));
        author(&mut model, "a1", "multiplicity", json!(general));
        assert_eq!(has(&model, "e1", ERROR)?, !valid, "{specific} in {general}");
        assert!(!has(&model, "e1", UNKNOWN)?);
    }
    Ok(())
}

#[test]
fn given_untyped_connector_then_no_defining_end_or_comparison_is_invented() -> Result {
    let mut model = document()?;
    author(&mut model, "connector", "type", Value::Null);
    author(&mut model, "e1", "definingEnd", json!({"$ref":"a1"}));
    author(&mut model, "e1", "multiplicity", json!("0..*"));
    assert_eq!(target(&model, "e1")?, None);
    assert!(!has(&model, "e1", ERROR)? && !has(&model, "e1", UNKNOWN)?);
    author(&mut model, "connector", "type", json!({"href":"external.uml#Association"}));
    assert!(has(&model, "e1", UNKNOWN)? && !has(&model, "e1", ERROR)?);
    author(&mut model, "e1", "multiplicity", json!("3..2"));
    assert!(has(&model, "e1", ERROR)?);
    Ok(())
}

#[test]
fn given_absent_order_then_ownership_and_stored_defining_end_do_not_establish_projection() -> Result {
    for (id, property) in [("connector", "end"), ("association", "memberEnd")] {
        let mut model = document()?;
        model.elements.iter_mut().find(|element| element.id == id).ok_or("element")?.properties.remove(property);
        author(&mut model, "e1", "definingEnd", json!({"$ref":"a1"}));
        let references = ReferenceIndex::new(Model::from_project(&model));
        assert!(matches!(connectors::defining_end(&references, references.element("e1")?), Err(QueryError::Unsupported { .. })));
        assert!(has(&model, "e1", UNKNOWN)? && !has(&model, "e1", ERROR)?);
    }
    Ok(())
}

#[test]
fn given_external_sibling_then_known_position_violations_win_in_both_orders() -> Result {
    for (end, member, ends, members) in [
        ("e1", "a1", json!([{"$ref":"e1"},{"href":"external.uml#End"}]), json!([{"$ref":"a1"},{"href":"external.uml#Member"}])),
        ("e2", "a2", json!([{"href":"external.uml#End"},{"$ref":"e2"}]), json!([{"href":"external.uml#Member"},{"$ref":"a2"}])),
    ] {
        let mut model = document()?;
        author(&mut model, "connector", "end", ends);
        author(&mut model, "association", "memberEnd", members);
        assert!(has(&model, end, UNKNOWN)? && !has(&model, end, ERROR)?);
        author(&mut model, end, "multiplicity", json!("0..*"));
        author(&mut model, member, "multiplicity", json!("1"));
        assert!(has(&model, end, ERROR)? && !has(&model, end, UNKNOWN)?);
    }
    Ok(())
}

#[test]
fn given_malformed_order_or_target_then_unknown_opposite_side_cannot_hide_invalid() -> Result {
    for (unknown_id, unknown_property, invalid_id, invalid_property) in [
        ("connector", "end", "association", "memberEnd"),
        ("association", "memberEnd", "connector", "end"),
    ] {
        let mut model = document()?;
        model.elements.iter_mut().find(|element| element.id == unknown_id).ok_or("element")?.properties.remove(unknown_property);
        for invalid in [json!(false), json!([{"$ref":"missing"},{"href":"external.uml#End"}]), json!([{"href":"external.uml#End"},{"$ref":"missing"}]), json!([{"$ref":"e1"},{"$ref":"e1"}])] {
            author(&mut model, invalid_id, invalid_property, invalid);
            assert!(has(&model, "e1", ERROR)? && !has(&model, "e1", UNKNOWN)?);
        }
    }
    Ok(())
}

#[test]
fn given_unknown_bound_on_either_side_then_invalid_opposite_bound_wins() -> Result {
    for (unknown, invalid) in [("e1", "a1"), ("a1", "e1")] {
        let mut model = document()?;
        author(&mut model, unknown, "multiplicity", json!("limit..*"));
        author(&mut model, invalid, "multiplicity", json!("0..*"));
        assert!(has(&model, "e1", UNKNOWN)?);
        author(&mut model, invalid, "multiplicity", json!("3..2"));
        assert!(has(&model, "e1", ERROR)? && !has(&model, "e1", UNKNOWN)?);
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

#[test]
fn given_identity_bounds_then_known_comparisons_and_invalid_values_survive_opaque_other_bound() -> Result {
    for upper_is_opaque in [false, true] {
        let mut model = document()?;
        author(&mut model, "a1", "multiplicity", json!("2..3"));
        for (id, role, kind, value) in [
            ("lower", "lowerValue", if upper_is_opaque { "uml.literal_integer" } else { "uml.opaque_expression" }, json!(1)),
            ("upper", "upperValue", if upper_is_opaque { "uml.opaque_expression" } else { "uml.literal_unlimited_natural" }, json!(4)),
        ] {
            model.elements.push(serde_json::from_value(json!({"id":id,"kind":kind,"name":"","properties":{"value":value}}))?);
            model.relations.push(serde_json::from_value(json!({"id":format!("owns-{id}"),"kind":"owns","from":"e1","to":id}))?);
            author(&mut model, "e1", role, json!({"$ref":id}));
        }
        assert!(has(&model, "e1", ERROR)? && !has(&model, "e1", UNKNOWN)?, "known bound violates containment");
        author(&mut model, "a1", "multiplicity", json!("0..*"));
        if upper_is_opaque {
            assert!(!has(&model, "e1", ERROR)? && !has(&model, "e1", UNKNOWN)?, "unlimited general upper decides the disjunction");
            author(&mut model, "lower", "value", json!(-1));
            let references = ReferenceIndex::new(Model::from_project(&model));
            assert!(matches!(bellatrix_mde::uml::value_specifications::multiplicity(&references, references.element("e1")?),
                Err(bellatrix_mde::uml::value_specifications::EvaluationError::Invalid { .. })));
            assert!(has(&model, "e1", ERROR)?);
        } else {
            assert!(has(&model, "e1", UNKNOWN)?);
            author(&mut model, "upper", "value", json!(-1));
            assert!(has(&model, "e1", ERROR)?, "invalid upper is not hidden by opaque lower");
        }
    }
    Ok(())
}

#[test]
fn given_unresolved_composite_membership_then_malformed_type_and_member_order_still_fail() -> Result {
    for property in ["type", "memberEnd"] {
        let mut model = document()?;
        author(&mut model, "connector", "end", json!([{"href":"external.uml#End1"},{"href":"external.uml#End2"}]));
        author(&mut model, if property == "type" { "connector" } else { "association" }, property, json!(false));
        assert!(has(&model, "e1", ERROR)? && !has(&model, "e1", UNKNOWN)?);
        let references = ReferenceIndex::new(Model::from_project(&model));
        assert!(matches!(connectors::defining_end(&references, references.element("e1")?), Err(QueryError::Invalid { .. })));
    }
    Ok(())
}
