use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PROPERTY_SUBSETTING_CONTEXT_CONFORMS";
const INVALID: &str = "MDE_PROPERTY_SUBSETTING_CONTEXT_CONFORMS_INVALID";
const UNKNOWN: &str = "MDE_PROPERTY_SUBSETTING_CONTEXT_CONFORMS_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut model = json!({"id":"project","name":"Subsetting rules","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"owner","kind":"uml.class","name":"Owner"},
        {"id":"super","kind":"uml.class","name":"Super"},
        {"id":"middle","kind":"uml.class","name":"Middle"},
        {"id":"sub","kind":"uml.class","name":"Sub"},
        {"id":"general","kind":"uml.attribute","name":"General","properties":{"type":{"$ref":"super"}}},
        {"id":"specific","kind":"uml.attribute","name":"Specific","properties":{"type":{"$ref":"super"},"subsettedProperty":[{"$ref":"general"}]}}
    ],"relations":[
        {"id":"sub-middle","kind":"uml.generalization","from":"sub","to":"middle"},
        {"id":"middle-super","kind":"uml.generalization","from":"middle","to":"super"}
    ]});
    for (owner, child) in [
        ("root", "owner"),
        ("root", "super"),
        ("root", "middle"),
        ("root", "sub"),
        ("owner", "general"),
        ("owner", "specific"),
    ] {
        model["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    if !rooted {
        model.as_object_mut().ok_or("model")?.remove("root_model");
    }
    Ok(serde_json::from_value(model)?)
}

fn issues(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "specific"
                && [ERROR, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| issue["code"] == *code)
        })
        .cloned()
        .collect())
}
fn expect(model: &Project, code: &str) -> Result {
    let found = issues(model)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    assert_eq!(
        found[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
fn reparent(model: &mut Project, child: &str, owner: Option<&str>) -> Result {
    if let Some(owner) = owner {
        model
            .relations
            .iter_mut()
            .find(|relation| relation.id == format!("owns-{child}"))
            .ok_or("owner")?
            .from = owner.into();
    } else {
        model
            .relations
            .retain(|relation| relation.id != format!("owns-{child}"));
    }
    Ok(())
}

#[test]
fn direct_and_transitive_owner_conformance_are_not_strict_specialization() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        assert!(issues(&model)?.is_empty());
        reparent(&mut model, "general", Some("super"))?;
        for owner in ["super", "middle", "sub"] {
            reparent(&mut model, "specific", Some(owner))?;
            assert!(issues(&model)?.is_empty());
        }
        reparent(&mut model, "general", Some("sub"))?;
        reparent(&mut model, "specific", Some("super"))?;
        expect(&model, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
