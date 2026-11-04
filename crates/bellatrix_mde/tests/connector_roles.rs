use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode,
    uml::{classifiers, references::ReferenceIndex, root::Model},
    validate_uml_model,
};
use serde_json::{Value, json};
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
    for (owner, child) in [
        ("root", "owner"),
        ("root", "parent"),
        ("root", "foreign"),
        ("root", "type"),
        ("parent", "r1"),
        ("owner", "r2"),
        ("foreign", "outside"),
        ("type", "port"),
        ("owner", "connector"),
        ("connector", "e1"),
        ("connector", "e2"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    Ok(serde_json::from_value(value)?)
}
fn author(model: &mut Project, id: &str, property: &str, value: Value) {
    model
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .expect("element")
        .properties
        .insert(property.into(), value);
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    let value: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(value["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .clone())
}
fn has(model: &Project, code: &str) -> Result<bool> {
    Ok(issues(model)?
        .iter()
        .any(|d| d["subject_id"] == "connector" && d["code"] == code))
}

#[test]
fn connector_roles_flow_uses_inherited_and_direct_roles_and_rejects_foreign_role_atomically()
-> Result {
    let model = document()?;
    assert!(
        !has(&model, ERROR)? && !has(&model, UNKNOWN)?,
        "{:?}",
        issues(&model)?
    );
    let index = ReferenceIndex::new(Model::from_project(&model));
    let roles = classifiers::all_roles(&index, "owner")?;
    assert_eq!(roles.len(), 2);
    assert!(roles.iter().any(|r| r.id() == "r1"));
    assert!(roles.iter().any(|r| r.id() == "r2"));
    let before = model.clone();
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "r1".into(),
            path: "properties.visibility".into(),
            value: json!("private"),
        },
        1,
    )
    .expect_err("non-inheritable role");
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
fn connector_roles_external_inverse_preserves_known_membership_and_atomic_rejection() -> Result {
    for end in ["e1", "e2"] {
        let mut model = document()?;
        author(
            &mut model,
            "connector",
            "structuredClassifier",
            json!({"href":"external.uml#Owner"}),
        );
        assert!(has(&model, UNKNOWN)? && !has(&model, ERROR)?);
        let before = model.clone();
        let error = apply(
            &model,
            &Command::SetOwner {
                element_id: if end == "e1" { "r1" } else { "r2" }.into(),
                owner_id: Some("foreign".into()),
            },
            1,
        )
        .expect_err("external inverse must not hide known non-membership");
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
