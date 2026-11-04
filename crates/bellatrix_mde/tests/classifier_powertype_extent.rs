use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "Classifier-maps_to_generalization_set:";
const ERROR: &str = "UML_CLASSIFIER_POWERTYPE_EXTENT";
const UNKNOWN: &str = "MDE_CLASSIFIER_POWERTYPE_EXTENT_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"project","name":"Generalization sets","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"general","kind":"uml.class","name":"General"},
        {"id":"other","kind":"uml.class","name":"Other"},
        {"id":"first","kind":"uml.class","name":"First"},
        {"id":"second","kind":"uml.class","name":"Second"},
        {"id":"set","kind":"uml.generalization_set","name":"Variants","properties":{"powertype":{"$ref":"other"},"generalization":[{"$ref":"first-general"},{"$ref":"second-general"}]}}
    ],"relations":[
        {"id":"first-general","kind":"uml.generalization","from":"first","to":"general"},
        {"id":"second-general","kind":"uml.generalization","from":"second","to":"general"}
    ]});
    for child in ["general", "other", "first", "second", "set"] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":"root","to":child}));
    }
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
}

fn target(project: &mut Project, id: &str, target: &str) -> Result {
    project
        .relations
        .iter_mut()
        .find(|relation| relation.id == id)
        .ok_or("relation")?
        .to = target.into();
    Ok(())
}
fn issues(project: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "other"
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with(RULE))
        })
        .cloned()
        .collect())
}
fn expect(project: &Project, code: &str) -> Result {
    let issues = issues(project)?;
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0]["code"], code, "{issues:#?}");
    assert_eq!(
        issues[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
fn edge(project: &mut Project, id: &str, from: &str, to: &str) -> Result {
    project.relations.push(serde_json::from_value(
        json!({"id":id,"kind":"uml.generalization","from":from,"to":to}),
    )?);
    Ok(())
}
fn extent(project: &mut Project, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "other")
        .ok_or("other")?
        .properties
        .insert("powertypeExtent".into(), value);
    Ok(())
}
fn omit_power(project: &mut Project) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "set")
        .ok_or("set")?
        .properties
        .remove("powertype");
    Ok(())
}
#[test]
fn canonical_opposite_and_authored_extent_both_establish_membership() -> Result {
    for rooted in [false, true] {
        let mut p = document(rooted)?;
        assert!(issues(&p)?.is_empty());
        extent(&mut p, json!([{"$ref":"set"}]))?;
        omit_power(&mut p)?;
        assert!(issues(&p)?.is_empty());
        target(&mut p, "first-general", "other")?;
        expect(&p, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&p, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}

#[test]
fn reverse_ancestry_and_snapshot_repair_do_not_invent_extra_constraints() -> Result {
    let mut p = document(false)?;
    extent(&mut p, json!([{"$ref":"set"}]))?;
    omit_power(&mut p)?;
    edge(&mut p, "power-general", "other", "general")?;
    assert!(issues(&p)?.is_empty());
    p.relations
        .retain(|relation| relation.id != "power-general");
    edge(&mut p, "specific-power", "first", "other")?;
    assert!(issues(&p)?.is_empty());
    target(&mut p, "first-general", "other")?;
    expect(&p, ERROR)?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&p, format)?, format)?;
        expect(&reopened, ERROR)?;
        target(&mut reopened, "first-general", "general")?;
        assert!(issues(&reopened)?.is_empty());
    }
    Ok(())
}

#[test]
fn unrelated_draft_edit_preserves_diagnostic_and_repair_remains_editable() -> Result {
    use bellatrix_mde::commands::{Command, apply};
    let mut p = document(true)?;
    extent(&mut p, json!([{"$ref":"set"}]))?;
    omit_power(&mut p)?;
    target(&mut p, "first-general", "other")?;
    let before = encode(&p, Format::Json)?;
    let command = Command::Rename {
        element_id: "other".into(),
        name: "Renamed".into(),
    };
    // Existing invalid drafts permit unrelated edits without losing evidence.
    let edited = apply(&p, &command, 1)?;
    expect(&edited, ERROR)?;
    assert_eq!(before, encode(&p, Format::Json)?);
    target(&mut p, "first-general", "general")?;
    let repaired = apply(&p, &command, 2)?;
    assert!(issues(&repaired)?.is_empty());
    Ok(())
}
