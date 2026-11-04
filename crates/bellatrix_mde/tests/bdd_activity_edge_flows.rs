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

fn uniform_rule(
    response: &Value,
    family: &str,
    subject: &str,
    severity: Option<&str>,
) -> FlowResult<()> {
    let error_code = format!("UML_{family}_NODE_EDGES");
    let warning_code = format!("MDE_{family}_NODE_EDGES_UNVERIFIED");
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| {
            diagnostic["subject_id"] == subject
                && (diagnostic["code"] == error_code || diagnostic["code"] == warning_code)
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity);
        assert_eq!(
            matching[0]["code"],
            if severity == "error" {
                error_code
            } else {
                warning_code
            }
        );
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

#[test]
fn fork_and_merge_edge_families_survive_homogeneous_mixed_empty_and_self_loop_repairs()
-> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve Fork and Merge edge-family semantics through repair")
    );
    for (family, node, kind, edges, cardinality) in [
        (
            "FORK",
            "parallel",
            "uml.fork_node",
            ["flow-3", "flow-4", "flow-5"],
            "UML_FORK_NODE_ONE_INCOMING_EDGE",
        ),
        (
            "MERGE",
            "merge",
            "uml.merge_node",
            ["flow-10", "flow-11", "flow-12"],
            "UML_MERGE_NODE_ONE_OUTGOING_EDGE",
        ),
    ] {
        println!("Given an imported Activity with homogeneous ControlFlows around {node}");
        let original = imported_activity()?;
        let baseline = validate(&original)?;
        uniform_rule(&baseline, family, node, None)?;
        println!("When I change one incident edge into an ObjectFlow");
        let mut mixed = original.clone();
        relation(&mut mixed, edges[1])?["kind"] = json!("uml.object_flow");
        let mixture = validate(&mixed)?;
        uniform_rule(&mixture, family, node, Some("error"))?;
        reopen(&format!("{node}-mixed-families"), &mixed, &mixture)?;
        println!("Then changing all incident edges into ObjectFlows restores homogeneity");
        for edge in edges {
            relation(&mut mixed, edge)?["kind"] = json!("uml.object_flow");
        }
        let all_objects = validate(&mixed)?;
        uniform_rule(&all_objects, family, node, None)?;
        reopen(&format!("{node}-all-object-flows"), &mixed, &all_objects)?;
        assert_eq!(validate(&original)?, baseline);

        println!("When I author an isolated {kind} with an empty incident-edge union");
        let mut empty = original.clone();
        empty["model"]["elements"].as_array_mut().ok_or("missing elements")?.push(
            json!({"id":"isolated-node", "kind":kind, "name":"Isolated", "properties":{"activity":{"$ref":"fulfillment"}}}));
        empty["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
            json!({"id":"owns-isolated-node", "kind":"owns", "from":"fulfillment", "to":"isolated-node"}));
        let vacuous = validate(&empty)?;
        uniform_rule(&vacuous, family, "isolated-node", None)?;
        assert!(
            vacuous["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?
                .iter()
                .any(|diagnostic| diagnostic["subject_id"] == "isolated-node"
                    && diagnostic["code"] == cardinality
                    && diagnostic["severity"] == "error")
        );
        reopen(&format!("{node}-empty-union"), &empty, &vacuous)?;
        println!("Then vacuous edge-family truth does not hide the separate cardinality violation");
        println!("When one self-loop supplies both the incoming and outgoing identity");
        empty["model"]["relations"].as_array_mut().ok_or("missing relations")?.extend([
            json!({"id":"isolated-loop", "kind":"uml.control_flow", "from":"isolated-node", "to":"isolated-node",
                "properties":{"activity":{"$ref":"fulfillment"}}}),
            json!({"id":"owns-isolated-loop", "kind":"owns", "from":"fulfillment", "to":"isolated-loop"}),
        ]);
        let self_loop = validate(&empty)?;
        uniform_rule(&self_loop, family, "isolated-node", None)?;
        assert!(
            !self_loop["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?
                .iter()
                .any(|diagnostic| diagnostic["subject_id"] == "isolated-node"
                    && diagnostic["code"] == cardinality)
        );
        reopen(&format!("{node}-single-self-loop"), &empty, &self_loop)?;
        println!("Then the union retains one edge identity and repair remains repeatable");
        assert_eq!(validate(&original)?, baseline);
    }
    Ok(())
}

#[test]
fn fork_and_merge_known_incidence_and_mixtures_do_not_depend_on_unresolved_neighbors()
-> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature").contains(
            "Scenario: Preserve provable Fork and Merge findings with unresolved incidence"
        )
    );
    for (family, node, outgoing, other_edge, role) in [
        ("FORK", "parallel", "flow-4", "flow-5", "source"),
        ("MERGE", "merge", "flow-12", "flow-11", "source"),
    ] {
        let original = imported_activity()?;
        let baseline = validate(&original)?;
        println!("Given homogeneous ControlFlows and a possibly incident ObjectFlow near {node}");
        let mut possible = original.clone();
        possible["model"]["elements"]
            .as_array_mut()
            .ok_or("missing elements")?
            .push(
                json!({"id":"possible-edge", "kind":"uml.object_flow", "name":"Possibly incident",
                "properties":{"activity":{"$ref":"fulfillment"},
                    "source":{"href":"unloaded.uml#source"}, "target":{"$ref":"receive"}}}),
            );
        possible["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
            json!({"id":"owns-possible-edge", "kind":"owns", "from":"fulfillment", "to":"possible-edge"}));
        let uncertain = validate(&possible)?;
        uniform_rule(&uncertain, family, node, Some("warning"))?;
        reopen(
            &format!("{node}-possible-object-flow"),
            &possible,
            &uncertain,
        )?;

        println!("When its known opposite endpoint reaches the node, incidence becomes certain");
        element(&mut possible, "possible-edge")?["properties"]["target"] = json!({"$ref":node});
        let witnessed = validate(&possible)?;
        uniform_rule(&witnessed, family, node, Some("error"))?;
        reopen(
            &format!("{node}-known-target-witness"),
            &possible,
            &witnessed,
        )?;
        println!("And a further unknown edge type cannot hide the already proved mixture");
        possible["model"]["elements"].as_array_mut().ok_or("missing elements")?.push(
            json!({"id":"unknown-edge", "kind":"uml.future_activity_edge", "name":"Unknown edge family",
                "properties":{"source":{"href":"unloaded.uml#source"},"target":{"href":"unloaded.uml#target"}}}));
        possible["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
            json!({"id":"owns-unknown-edge", "kind":"owns", "from":"fulfillment", "to":"unknown-edge"}));
        let mixed_with_unknown = validate(&possible)?;
        uniform_rule(&mixed_with_unknown, family, node, Some("error"))?;
        reopen(
            &format!("{node}-mixture-with-unknown-family"),
            &possible,
            &mixed_with_unknown,
        )?;

        println!("When incidence is known but only the extra edge's family is unresolved");
        let mut unknown_family = original.clone();
        unknown_family["model"]["elements"].as_array_mut().ok_or("missing elements")?.push(
            json!({"id":"known-incidence-unknown-family", "kind":"uml.future_activity_edge", "name":"Unknown family",
                "properties":{"source":{"$ref":"receive"}, "target":{"$ref":node}}}));
        unknown_family["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
            json!({"id":"owns-known-incidence-unknown-family", "kind":"owns", "from":"fulfillment", "to":"known-incidence-unknown-family"}));
        let unknown_family_response = validate(&unknown_family)?;
        uniform_rule(&unknown_family_response, family, node, Some("warning"))?;
        assert_eq!(
            unknown_family_response["has_errors"], false,
            "An unknown extra family does not prove a mixture: {unknown_family_response}"
        );
        reopen(
            &format!("{node}-known-incidence-unknown-family"),
            &unknown_family,
            &unknown_family_response,
        )?;
        assert_eq!(validate(&original)?, baseline);

        println!("When the only possible extra edge is known to be of the same ControlFlow family");
        let mut same_family = original.clone();
        same_family["model"]["elements"].as_array_mut().ok_or("missing elements")?.push(
            json!({"id":"same-family-edge", "kind":"uml.control_flow", "name":"Possibly incident control",
                "properties":{"activity":{"$ref":"fulfillment"}, "source":{"href":"unloaded.uml#source"},
                    "target":{"$ref":"receive"}}}));
        same_family["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
            json!({"id":"owns-same-family-edge", "kind":"owns", "from":"fulfillment", "to":"same-family-edge"}));
        let compatible = validate(&same_family)?;
        uniform_rule(&compatible, family, node, None)?;
        reopen(
            &format!("{node}-possible-same-family"),
            &same_family,
            &compatible,
        )?;

        println!("When an ObjectFlow has contradictory authored and canonical incidence");
        let mut contradictory = original.clone();
        relation(&mut contradictory, outgoing)?["kind"] = json!("uml.object_flow");
        relation(&mut contradictory, outgoing)?["properties"][role] = json!({"$ref":"receive"});
        let unresolved_incidence = validate(&contradictory)?;
        uniform_rule(&unresolved_incidence, family, node, Some("warning"))?;
        reopen(
            &format!("{node}-contradictory-incidence"),
            &contradictory,
            &unresolved_incidence,
        )?;
        println!("Then a second, definitely incident ObjectFlow still proves a mixture");
        relation(&mut contradictory, other_edge)?["kind"] = json!("uml.object_flow");
        let independently_mixed = validate(&contradictory)?;
        uniform_rule(&independently_mixed, family, node, Some("error"))?;
        reopen(
            &format!("{node}-independent-mixture"),
            &contradictory,
            &independently_mixed,
        )?;
        assert_eq!(validate(&original)?, baseline);
    }
    Ok(())
}
