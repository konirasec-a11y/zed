use bellatrix_mde::{
    commands::{apply, Command}, decode, encode, validate_uml_model, Format, Project,
    uml::{classifiers::{self, QueryError}, references::ReferenceIndex, root::Model},
};
use serde_json::{json, Value};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_CONNECTOR_ROLES";
const UNKNOWN: &str = "MDE_CONNECTOR_ROLES_UNVERIFIED";

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
fn has(model: &Project, code: &str) -> Result<bool> {
    Ok(issues(model)?.iter().any(|d| d["subject_id"] == "connector" && d["code"] == code))
}

#[test]
fn connector_roles_flow_uses_inherited_and_direct_roles_and_rejects_foreign_role_atomically() -> Result {
    let model = document()?;
    assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?, "{:?}", issues(&model)?);
    let index = ReferenceIndex::new(Model::from_project(&model));
    let roles = classifiers::all_roles(&index, "owner")?;
    assert_eq!(roles.len(), 2);
    assert!(roles.iter().any(|r| r.id() == "r1"));
    assert!(roles.iter().any(|r| r.id() == "r2"));
    let before = model.clone();
    let error = apply(&model, &Command::SetProperty { element_id:"r1".into(), path:"properties.visibility".into(), value:json!("private") }, 1).expect_err("non-inheritable role");
    assert!(error.contains("Connector-roles"), "{error}");
    assert_eq!(before, model);
    let mut invalid = model;
    author(&mut invalid, "e1", "role", json!({"$ref":"outside"}));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        assert!(has(&reopened, ERROR)?);
    }
    Ok(())
}

#[test]
fn connector_roles_flow_obeys_visibility_redefinition_and_diamond_set_semantics() -> Result {
    let mut model = document()?;
    author(&mut model, "r1", "visibility", json!("private"));
    assert!(has(&model, ERROR)?);
    author(&mut model, "r1", "visibility", json!("protected"));
    assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    author(&mut model, "r2", "redefinedProperty", json!([{"$ref":"r1"}]));
    assert!(has(&model, ERROR)?, "redefined inherited role must be excluded");
    author(&mut model, "e1", "role", json!({"$ref":"r2"}));
    assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    model.elements.iter_mut().find(|e| e.id == "r2").unwrap().properties.remove("redefinedProperty");
    model.elements.push(serde_json::from_value(json!({"id":"branch","kind":"uml.class","name":"Branch"}))?);
    for (id, kind, from, to) in [("owns-branch","owns","root","branch"),("branch-parent","uml.generalization","branch","parent"),("owner-branch","uml.generalization","owner","branch")] {
        model.relations.push(serde_json::from_value(json!({"id":id,"kind":kind,"from":from,"to":to}))?);
    }
    let index = ReferenceIndex::new(Model::from_project(&model));
    assert_eq!(classifiers::all_roles(&index, "owner")?.iter().filter(|r| r.id() == "r1").count(), 1);
    Ok(())
}

#[test]
fn connector_roles_flow_accepts_port_on_member_part_but_not_non_port_or_foreign_part() -> Result {
    let mut model = document()?;
    author(&mut model, "e1", "role", json!({"$ref":"port"}));
    author(&mut model, "e1", "partWithPort", json!({"$ref":"r2"}));
    assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    author(&mut model, "e1", "role", json!({"$ref":"outside"}));
    assert!(has(&model, ERROR)?, "partWithPort is only an alternative for a Port role");
    author(&mut model, "e1", "role", json!({"$ref":"port"}));
    author(&mut model, "e1", "partWithPort", json!({"$ref":"outside"}));
    assert!(has(&model, ERROR)?);
    author(&mut model, "e1", "partWithPort", Value::Null);
    assert!(has(&model, ERROR)?);
    Ok(())
}

#[test]
fn connector_roles_flow_proves_direct_membership_without_claiming_complete_partial_namespace() -> Result {
    for members in [json!([{"$ref":"r2"},{"href":"external.uml#Role"}]),json!([{"href":"external.uml#Role"},{"$ref":"r2"}])] {
        let mut model = document()?;
        author(&mut model, "owner", "ownedAttribute", members);
        author(&mut model, "e1", "role", json!({"$ref":"r2"}));
        let index = ReferenceIndex::new(Model::from_project(&model));
        assert!(matches!(classifiers::all_roles(&index, "owner"), Err(QueryError::Unsupported { .. })));
        assert_eq!(classifiers::has_role(&index, "owner", "r2")?, true);
        assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
        author(&mut model, "e1", "role", json!({"$ref":"r1"}));
        assert!(has(&model, UNKNOWN)?, "unresolved complete namespace does not invent inherited evidence");
        for malformed in [json!([{"href":"external.uml#Role"},false]),json!([false,{"href":"external.uml#Role"}]),json!([{"$ref":"r2"},{"$ref":"r2"}])] {
            author(&mut model, "owner", "ownedAttribute", malformed);
            author(&mut model, "e1", "role", json!({"$ref":"r2"}));
            assert!(has(&model, ERROR)?, "known local membership does not hide invalid ownership");
        }
    }
    Ok(())
}

