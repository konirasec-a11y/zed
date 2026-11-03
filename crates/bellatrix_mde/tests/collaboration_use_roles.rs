use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_COLLABORATION_USE_EVERY_ROLE";
const UNKNOWN: &str = "MDE_COLLABORATION_USE_EVERY_ROLE_UNVERIFIED";
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
#[test]
fn given_all_roles_bound_then_explicit_and_inferred_binding_pass_roundtrip() -> Result {
    for explicit in [false, true] {
        let model = document(explicit)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}
#[test]
fn given_unbound_role_then_known_violation_survives_unknown_role_neighbors() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        let mut roles = vec![
            json!({"$ref":"role1"}),
            json!({"$ref":"role2"}),
            json!({"href":"external.uml#Role"}),
        ];
        if reverse {
            roles.reverse();
            model.relations.reverse();
        }
        author(
            &mut model,
            "collaboration",
            "collaborationRole",
            json!(roles),
        )?;
        assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn given_unknown_bindings_then_known_witness_passes_but_uncovered_role_remains_unknown() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        let mut bindings = vec![
            json!({"$ref":"binding"}),
            json!({"href":"external.uml#Binding"}),
        ];
        if reverse {
            bindings.reverse();
        }
        author(&mut model, "usage", "roleBinding", json!(bindings))?;
        assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
        author(
            &mut model,
            "collaboration",
            "collaborationRole",
            json!([{"$ref":"role1"},{"$ref":"role2"}]),
        )?;
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
    }
    Ok(())
}
#[test]
fn given_partial_supplier_list_then_known_membership_is_a_witness_without_binary_fallback() -> Result
{
    for reverse in [false, true] {
        let mut model = document(true)?;
        let mut suppliers = vec![
            json!({"$ref":"role1"}),
            json!({"href":"external.uml#Supplier"}),
        ];
        if reverse {
            suppliers.reverse();
        }
        author(&mut model, "binding", "supplier", json!(suppliers))?;
        assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
        author(&mut model, "binding", "supplier", json!([{"$ref":"role2"}]))?;
        assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn given_roles_or_type_unresolved_then_unknown_not_satisfied() -> Result {
    let mut model = document(true)?;
    author(
        &mut model,
        "collaboration",
        "collaborationRole",
        json!([{"href":"external.uml#Role"}]),
    )?;
    assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
    for value in [Some(json!({"href":"external.uml#Collaboration"})), None] {
        let mut model = document(true)?;
        if let Some(value) = value {
            author(&mut model, "usage", "type", value)?;
        } else {
            model
                .elements
                .iter_mut()
                .find(|element| element.id == "usage")
                .ok_or("usage")?
                .properties
                .remove("type");
        }
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
    }
    Ok(())
}
#[test]
fn given_malformed_known_data_then_invalid_dominates_unknown_type_or_bindings() -> Result {
    for (id, key, value) in [
        (
            "usage",
            "roleBinding",
            json!([{"href":"external.uml#Binding"},{"$ref":"missing"}]),
        ),
        (
            "usage",
            "roleBinding",
            json!([{"$ref":"binding"},{"$ref":"binding"}]),
        ),
        (
            "collaboration",
            "collaborationRole",
            json!([{"href":"external.uml#Role"},false]),
        ),
        (
            "collaboration",
            "collaborationRole",
            json!([{"$ref":"owner"}]),
        ),
        (
            "binding",
            "supplier",
            json!([{"href":"external.uml#Supplier"},false]),
        ),
        ("usage", "type", Value::Null),
        ("usage", "type", json!({"$ref":"owner"})),
    ] {
        let mut model = document(true)?;
        author(&mut model, id, key, value)?;
        assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?, "{id}.{key}");
    }
    let mut model = document(true)?;
    author(
        &mut model,
        "usage",
        "type",
        json!({"href":"external.uml#Collaboration"}),
    )?;
    author(&mut model, "binding", "supplier", json!([false]))?;
    assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    Ok(())
}
#[test]
fn given_binding_owned_elsewhere_or_unowned_then_it_is_not_inferred_from_dependency_endpoints()
-> Result {
    let mut model = document(false)?;
    model
        .relations
        .retain(|relation| relation.id != "owns-binding");
    assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    author(
        &mut model,
        "usage",
        "roleBinding",
        json!([{"$ref":"binding"}]),
    )?;
    assert!(has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    Ok(())
}
#[test]
fn given_no_collaboration_roles_then_owned_attributes_are_not_implicitly_selected() -> Result {
    for explicit in [false, true] {
        let mut model = document(false)?;
        model
            .relations
            .retain(|relation| relation.id != "binding" && relation.id != "owns-binding");
        if explicit {
            author(&mut model, "collaboration", "collaborationRole", json!([]))?;
        } else {
            model
                .elements
                .iter_mut()
                .find(|element| element.id == "collaboration")
                .ok_or("collaboration")?
                .properties
                .remove("collaborationRole");
        }
        assert!(!has(&model, ERROR)? && !has(&model, UNKNOWN)?);
    }
    Ok(())
}
#[test]
fn given_only_binding_deleted_then_unbound_role_rejection_is_atomic() -> Result {
    let model = document(false)?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::DeleteElement {
            element_id: "binding".into(),
        },
        1,
    )
    .expect_err("required role binding");
    assert!(error.to_string().contains(ERROR), "{error}");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}

#[test]
fn given_two_roles_then_one_or_two_bindings_can_cover_both() -> Result {
    for split in [false, true] {
        let mut model = document(false)?;
        author(
            &mut model,
            "collaboration",
            "collaborationRole",
            json!([{"$ref":"role1"},{"$ref":"role2"}]),
        )?;
        if split {
            model.relations.push(serde_json::from_value(json!({"id":"binding2","kind":"uml.dependency","from":"client","to":"role2","properties":{"visibility":null}}))?);
            model.relations.push(serde_json::from_value(
                json!({"id":"owns-binding2","kind":"owns","from":"usage","to":"binding2"}),
            )?);
        } else {
            author(
                &mut model,
                "binding",
                "supplier",
                json!([{"$ref":"role1"},{"$ref":"role2"}]),
            )?;
        }
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}

#[test]
fn given_dependency_object_then_explicit_suppliers_provide_canonical_witnesses() -> Result {
    for explicit in [false, true] {
        let mut model = document(explicit)?;
        model.relations.retain(|relation| relation.id != "binding");
        model.elements.push(serde_json::from_value(json!({"id":"binding","kind":"uml.dependency","name":"Binding","properties":{"visibility":null,"client":[{"$ref":"client"}],"supplier":[{"$ref":"role1"}]}}))?);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format)?, format)?;
            assert!(!has(&reopened, ERROR)? && !has(&reopened, UNKNOWN)?);
        }
    }
    Ok(())
}
