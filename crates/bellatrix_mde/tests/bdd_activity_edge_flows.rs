use bellatrix_mde::{Format, Project, decode_model_document, encode, validate_uml_model};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

type FlowResult<T> = Result<T, Box<dyn Error>>;

struct SavedFlow(PathBuf);
impl Drop for SavedFlow {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not clean Activity BDD directory {}: {error}",
                self.0.display()
            );
        }
    }
}

fn imported_activity() -> FlowResult<Value> {
    Ok(serde_json::from_str(
        &decode_model_document(
            include_bytes!("../fixtures/bdd/behavioral/activity.valid.json"),
            "json",
        )
        .map_err(std::io::Error::other)?,
    )?)
}

fn element<'a>(envelope: &'a mut Value, identifier: &str) -> FlowResult<&'a mut Value> {
    envelope["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .iter_mut()
        .find(|element| element["id"] == identifier)
        .ok_or_else(|| format!("missing {identifier}").into())
}

fn relation<'a>(envelope: &'a mut Value, identifier: &str) -> FlowResult<&'a mut Value> {
    envelope["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .iter_mut()
        .find(|relation| relation["id"] == identifier)
        .ok_or_else(|| format!("missing {identifier}").into())
}

fn validate(envelope: &Value) -> FlowResult<Value> {
    let response: Value = serde_json::from_str(
        &validate_uml_model(&envelope.to_string()).map_err(std::io::Error::other)?,
    )?;
    assert_eq!(response["api_version"], 1);
    assert_eq!(response["coverage"], "implemented_rules_only");
    let errors = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .any(|diagnostic| diagnostic["severity"] == "error");
    assert_eq!(response["has_errors"], errors);
    Ok(response)
}

fn rule_diagnostic(response: &Value, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic["code"].as_str(),
                Some("UML_CONTROL_FLOW_OBJECT_NODES" | "MDE_CONTROL_FLOW_OBJECT_NODES_UNVERIFIED")
            )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["subject_id"], "order-data");
        assert_eq!(matching[0]["severity"], severity);
        assert_eq!(
            matching[0]["code"],
            if severity == "error" {
                "UML_CONTROL_FLOW_OBJECT_NODES"
            } else {
                "MDE_CONTROL_FLOW_OBJECT_NODES_UNVERIFIED"
            }
        );
        assert!(
            matching[0]["message"]
                .as_str()
                .ok_or("missing message")?
                .starts_with("ControlFlow-object_nodes:")
        );
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

fn reopen(name: &str, envelope: &Value, expected: &Value) -> FlowResult<()> {
    let project: Project = serde_json::from_value(envelope["model"].clone())?;
    let directory = std::env::temp_dir().join(format!(
        "bellatrix-activity-edge-{}-{name}",
        std::process::id()
    ));
    fs::create_dir(&directory)?;
    let saved = SavedFlow(directory);
    for (format, name) in [(Format::Json, "json"), (Format::Protobuf, "protobuf")] {
        println!("And I save, close and reopen {name}, preserving authored data and findings");
        let path = saved.0.join(name);
        fs::write(&path, encode(&project, format)?)?;
        let reopened: Value = serde_json::from_str(
            &decode_model_document(&fs::read(path)?, name).map_err(std::io::Error::other)?,
        )?;
        assert_eq!(reopened["model"], serde_json::to_value(&project)?);
        assert_eq!(&validate(&reopened)?, expected);
    }
    Ok(())
}

