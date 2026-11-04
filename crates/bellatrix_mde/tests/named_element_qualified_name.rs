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
