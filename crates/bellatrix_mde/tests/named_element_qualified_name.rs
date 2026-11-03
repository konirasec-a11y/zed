use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const HAS: &str = "UML_NAMED_ELEMENT_HAS_QUALIFIED_NAME";
const NONE: &str = "UML_NAMED_ELEMENT_HAS_NO_QUALIFIED_NAME";
const INVALID: &str = "MDE_NAMED_ELEMENT_QUALIFIED_NAME_INVALID";
const UNKNOWN: &str = "MDE_NAMED_ELEMENT_QUALIFIED_NAME_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Qualified names","root_model":"root","elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"package","kind":"uml.package","name":"Pkg"},
        {"id":"leaf","kind":"uml.class","name":"Leaf","properties":{"qualifiedName":"Root::Pkg::Leaf"}}
    ],"relations":[
        {"id":"owns-package","kind":"owns","from":"root","to":"package"},
        {"id":"owns-leaf","kind":"owns","from":"package","to":"leaf"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
}
fn author(p: &mut Project, id: &str, key: &str, value: Value) -> Result {
    p.elements
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn name(p: &mut Project, id: &str, value: &str) -> Result {
    p.elements
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("element")?
        .name = value.into();
    Ok(())
}
fn issues(p: &Project, id: &str) -> Result<Vec<Value>> {
    let result: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":p}).to_string(),
    )?)?;
    Ok(result["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == id
                && [HAS, NONE, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| d["code"] == *code)
        })
        .cloned()
        .collect())
}
fn expect(p: &Project, id: &str, code: &str) -> Result {
    let found = issues(p, id)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    Ok(())
}
fn expect_invalid_graph(p: &Project, id: &str) -> Result {
    let diagnostics = bellatrix_mde::uml::root::Model::from_project(p).validate();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.subject_id == id && diagnostic.code == INVALID),
        "{diagnostics:#?}"
    );
    // The public boundary rejects graph corruption before running UML rules.
    let result: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":p}).to_string(),
    )?)?;
    assert_eq!(result["diagnostics"][0]["code"], "GRAPH_INTEGRITY");
    Ok(())
}
#[test]
fn named_namespace_chain_uses_outermost_first_and_normative_separator() -> Result {
    for rooted in [true, false] {
        let mut p = document(rooted)?;
        assert!(issues(&p, "leaf")?.is_empty());
        author(&mut p, "leaf", "qualifiedName", json!("Leaf::Pkg::Root"))?;
        expect(&p, "leaf", HAS)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&p, format)?, format)?;
            expect(&reopened, "leaf", HAS)?;
        }
        author(&mut p, "leaf", "qualifiedName", Value::Null)?;
        expect(&p, "leaf", HAS)?;
    }
    Ok(())
}
#[test]
fn missing_subject_or_ancestor_name_requires_null_not_empty_string() -> Result {
    for id in ["leaf", "package", "root"] {
        let mut p = document(true)?;
        name(&mut p, id, "")?;
        expect(&p, "leaf", NONE)?;
        author(&mut p, "leaf", "qualifiedName", Value::Null)?;
        assert!(issues(&p, "leaf")?.is_empty());
        author(&mut p, "leaf", "qualifiedName", json!(""))?;
        expect(&p, "leaf", NONE)?;
    }
    Ok(())
}
#[test]
fn absent_projection_is_not_a_missing_authored_value() -> Result {
    let mut p = document(true)?;
    p.elements
        .iter_mut()
        .find(|e| e.id == "leaf")
        .ok_or("leaf")?
        .properties
        .remove("qualifiedName");
    assert!(issues(&p, "leaf")?.is_empty());
    Ok(())
}
#[test]
fn external_namespace_is_unknown_but_known_absent_name_is_decisive() -> Result {
    let mut p = document(true)?;
    author(
        &mut p,
        "package",
        "namespace",
        json!({"href":"external.uml#P"}),
    )?;
    expect(&p, "leaf", UNKNOWN)?;
    name(&mut p, "package", "")?;
    expect(&p, "leaf", NONE)?;
    author(&mut p, "leaf", "qualifiedName", Value::Null)?;
    assert!(issues(&p, "leaf")?.is_empty());
    Ok(())
}
#[test]
fn malformed_projection_and_cycles_are_invalid_even_if_name_is_absent() -> Result {
    for value in [json!(42), json!([]), json!({"$ref":"leaf"})] {
        let mut p = document(true)?;
        author(&mut p, "leaf", "qualifiedName", value)?;
        expect(&p, "leaf", INVALID)?;
    }
    let mut p = document(false)?;
    name(&mut p, "leaf", "")?;
    p.elements
        .iter_mut()
        .find(|element| element.id == "leaf")
        .ok_or("leaf")?
        .kind = "uml.package".into();
    p.relations.push(serde_json::from_value(
        json!({"id":"owns-root","kind":"owns","from":"leaf","to":"root"}),
    )?);
    expect_invalid_graph(&p, "leaf")?;
    Ok(())
}
#[test]
fn nonnamespace_owner_does_not_become_a_name_prefix() -> Result {
    let mut p = document(true)?;
    p.elements.push(serde_json::from_value(json!({"id":"value","kind":"uml.literal_integer","name":"Value","properties":{"value":1,"qualifiedName":"Value","visibility":null}}))?);
    p.elements.push(serde_json::from_value(json!({"id":"property","kind":"uml.property","name":"Property","properties":{"defaultValue":{"$ref":"value"}}}))?);
    p.relations.push(serde_json::from_value(
        json!({"id":"owns-property","kind":"owns","from":"leaf","to":"property"}),
    )?);
    p.relations.push(serde_json::from_value(
        json!({"id":"owns-value","kind":"owns","from":"property","to":"value"}),
    )?);
    assert!(issues(&p, "value")?.is_empty());
    author(&mut p, "value", "qualifiedName", json!("Property::Value"))?;
    expect(&p, "value", HAS)?;
    Ok(())
}
#[test]
fn relationship_name_type_is_checked_and_null_is_legitimate() -> Result {
    let mut p = document(true)?;
    p.relations.push(serde_json::from_value(json!({"id":"association","kind":"uml.association","from":"leaf","to":"leaf","properties":{"name":42,"qualifiedName":null}}))?);
    expect(&p, "association", INVALID)?;
    p.relations
        .iter_mut()
        .find(|r| r.id == "association")
        .ok_or("association")?
        .properties
        .insert("name".into(), Value::Null);
    assert!(issues(&p, "association")?.is_empty());
    Ok(())
}
#[test]
fn rename_cannot_leave_stale_projection_and_invalid_snapshot_can_be_repaired() -> Result {
    let p = document(true)?;
    let before = encode(&p, Format::Json)?;
    let error = apply(
        &p,
        &Command::Rename {
            element_id: "leaf".into(),
            name: "Changed".into(),
        },
        1,
    )
    .expect_err("stored qualifiedName becomes stale");
    assert!(error.contains("NamedElement-has_qualified_name"), "{error}");
    assert_eq!(before, encode(&p, Format::Json)?);
    let mut invalid = p;
    name(&mut invalid, "leaf", "Changed")?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, "leaf", HAS)?;
        let repaired = apply(
            &reopened,
            &Command::Rename {
                element_id: "leaf".into(),
                name: "Leaf".into(),
            },
            2,
        )?;
        assert!(issues(&repaired, "leaf")?.is_empty());
    }
    Ok(())
}

