use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "GeneralizationSet-maps_to_generalization_set:";
const ERROR: &str = "UML_GENERALIZATION_SET_POWERTYPE";
const INVALID: &str = "MDE_GENERALIZATION_SET_POWERTYPE_INVALID";
const UNKNOWN: &str = "MDE_GENERALIZATION_SET_POWERTYPE_UNVERIFIED";
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
fn author(project: &mut Project, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "set")
        .ok_or("set")?
        .properties
        .insert("generalization".into(), value);
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
            issue["subject_id"] == "set"
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
fn power(project: &mut Project, value: Value) -> Result {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "set")
        .ok_or("set")?
        .properties
        .insert("powertype".into(), value);
    Ok(())
}
fn edge(project: &mut Project, id: &str, from: &str, to: &str) -> Result {
    project.relations.push(serde_json::from_value(
        json!({"id":id,"kind":"uml.generalization","from":from,"to":to}),
    )?);
    Ok(())
}
fn middle(project: &mut Project) -> Result {
    project.elements.push(serde_json::from_value(
        json!({"id":"middle","kind":"uml.class","name":"Middle"}),
    )?);
    project.relations.push(serde_json::from_value(
        json!({"id":"owns-middle","kind":"owns","from":"root","to":"middle"}),
    )?);
    Ok(())
}
#[test]
fn independent_powertype_and_empty_sets_are_valid_in_rooted_and_draft_snapshots() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        for format in [Format::Json, Format::Protobuf] {
            assert!(issues(&decode(&encode(&project, format)?, format)?)?.is_empty());
        }
        author(&mut project, json!([]))?;
        assert!(issues(&project)?.is_empty());
    }
    Ok(())
}

#[test]
fn powertype_must_differ_from_general_and_specific_identities() -> Result {
    for id in ["general", "first", "second"] {
        let mut project = document(true)?;
        power(&mut project, json!({"$ref":id}))?;
        expect(&project, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn forbidden_ancestry_is_checked_transitively_in_both_normative_directions() -> Result {
    for direction in ["general-to-power", "power-to-specific"] {
        let mut project = document(true)?;
        middle(&mut project)?;
        if direction == "general-to-power" {
            edge(&mut project, "path-first", "general", "middle")?;
            edge(&mut project, "path-second", "middle", "other")?;
        } else {
            edge(&mut project, "path-first", "other", "middle")?;
            edge(&mut project, "path-second", "middle", "first")?;
        }
        expect(&project, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}

#[test]
fn invalid_powertype_snapshot_can_be_repaired_without_losing_other_properties() -> Result {
    let mut project = document(true)?;
    power(&mut project, json!({"$ref":"general"}))?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&project, format)?, format)?;
        expect(&reopened, ERROR)?;
        power(&mut reopened, json!({"$ref":"other"}))?;
        assert!(issues(&reopened)?.is_empty());
        assert_eq!(reopened.relations, project.relations);
    }
    Ok(())
}

#[test]
fn external_powertype_does_not_hide_invalid_known_general_ancestry() -> Result {
    for cycle in [true, false] {
        let mut project = document(true)?;
        power(&mut project, json!({"href":"external.uml#Power"}))?;
        if cycle {
            edge(&mut project, "cycle", "general", "general")?;
        } else {
            project
                .elements
                .iter_mut()
                .find(|element| element.id == "general")
                .ok_or("general")?
                .properties
                .insert("general".into(), json!(42));
        }
        expect(&project, INVALID)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&project, format)?, format)?, INVALID)?;
        }
    }
    Ok(())
}