#[test]
fn authored_control_flow_checks_both_pin_endpoints_and_repairs_without_materializing_defaults()
-> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Repair a ControlFlow between typed Pins through persistence")
    );
    println!("Given an imported Activity with a valid ObjectFlow between OutputPin and InputPin");
    let original = imported_activity()?;
    let baseline = validate(&original)?;
    assert_eq!(baseline["has_errors"], false);
    let mut authored = original.clone();
    relation(&mut authored, "order-data")?["kind"] = json!("uml.control_flow");
    println!("When I change that edge into a ControlFlow and leave isControlType omitted");
    let implicit_false = validate(&authored)?;
    rule_diagnostic(&implicit_false, Some("error"))?;
    assert_eq!(
        implicit_false["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .filter(|diagnostic| diagnostic["severity"] == "error")
            .count(),
        1,
        "{implicit_false}"
    );
    assert!(
        element(&mut authored, "order-output")?["properties"]
            .get("isControlType")
            .is_none()
    );
    assert!(
        element(&mut authored, "order-input")?["properties"]
            .get("isControlType")
            .is_none()
    );
    reopen("implicit-false", &authored, &implicit_false)?;

    println!("When only the source Pin is explicitly a control-type ObjectNode");
    element(&mut authored, "order-output")?["properties"]["isControlType"] = json!(true);
    let target_false = validate(&authored)?;
    rule_diagnostic(&target_false, Some("error"))?;
    reopen("target-false", &authored, &target_false)?;
    println!("Then the target independently violates the normative conjunction");
    element(&mut authored, "order-input")?["properties"]["isControlType"] = json!(true);
    let both_true = validate(&authored)?;
    rule_diagnostic(&both_true, None)?;
    assert_eq!(both_true["has_errors"], false, "{both_true}");
    reopen("both-true", &authored, &both_true)?;

    println!("When the source alone is changed back to false");
    element(&mut authored, "order-output")?["properties"]["isControlType"] = json!(false);
    let source_false = validate(&authored)?;
    rule_diagnostic(&source_false, Some("error"))?;
    reopen("source-false", &authored, &source_false)?;
    println!("And a malformed Boolean remains an authored error, never an implicit default");
    element(&mut authored, "order-output")?["properties"]["isControlType"] = json!("true");
    let malformed = validate(&authored)?;
    rule_diagnostic(&malformed, Some("warning"))?;
    assert!(
        malformed["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(|diagnostic| diagnostic["code"] == "UML_PROPERTY_VALUE"
                && diagnostic["subject_id"] == "order-output")
    );
    reopen("malformed-control-type", &authored, &malformed)?;

    println!(
        "When I repair the Boolean but contradict a stored endpoint with the canonical relation"
    );
    element(&mut authored, "order-output")?["properties"]["isControlType"] = json!(true);
    relation(&mut authored, "order-data")?["properties"]["source"] = json!({"$ref":"receive"});
    let contradictory = validate(&authored)?;
    rule_diagnostic(&contradictory, Some("warning"))?;
    assert_eq!(
        contradictory["has_errors"], false,
        "An endpoint mismatch is not a proved ObjectNode predicate violation: {contradictory}"
    );
    reopen("contradictory-endpoint", &authored, &contradictory)?;
    element(&mut authored, "order-input")?["properties"]["isControlType"] = json!(false);
    let contradictory_with_violation = validate(&authored)?;
    rule_diagnostic(&contradictory_with_violation, Some("error"))?;
    reopen(
        "contradiction-known-violation",
        &authored,
        &contradictory_with_violation,
    )?;
    element(&mut authored, "order-input")?["properties"]["isControlType"] = json!(true);
    assert_eq!(validate(&authored)?, contradictory);
    relation(&mut authored, "order-data")?["properties"]["source"] = json!({"$ref":"order-output"});
    assert_eq!(validate(&authored)?, both_true);
    println!("Then repairs and the original ObjectFlow remain deterministic after invalid calls");
    assert_eq!(validate(&original)?, baseline);
    Ok(())
}

#[test]
fn object_record_control_flow_keeps_external_and_unknown_endpoints_unverified() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Reopen object-record ControlFlows with unresolved endpoints")
    );
    println!("Given the same authored Activity with its edge represented as a ControlFlow object");
    let mut authored = imported_activity()?;
    let properties = relation(&mut authored, "order-data")?["properties"].clone();
    authored["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .retain(|relation| relation["id"] != "order-data");
    authored["model"]["elements"].as_array_mut().ok_or("missing elements")?.push(
        json!({"id":"order-data", "kind":"uml.control_flow", "name":"Order control", "properties":properties}));
    let default_false = validate(&authored)?;
    rule_diagnostic(&default_false, Some("error"))?;
    reopen("object-default-false", &authored, &default_false)?;
    element(&mut authored, "order-output")?["properties"]["isControlType"] = json!(true);
    element(&mut authored, "order-input")?["properties"]["isControlType"] = json!(true);
    let valid = validate(&authored)?;
    rule_diagnostic(&valid, None)?;
    assert_eq!(valid["has_errors"], false, "{valid}");
    reopen("object-control-true", &authored, &valid)?;

    println!(
        "When the source becomes an unresolved external href and the known target remains allowed"
    );
    element(&mut authored, "order-data")?["properties"]["source"] =
        json!({"href":"other.uml#source"});
    let external = validate(&authored)?;
    rule_diagnostic(&external, Some("warning"))?;
    assert_eq!(external["has_errors"], false, "{external}");
    reopen("object-external-source", &authored, &external)?;
    println!(
        "When the known target violates the rule, an unknown source cannot hide that violation"
    );
    element(&mut authored, "order-input")?["properties"]["isControlType"] = json!(false);
    let proved = validate(&authored)?;
    rule_diagnostic(&proved, Some("error"))?;
    reopen("object-known-violation", &authored, &proved)?;
    element(&mut authored, "order-input")?["properties"]["isControlType"] = json!(true);
    assert_eq!(validate(&authored)?, external);

    println!(
        "When a local endpoint has an unsupported UML metaclass instead of a guessed Pin type"
    );
    element(&mut authored, "order-data")?["properties"]["source"] = json!({"$ref":"order-output"});
    element(&mut authored, "order-output")?["kind"] = json!("uml.future_pin");
    let unknown = validate(&authored)?;
    rule_diagnostic(&unknown, Some("warning"))?;
    assert_eq!(unknown["has_errors"], false, "{unknown}");
    reopen("object-unknown-endpoint", &authored, &unknown)?;
    println!("Then restoring its normative subtype restores the original checked result");
    element(&mut authored, "order-output")?["kind"] = json!("uml.output_pin");
    assert_eq!(validate(&authored)?, valid);
    Ok(())
}