#[test]
fn root_without_namespace_uses_its_own_name() -> Result {
    let mut p = document(true)?;
    author(&mut p, "root", "qualifiedName", json!("Root"))?;
    assert!(issues(&p, "root")?.is_empty());
    author(&mut p, "root", "qualifiedName", json!("::Root"))?;
    expect(&p, "root", HAS)?;
    Ok(())
}

#[test]
fn duplicate_subject_identity_cannot_validate_a_projection() -> Result {
    let mut p = document(false)?;
    p.relations.clear();
    author(&mut p, "leaf", "qualifiedName", json!("Leaf"))?;
    p.elements.push(serde_json::from_value(
        json!({"id":"leaf","kind":"uml.class","name":"Other"}),
    )?);
    expect_invalid_graph(&p, "leaf")?;
    Ok(())
}

#[test]
fn authored_namespace_is_not_treated_as_canonical_ownership() -> Result {
    let mut p = document(true)?;
    author(&mut p, "leaf", "namespace", json!({"$ref":"root"}))?;
    expect(&p, "leaf", UNKNOWN)?;
    author(&mut p, "leaf", "qualifiedName", json!("Root::Leaf"))?;
    expect(&p, "leaf", UNKNOWN)?;
    Ok(())
}

#[test]
fn exposed_template_parameter_uses_the_templates_namespace() -> Result {
    let project: Project = serde_json::from_value(json!({
        "id":"templates", "name":"Templates",
        "elements":[{"id":"owner", "name":"Owner", "kind":"uml.class"}],
        "diagrams":[{"id":"diagram", "name":"D", "uml_di_type":"UMLClassDiagram",
            "views":[{"id":"view", "subject":"owner", "geometry":{"kind":"rect",
                "x":0.0,"y":0.0,"width":180.0,"height":120.0}}]}]
    }))?;
    let mut project = apply(
        &project,
        &Command::QuickEdit {
            diagram_id: "diagram".into(),
            element_id: "owner".into(),
            action: bellatrix_mde::UmlQuickCreate::TemplateParameter,
        },
        1,
    )?;
    let parametered = project
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.parametered_element")
        .ok_or("parameteredElement")?
        .to
        .clone();
    let parameter_name = project
        .elements
        .iter()
        .find(|element| element.id == parametered)
        .ok_or("parametered classifier")?
        .name
        .clone();
    author(
        &mut project,
        &parametered,
        "qualifiedName",
        json!(format!("Owner::{parameter_name}")),
    )?;
    assert!(issues(&project, &parametered)?.is_empty());
    author(
        &mut project,
        &parametered,
        "qualifiedName",
        json!(parameter_name),
    )?;
    expect(&project, &parametered, HAS)?;
    Ok(())
}

#[test]
fn external_namespace_cannot_mask_canonical_ownership_corruption() -> Result {
    for property in ["namespace", "owner"] {
        for multiple_owners in [false, true] {
            let mut p = document(false)?;
            name(&mut p, "leaf", "")?;
            author(&mut p, "leaf", "qualifiedName", Value::Null)?;
            author(&mut p, "leaf", property, json!({"href":"external.uml#P"}))?;
            p.relations.push(serde_json::from_value(if multiple_owners {
                json!({"id":"second-owner","kind":"owns","from":"root","to":"leaf"})
            } else {
                json!({"id":"cycle","kind":"owns","from":"leaf","to":"root"})
            })?);
            expect_invalid_graph(&p, "leaf")?;
        }
    }
    Ok(())
}