#[test]
fn connector_roles_flow_retains_unknown_and_detects_invalid_in_other_end_or_input() -> Result {
    for (unknown, invalid) in [("e1","e2"),("e2","e1")] {
        let mut model = document()?;
        author(&mut model, unknown, "role", json!({"href":"external.uml#Role"}));
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
        author(&mut model, invalid, "role", json!({"$ref":"outside"}));
        assert!(has(&model, ERROR)?);
        author(&mut model, unknown, "partWithPort", json!(false));
        assert!(has(&model, ERROR)?);
    }
    let mut model = document()?;
    author(&mut model, "connector", "structuredClassifier", json!({"$ref":"foreign"}));
    assert!(has(&model, ERROR)?, "stored inverse cannot override semantic owner");
    model.elements.iter_mut().find(|e| e.id == "connector").unwrap().properties.remove("structuredClassifier");
    model.relations.iter_mut().find(|r| r.id == "owns-connector").unwrap().from = "root".into();
    assert!(has(&model, ERROR)?, "non-StructuredClassifier owner");
    Ok(())
}

#[test]
fn connector_roles_flow_preserves_direct_proof_beside_unknown_ancestry_but_rejects_cycles() -> Result {
    let mut model = document()?;
    author(&mut model, "owner", "general", json!([{"href":"external.uml#Base"}]));
    author(&mut model, "e1", "role", json!({"$ref":"r2"}));
    let index = ReferenceIndex::new(Model::from_project(&model));
    assert!(matches!(classifiers::all_roles(&index, "owner"), Err(QueryError::Unsupported { .. })));
    assert_eq!(classifiers::has_role(&index, "owner", "r2")?, true);
    assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    model.relations.push(serde_json::from_value(json!({"id":"cycle","kind":"uml.generalization","from":"parent","to":"owner"}))?);
    let index = ReferenceIndex::new(Model::from_project(&model));
    assert!(matches!(classifiers::has_role(&index, "owner", "r2"), Err(QueryError::Invalid { .. })));
    assert!(has(&model, ERROR)?);
    Ok(())
}

#[test]
fn connector_roles_unknown_owner_does_not_hide_invalid_stored_inverse() -> Result {
    for unknown_owner in [false, true] {
        let mut model = document()?;
        if unknown_owner {
            model.elements.iter_mut().find(|element| element.id == "owner")
                .ok_or("owner")?.kind = "uml.unknown_owner".into();
            assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
        }
        for stored in [json!(false), json!({"$ref":"foreign"}), json!({"$ref":"missing"}), json!({"$ref":"e1"})] {
            author(&mut model, "connector", "structuredClassifier", stored.clone());
            assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?, "stored {stored}: {:?}", issues(&model)?);
        }
        author(&mut model, "connector", "structuredClassifier", json!({"$ref":"owner"}));
        assert!(!has(&model, ERROR)?);
        assert_eq!(has(&model, UNKNOWN)?, unknown_owner);
        author(&mut model, "connector", "structuredClassifier", json!({"href":"external.uml#Owner"}));
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
    }
    Ok(())
}

#[test]
fn connector_roles_unknown_owner_still_checks_each_end_input_in_both_orders() -> Result {
    for unknown_owner in [false, true] {
        for (invalid_end, other_end) in [("e1", "e2"), ("e2", "e1")] {
            for property in ["role", "partWithPort"] {
                for invalid in [json!(false), json!({"$ref":"missing"}), json!({"$ref":"connector"})] {
                    let mut model = document()?;
                    if unknown_owner {
                        model.elements.iter_mut().find(|element| element.id == "owner")
                            .ok_or("owner")?.kind = "uml.unknown_owner".into();
                    } else {
                        author(&mut model, "connector", "structuredClassifier", json!({"href":"external.uml#Owner"}));
                    }
                    author(&mut model, other_end, "role", json!({"href":"external.uml#Role"}));
                    // Keep the opposite input unresolved as well as the opposite end.
                    author(&mut model, invalid_end, if property == "role" { "partWithPort" } else { "role" }, json!({"href":"external.uml#Input"}));
                    assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
                    author(&mut model, invalid_end, property, invalid.clone());
                    assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?, "{invalid_end}.{property} = {invalid}: {:?}", issues(&model)?);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn connector_roles_external_inverse_preserves_known_membership_and_atomic_rejection() -> Result {
    for end in ["e1", "e2"] {
        let mut model = document()?;
        author(&mut model, "connector", "structuredClassifier", json!({"href":"external.uml#Owner"}));
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
        let before = model.clone();
        let error = apply(&model, &Command::SetOwner {
            element_id: if end == "e1" { "r1" } else { "r2" }.into(),
            owner_id: Some("foreign".into()),
        }, 1).expect_err("external inverse must not hide known non-membership");
        assert!(error.contains("Connector-roles"), "{error}");
        assert_eq!(model, before);
        author(&mut model, end, "role", json!({"$ref":"outside"}));
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
        author(&mut model, end, "role", json!({"$ref":"port"}));
        author(&mut model, end, "partWithPort", json!({"$ref":"outside"}));
        assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
        author(&mut model, end, "partWithPort", json!({"$ref":"r2"}));
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
    }
    Ok(())
}
