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
            .map(|diagnostic| (
                diagnostic["code"].as_str(),
                diagnostic["subject_id"].as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (Some("UML_CONTROL_FLOW_OBJECT_NODES"), Some("order-data")),
            (
                Some("UML_OBJECT_NODE_OBJECT_FLOW_EDGES"),
                Some("order-input")
            ),
            (
                Some("UML_OBJECT_NODE_OBJECT_FLOW_EDGES"),
                Some("order-output")
            ),
        ],
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

fn join_rule(response: &Value, subject: &str, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| {
            diagnostic["subject_id"] == subject
                && matches!(
                    diagnostic["code"].as_str(),
                    Some(
                        "UML_JOIN_NODE_INCOMING_OBJECT_FLOW"
                            | "MDE_JOIN_NODE_INCOMING_OBJECT_FLOW_UNVERIFIED"
                    )
                )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity);
        assert_eq!(
            matching[0]["code"],
            if severity == "error" {
                "UML_JOIN_NODE_INCOMING_OBJECT_FLOW"
            } else {
                "MDE_JOIN_NODE_INCOMING_OBJECT_FLOW_UNVERIFIED"
            }
        );
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

fn add_activity_edge_object(
    envelope: &mut Value,
    id: &str,
    kind: &str,
    source: Value,
    target: Value,
) -> FlowResult<()> {
    envelope["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .push(
            json!({"id":id,"kind":kind,"name":id,"properties":{"activity":{"$ref":"fulfillment"},
            "source":source,"target":target}}),
        );
    envelope["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .push(json!({"id":format!("owns-{id}"),"kind":"owns","from":"fulfillment","to":id}));
    Ok(())
}

#[test]
fn join_outgoing_family_follows_object_input_existence_through_repair_and_self_loops()
-> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve the Join input-existence conditional across repairs")
    );
    println!("Given an imported JoinNode with ControlFlow inputs and a ControlFlow output");
    let original = imported_activity()?;
    let baseline = validate(&original)?;
    join_rule(&baseline, "synchronize", None)?;
    let mut authored = original.clone();
    println!("When its output becomes ObjectFlow without an incoming ObjectFlow");
    relation(&mut authored, "flow-8")?["kind"] = json!("uml.object_flow");
    let wrong_default_branch = validate(&authored)?;
    join_rule(&wrong_default_branch, "synchronize", Some("error"))?;
    reopen("join-no-object-input", &authored, &wrong_default_branch)?;
    println!("When one incoming edge becomes ObjectFlow, remaining ControlFlow inputs are allowed");
    relation(&mut authored, "flow-6")?["kind"] = json!("uml.object_flow");
    let object_branch = validate(&authored)?;
    join_rule(&object_branch, "synchronize", None)?;
    assert_eq!(object_branch["has_errors"], false, "{object_branch}");
    reopen("join-mixed-inputs-object-output", &authored, &object_branch)?;
    println!(
        "Then changing only the output back to ControlFlow violates the selected object branch"
    );
    relation(&mut authored, "flow-8")?["kind"] = json!("uml.control_flow");
    let wrong_object_branch = validate(&authored)?;
    join_rule(&wrong_object_branch, "synchronize", Some("error"))?;
    reopen(
        "join-object-input-control-output",
        &authored,
        &wrong_object_branch,
    )?;
    relation(&mut authored, "flow-6")?["kind"] = json!("uml.control_flow");
    assert_eq!(validate(&authored)?, baseline);

    println!("When an isolated Join has an empty input and output set");
    let mut isolated = original.clone();
    isolated["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .push(
            json!({"id":"isolated-join","kind":"uml.join_node","name":"Isolated join",
            "properties":{"activity":{"$ref":"fulfillment"}}}),
        );
    isolated["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
        json!({"id":"owns-isolated-join","kind":"owns","from":"fulfillment","to":"isolated-join"}));
    let empty = validate(&isolated)?;
    join_rule(&empty, "isolated-join", None)?;
    assert!(
        empty["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(
                |diagnostic| diagnostic["code"] == "UML_JOIN_NODE_ONE_OUTGOING_EDGE"
                    && diagnostic["subject_id"] == "isolated-join"
            )
    );
    reopen("join-empty-inputs-and-outputs", &isolated, &empty)?;
    println!("Then a ControlFlow self-loop selects and satisfies the control branch");
    add_activity_edge_object(
        &mut isolated,
        "join-loop",
        "uml.control_flow",
        json!({"$ref":"isolated-join"}),
        json!({"$ref":"isolated-join"}),
    )?;
    let control_loop = validate(&isolated)?;
    join_rule(&control_loop, "isolated-join", None)?;
    assert_eq!(control_loop["has_errors"], false, "{control_loop}");
    reopen("join-control-self-loop", &isolated, &control_loop)?;
    println!("And the same identity as ObjectFlow witnesses and satisfies the object branch");
    element(&mut isolated, "join-loop")?["kind"] = json!("uml.object_flow");
    let object_loop = validate(&isolated)?;
    join_rule(&object_loop, "isolated-join", None)?;
    assert_eq!(object_loop["has_errors"], false, "{object_loop}");
    reopen("join-object-self-loop", &isolated, &object_loop)?;
    assert_eq!(validate(&original)?, baseline);
    Ok(())
}

#[test]
fn join_unknown_inputs_and_outputs_preserve_proved_conditional_findings() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Keep unresolved Join inputs distinct from proved absence")
    );
    let original = imported_activity()?;
    let baseline = validate(&original)?;
    println!("Given known control inputs and a possibly incoming ObjectFlow");
    let mut unknown = original.clone();
    add_activity_edge_object(
        &mut unknown,
        "possible-join-input",
        "uml.object_flow",
        json!({"$ref":"receive"}),
        json!({"href":"unloaded.uml#target"}),
    )?;
    let unknown_control_output = validate(&unknown)?;
    join_rule(&unknown_control_output, "synchronize", Some("warning"))?;
    reopen(
        "join-unknown-input-control-output",
        &unknown,
        &unknown_control_output,
    )?;
    println!("When the possible input is known ControlFlow, it cannot change ObjectFlow existence");
    element(&mut unknown, "possible-join-input")?["kind"] = json!("uml.control_flow");
    let irrelevant_control_input = validate(&unknown)?;
    join_rule(&irrelevant_control_input, "synchronize", None)?;
    reopen(
        "join-unknown-control-input",
        &unknown,
        &irrelevant_control_input,
    )?;
    element(&mut unknown, "possible-join-input")?["kind"] = json!("uml.object_flow");
    relation(&mut unknown, "flow-8")?["kind"] = json!("uml.object_flow");
    let unknown_object_output = validate(&unknown)?;
    join_rule(&unknown_object_output, "synchronize", Some("warning"))?;
    reopen(
        "join-unknown-input-object-output",
        &unknown,
        &unknown_object_output,
    )?;
    println!("When an incoming ObjectFlow is independently known, it selects the object branch");
    relation(&mut unknown, "flow-6")?["kind"] = json!("uml.object_flow");
    let known_witness = validate(&unknown)?;
    join_rule(&known_witness, "synchronize", None)?;
    reopen("join-known-object-witness", &unknown, &known_witness)?;
    println!("When a possibly outgoing ControlFlow prevents proving the required output family");
    add_activity_edge_object(
        &mut unknown,
        "possible-join-output",
        "uml.control_flow",
        json!({"href":"unloaded.uml#source"}),
        json!({"$ref":"receive"}),
    )?;
    let unknown_output = validate(&unknown)?;
    join_rule(&unknown_output, "synchronize", Some("warning"))?;
    reopen("join-unknown-wrong-output", &unknown, &unknown_output)?;
    element(&mut unknown, "possible-join-output")?["kind"] = json!("uml.object_flow");
    let compatible_output = validate(&unknown)?;
    join_rule(&compatible_output, "synchronize", None)?;
    reopen(
        "join-unknown-compatible-output",
        &unknown,
        &compatible_output,
    )?;
    relation(&mut unknown, "flow-8")?["kind"] = json!("uml.control_flow");
    let proved_violation = validate(&unknown)?;
    join_rule(&proved_violation, "synchronize", Some("error"))?;
    reopen("join-known-wrong-output", &unknown, &proved_violation)?;

    println!("When incoming incidence is known but its edge family is unresolved");
    let mut unknown_family = original.clone();
    add_activity_edge_object(
        &mut unknown_family,
        "join-unknown-family",
        "uml.future_activity_edge",
        json!({"$ref":"receive"}),
        json!({"$ref":"synchronize"}),
    )?;
    let unresolved_family = validate(&unknown_family)?;
    join_rule(&unresolved_family, "synchronize", Some("warning"))?;
    reopen(
        "join-unknown-input-family",
        &unknown_family,
        &unresolved_family,
    )?;

    println!("When the guard remains unknown but no output can violate either branch");
    let mut empty_output = original.clone();
    add_activity_edge_object(
        &mut empty_output,
        "possible-join-input",
        "uml.object_flow",
        json!({"$ref":"receive"}),
        json!({"href":"unloaded.uml#target"}),
    )?;
    empty_output["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .retain(|relation| {
            relation["id"] != "flow-8"
                && !(relation["kind"] == "owns" && relation["to"] == "flow-8")
        });
    let vacuous_branches = validate(&empty_output)?;
    join_rule(&vacuous_branches, "synchronize", None)?;
    assert!(
        vacuous_branches["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(
                |diagnostic| diagnostic["code"] == "UML_JOIN_NODE_ONE_OUTGOING_EDGE"
                    && diagnostic["subject_id"] == "synchronize"
            )
    );
    reopen(
        "join-unknown-guard-empty-output",
        &empty_output,
        &vacuous_branches,
    )?;
    println!(
        "Then the family conclusion is independent of guard completions, not an OCL null default"
    );
    let mut both_branches_false = original.clone();
    add_activity_edge_object(
        &mut both_branches_false,
        "possible-join-input",
        "uml.object_flow",
        json!({"$ref":"receive"}),
        json!({"href":"unloaded.uml#target"}),
    )?;
    add_activity_edge_object(
        &mut both_branches_false,
        "extra-join-output",
        "uml.object_flow",
        json!({"$ref":"synchronize"}),
        json!({"$ref":"receive"}),
    )?;
    let independent_false = validate(&both_branches_false)?;
    join_rule(&independent_false, "synchronize", Some("error"))?;
    reopen(
        "join-both-branches-false",
        &both_branches_false,
        &independent_false,
    )?;
    assert_eq!(validate(&original)?, baseline);
    Ok(())
}

#[test]
fn decision_edge_exclusion_survives_import_edit_persistence_and_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve Decision edge identity exclusion through repair")
    );
    let original = imported_activity()?;
    let baseline = validate(&original)?;
    uniform_rule(&baseline, "DECISION", "check", None)?;
    let mut edited = original.clone();
    println!("When an ObjectFlow decision input accompanies ordinary ControlFlows");
    add_activity_edge_object(
        &mut edited,
        "decision-input",
        "uml.object_flow",
        json!({"$ref":"receive"}),
        json!({"$ref":"check"}),
    )?;
    let mixed = validate(&edited)?;
    uniform_rule(&mixed, "DECISION", "check", Some("error"))?;
    reopen("decision-mixed", &edited, &mixed)?;
    element(&mut edited, "check")?["properties"]["decisionInputFlow"] =
        json!({"$ref":"decision-input"});
    let excluded = validate(&edited)?;
    uniform_rule(&excluded, "DECISION", "check", None)?;
    reopen("decision-excluded", &edited, &excluded)?;
    let mut too_many_inputs = edited.clone();
    add_activity_edge_object(
        &mut too_many_inputs,
        "third-input",
        "uml.control_flow",
        json!({"$ref":"receive"}),
        json!({"$ref":"check"}),
    )?;
    let cardinality = validate(&too_many_inputs)?;
    uniform_rule(&cardinality, "DECISION", "check", None)?;
    assert!(
        cardinality["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(|diagnostic| diagnostic["subject_id"] == "check"
                && diagnostic["code"] == "UML_DECISION_NODE_INCOMING_OUTGOING_EDGES")
    );
    reopen(
        "decision-exclusion-preserves-cardinality",
        &too_many_inputs,
        &cardinality,
    )?;
    let mut all_objects = edited.clone();
    for id in ["flow-2", "flow-3", "flow-9"] {
        relation(&mut all_objects, id)?["kind"] = json!("uml.object_flow");
    }
    let object_response = validate(&all_objects)?;
    uniform_rule(&object_response, "DECISION", "check", None)?;
    reopen("decision-all-objects", &all_objects, &object_response)?;

    println!("Then exclusion removes one identity, not the entire ObjectFlow family");
    relation(&mut edited, "flow-9")?["kind"] = json!("uml.object_flow");
    let second_object = validate(&edited)?;
    uniform_rule(&second_object, "DECISION", "check", Some("error"))?;
    reopen("decision-second-object", &edited, &second_object)?;
    relation(&mut edited, "flow-9")?["kind"] = json!("uml.control_flow");
    for (name, selector) in [
        ("null", Value::Null),
        ("unrelated", json!({"$ref":"order-data"})),
    ] {
        element(&mut edited, "check")?["properties"]["decisionInputFlow"] = selector;
        let response = validate(&edited)?;
        uniform_rule(&response, "DECISION", "check", Some("error"))?;
        reopen(&format!("decision-{name}"), &edited, &response)?;
    }
    println!("When exclusion is unresolved or malformed it does not establish a violation");
    for (name, selector) in [
        ("href", json!({"href":"external.uml#edge"})),
        ("malformed", json!(42)),
        ("wrong-type", json!({"$ref":"receive"})),
    ] {
        element(&mut edited, "check")?["properties"]["decisionInputFlow"] = selector;
        let response = validate(&edited)?;
        uniform_rule(&response, "DECISION", "check", Some("warning"))?;
        if name == "malformed" || name == "wrong-type" {
            assert!(
                response["diagnostics"]
                    .as_array()
                    .ok_or("missing diagnostics")?
                    .iter()
                    .any(|diagnostic| diagnostic["subject_id"] == "check"
                        && diagnostic["code"] == "UML_PROPERTY_CONTRACT"
                        && diagnostic["severity"] == "error"),
                "{response}"
            );
        } else {
            assert_eq!(response["has_errors"], false, "{response}");
        }
        reopen(&format!("decision-{name}"), &edited, &response)?;
    }
    let mut homogeneous = original.clone();
    element(&mut homogeneous, "check")?["properties"]["decisionInputFlow"] =
        json!({"href":"external.uml#edge"});
    let response = validate(&homogeneous)?;
    uniform_rule(&response, "DECISION", "check", None)?;
    reopen(
        "decision-homogeneous-unknown-exclusion",
        &homogeneous,
        &response,
    )?;

    println!("Then exclusion applies to outgoing identity independently of incoming membership");
    element(&mut edited, "check")?["properties"]["decisionInputFlow"] =
        json!({"$ref":"decision-input"});
    element(&mut edited, "decision-input")?["properties"]["source"] = json!({"$ref":"check"});
    element(&mut edited, "decision-input")?["properties"]["target"] = json!({"$ref":"receive"});
    let outgoing = validate(&edited)?;
    uniform_rule(&outgoing, "DECISION", "check", None)?;
    reopen("decision-excluded-outgoing", &edited, &outgoing)?;
    element(&mut edited, "decision-input")?["properties"]["target"] = json!({"$ref":"check"});
    let self_loop = validate(&edited)?;
    uniform_rule(&self_loop, "DECISION", "check", None)?;
    reopen("decision-excluded-self-loop", &edited, &self_loop)?;
    let repaired = validate(&original)?;
    assert_eq!(repaired, baseline);
    reopen("decision-repaired", &original, &repaired)?;
    Ok(())
}

#[test]
fn object_flow_distribution_flags_preserve_defaults_unknowns_and_repairs() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve ObjectFlow distribution flags through repair")
    );
    for object_record in [false, true] {
        let mut original = imported_activity()?;
        if object_record {
            let edge = relation(&mut original, "order-data")?.clone();
            original["model"]["relations"]
                .as_array_mut()
                .ok_or("missing relations")?
                .retain(|relation| relation["id"] != "order-data");
            let mut properties = edge["properties"].clone();
            properties["source"] = json!({"$ref":"order-output"});
            properties["target"] = json!({"$ref":"order-input"});
            original["model"]["elements"].as_array_mut().ok_or("missing elements")?
                .push(json!({"id":"order-data", "kind":"uml.object_flow", "name":"order-data", "properties":properties}));
        }
        let baseline = validate(&original)?;
        println!("Given an imported ObjectFlow with implicit false distribution defaults");
        assert_eq!(baseline["has_errors"], false, "{baseline}");
        reopen(
            &format!("object-flow-defaults-{object_record}"),
            &original,
            &baseline,
        )?;
        for (name, multicast, multireceive, expected, malformed) in [
            ("null-left", Value::Null, json!(true), Some("warning"), true),
            (
                "null-right",
                json!(true),
                Value::Null,
                Some("warning"),
                true,
            ),
            ("number-right", json!(true), json!(1), Some("warning"), true),
            ("false-false", json!(false), json!(false), None, false),
            ("multicast", json!(true), json!(false), None, false),
            ("multireceive", json!(false), json!(true), None, false),
            ("both", json!(true), json!(true), Some("error"), false),
            (
                "unknown-right",
                json!(true),
                json!("true"),
                Some("warning"),
                true,
            ),
            (
                "unknown-left",
                json!("true"),
                json!(true),
                Some("warning"),
                true,
            ),
            ("false-left", json!(false), json!("true"), None, true),
            ("false-right", json!("true"), json!(false), None, true),
            ("both-unknown", json!([]), json!({}), Some("warning"), true),
        ] {
            println!("When editing distribution flags to {name} in object-record={object_record}");
            let mut edited = original.clone();
            let edge = if object_record {
                element(&mut edited, "order-data")?
            } else {
                relation(&mut edited, "order-data")?
            };
            edge["properties"]["isMulticast"] = multicast;
            edge["properties"]["isMultireceive"] = multireceive;
            let response = validate(&edited)?;
            let diagnostics = response["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?;
            let matching: Vec<_> = diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic["subject_id"] == "order-data"
                        && matches!(
                            diagnostic["code"].as_str(),
                            Some(
                                "UML_OBJECT_FLOW_IS_MULTICAST_OR_IS_MULTIRECEIVE"
                                    | "MDE_OBJECT_FLOW_IS_MULTICAST_OR_IS_MULTIRECEIVE_UNVERIFIED"
                            )
                        )
                })
                .collect();
            if let Some(severity) = expected {
                assert_eq!(matching.len(), 1, "{response}");
                assert_eq!(matching[0]["severity"], severity, "{response}");
            } else {
                assert!(matching.is_empty(), "{response}");
            }
            if malformed {
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic["subject_id"] == "order-data"
                            && diagnostic["code"] == "UML_PROPERTY_VALUE"
                            && diagnostic["severity"] == "error"),
                    "{response}"
                );
            }
            assert_eq!(
                response["has_errors"],
                malformed || expected == Some("error"),
                "{response}"
            );
            reopen(
                &format!("object-flow-flags-{name}-{object_record}"),
                &edited,
                &response,
            )?;
        }
        let repaired = validate(&original)?;
        assert_eq!(repaired, baseline);
        reopen(
            &format!("object-flow-flags-repaired-{object_record}"),
            &original,
            &repaired,
        )?;
    }
    Ok(())
}

#[test]
fn object_flow_selection_requires_object_source_through_persistence_and_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve ObjectFlow selection source semantics through repair")
    );
    for object_record in [false, true] {
        let mut original = imported_activity()?;
        original["model"]["elements"].as_array_mut().ok_or("missing elements")?
            .push(json!({"id":"select-order","kind":"uml.opaque_behavior","name":"selectOrder","properties":{}}));
        original["model"]["relations"]
            .as_array_mut()
            .ok_or("missing relations")?
            .push(
                json!({"id":"owns-select-order","kind":"owns","from":"root","to":"select-order"}),
            );
        if object_record {
            let edge = relation(&mut original, "order-data")?.clone();
            original["model"]["relations"]
                .as_array_mut()
                .ok_or("missing relations")?
                .retain(|relation| relation["id"] != "order-data");
            let mut properties = edge["properties"].clone();
            properties["source"] = json!({"$ref":"order-output"});
            properties["target"] = json!({"$ref":"order-input"});
            original["model"]["elements"].as_array_mut().ok_or("missing elements")?
                .push(json!({"id":"order-data","kind":"uml.object_flow","name":"order-data","properties":properties}));
        }
        let baseline = validate(&original)?;
        assert_eq!(baseline["has_errors"], false, "{baseline}");
        reopen(
            &format!("selection-baseline-{object_record}"),
            &original,
            &baseline,
        )?;
        for (name, source, selection, severity, contract) in [
            (
                "pin",
                "order-output",
                json!({"$ref":"select-order"}),
                None,
                false,
            ),
            (
                "action",
                "receive",
                json!({"$ref":"select-order"}),
                Some("error"),
                false,
            ),
            ("null", "receive", Value::Null, None, false),
            (
                "href",
                "receive",
                json!({"href":"external.uml#selection"}),
                Some("error"),
                false,
            ),
            (
                "href-pin",
                "order-output",
                json!({"href":"external.uml#selection"}),
                None,
                false,
            ),
            (
                "wrong-type",
                "receive",
                json!({"$ref":"receive"}),
                Some("warning"),
                true,
            ),
            ("malformed", "receive", json!(42), Some("warning"), true),
            ("malformed-pin", "order-output", json!(42), None, true),
        ] {
            println!("When authoring ObjectFlow selection {name}, object-record={object_record}");
            let mut edited = original.clone();
            let edge = if object_record {
                element(&mut edited, "order-data")?
            } else {
                relation(&mut edited, "order-data")?
            };
            if object_record {
                edge["properties"]["source"] = json!({"$ref":source});
            } else {
                edge["from"] = json!(source);
                edge["properties"]["source"] = json!({"$ref":source});
            }
            edge["properties"]["selection"] = selection;
            let response = validate(&edited)?;
            selection_rule(&response, severity)?;
            if contract {
                assert!(
                    response["diagnostics"]
                        .as_array()
                        .ok_or("missing diagnostics")?
                        .iter()
                        .any(|d| d["subject_id"] == "order-data"
                            && d["code"] == "UML_PROPERTY_CONTRACT"
                            && d["severity"] == "error"),
                    "{response}"
                );
            }
            assert_eq!(
                response["has_errors"],
                contract || severity == Some("error"),
                "{response}"
            );
            reopen(
                &format!("selection-{name}-{object_record}"),
                &edited,
                &response,
            )?;
        }
        let mut ambiguous = original.clone();
        let edge = if object_record {
            element(&mut ambiguous, "order-data")?
        } else {
            relation(&mut ambiguous, "order-data")?
        };
        edge["properties"]["selection"] = json!({"$ref":"select-order"});
        edge["properties"]["source"] = if object_record {
            json!({"href":"external.uml#source"})
        } else {
            json!({"$ref":"receive"})
        };
        let response = validate(&ambiguous)?;
        selection_rule(&response, Some("warning"))?;
        reopen(
            &format!("selection-ambiguous-source-{object_record}"),
            &ambiguous,
            &response,
        )?;
        assert_eq!(validate(&original)?, baseline);
        reopen(
            &format!("selection-repaired-{object_record}"),
            &original,
            &baseline,
        )?;
    }
    Ok(())
}

fn selection_rule(response: &Value, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == "order-data"
                && matches!(
                    d["code"].as_str(),
                    Some(
                        "UML_OBJECT_FLOW_SELECTION_BEHAVIOR"
                            | "MDE_OBJECT_FLOW_SELECTION_BEHAVIOR_UNVERIFIED"
                    )
                )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity, "{response}");
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

#[test]
fn activity_parameter_direction_survives_unknown_incidence_persistence_and_repair() -> FlowResult<()>
{
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve ActivityParameterNode flow direction through repair")
    );
    let mut original = imported_activity()?;
    original["model"]["elements"].as_array_mut().ok_or("missing elements")?.extend([
        json!({"id":"activity-parameter","kind":"uml.parameter","name":"incomingOrder","properties":{"direction":"in","type":{"$ref":"order"}}}),
        json!({"id":"parameter-node","kind":"uml.activity_parameter_node","name":"parameterNode","properties":{"parameter":{"$ref":"activity-parameter"},"type":{"$ref":"order"}}}),
    ]);
    original["model"]["relations"].as_array_mut().ok_or("missing relations")?.extend([
        json!({"id":"owns-activity-parameter","kind":"owns","from":"fulfillment","to":"activity-parameter"}),
        json!({"id":"owns-parameter-node","kind":"owns","from":"fulfillment","to":"parameter-node"}),
    ]);
    let baseline = validate(&original)?;
    parameter_direction_rule(&baseline, None)?;
    reopen("parameter-empty", &original, &baseline)?;
    for object_record in [false, true] {
        let mut edited = original.clone();
        if object_record {
            add_activity_edge_object(
                &mut edited,
                "parameter-input",
                "uml.object_flow",
                json!({"$ref":"order-output"}),
                json!({"$ref":"parameter-node"}),
            )?;
        } else {
            edited["model"]["relations"].as_array_mut().ok_or("missing relations")?.extend([
                json!({"id":"parameter-input","kind":"uml.object_flow","from":"order-output","to":"parameter-node","properties":{"activity":{"$ref":"fulfillment"}}}),
                json!({"id":"owns-parameter-input","kind":"owns","from":"fulfillment","to":"parameter-input"}),
            ]);
        }
        let incoming = validate(&edited)?;
        parameter_direction_rule(&incoming, None)?;
        reopen(
            &format!("parameter-incoming-{object_record}"),
            &edited,
            &incoming,
        )?;
        add_activity_edge_object(
            &mut edited,
            "parameter-output",
            "uml.object_flow",
            json!({"href":"external.uml#source"}),
            json!({"$ref":"order-input"}),
        )?;
        let uncertain = validate(&edited)?;
        parameter_direction_rule(&uncertain, Some("warning"))?;
        reopen(
            &format!("parameter-unknown-outgoing-{object_record}"),
            &edited,
            &uncertain,
        )?;
        element(&mut edited, "parameter-output")?["properties"]["source"] =
            json!({"$ref":"parameter-node"});
        let both = validate(&edited)?;
        parameter_direction_rule(&both, Some("error"))?;
        reopen(&format!("parameter-both-{object_record}"), &edited, &both)?;
        add_activity_edge_object(
            &mut edited,
            "parameter-unknown",
            "uml.future_edge",
            json!({"href":"external.uml#source"}),
            json!({"$ref":"parameter-node"}),
        )?;
        let independent = validate(&edited)?;
        parameter_direction_rule(&independent, Some("error"))?;
        reopen(
            &format!("parameter-known-witnesses-{object_record}"),
            &edited,
            &independent,
        )?;
        let edge = if object_record {
            element(&mut edited, "parameter-input")?
        } else {
            relation(&mut edited, "parameter-input")?
        };
        if object_record {
            edge["properties"]["target"] = json!({"$ref":"order-input"});
        } else {
            edge["to"] = json!("order-input");
        }
        let unknown_family = validate(&edited)?;
        parameter_direction_rule(&unknown_family, Some("warning"))?;
        reopen(
            &format!("parameter-unknown-family-{object_record}"),
            &edited,
            &unknown_family,
        )?;
        element(&mut edited, "parameter-unknown")?["properties"]["target"] =
            json!({"$ref":"order-input"});
        let outgoing_only = validate(&edited)?;
        parameter_direction_rule(&outgoing_only, None)?;
        reopen(
            &format!("parameter-outgoing-only-{object_record}"),
            &edited,
            &outgoing_only,
        )?;
        element(&mut edited, "parameter-output")?["properties"]["target"] =
            json!({"$ref":"parameter-node"});
        let self_loop = validate(&edited)?;
        parameter_direction_rule(&self_loop, Some("error"))?;
        reopen(
            &format!("parameter-self-loop-{object_record}"),
            &edited,
            &self_loop,
        )?;
        element(&mut edited, "parameter-output")?["kind"] = json!("uml.control_flow");
        element(&mut edited, "parameter-node")?["properties"]["isControlType"] = json!(true);
        let control_loop = validate(&edited)?;
        parameter_direction_rule(&control_loop, Some("error"))?;
        reopen(
            &format!("parameter-control-loop-{object_record}"),
            &edited,
            &control_loop,
        )?;
        element(&mut edited, "parameter-output")?["properties"]["source"] = json!(42);
        let malformed = validate(&edited)?;
        parameter_direction_rule(&malformed, Some("warning"))?;
        assert!(
            malformed["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?
                .iter()
                .any(|d| d["subject_id"] == "parameter-output"
                    && d["code"] == "UML_PROPERTY_CONTRACT"
                    && d["severity"] == "error"),
            "{malformed}"
        );
        reopen(
            &format!("parameter-malformed-source-{object_record}"),
            &edited,
            &malformed,
        )?;
        add_activity_edge_object(
            &mut edited,
            "independent-output",
            "uml.object_flow",
            json!({"$ref":"parameter-node"}),
            json!({"$ref":"order-input"}),
        )?;
        let witnessed = validate(&edited)?;
        parameter_direction_rule(&witnessed, Some("error"))?;
        reopen(
            &format!("parameter-malformed-opposite-witness-{object_record}"),
            &edited,
            &witnessed,
        )?;
    }
    assert_eq!(validate(&original)?, baseline);
    reopen("parameter-repaired", &original, &baseline)?;
    Ok(())
}

fn parameter_direction_rule(response: &Value, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == "parameter-node"
                && matches!(
                    d["code"].as_str(),
                    Some(
                        "UML_ACTIVITY_PARAMETER_NODE_NO_EDGES"
                            | "MDE_ACTIVITY_PARAMETER_NODE_NO_EDGES_UNVERIFIED"
                    )
                )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity, "{response}");
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

#[test]
fn activity_parameter_enum_direction_tracks_flow_orientation_and_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve Activity Parameter enum directions through repair")
    );
    let mut original = imported_activity()?;
    original["model"]["elements"].as_array_mut().ok_or("missing elements")?.extend([
        json!({"id":"direction-parameter","kind":"uml.parameter","name":"orderParam","properties":{"type":{"$ref":"order"}}}),
        json!({"id":"direction-node","kind":"uml.activity_parameter_node","name":"parameterNode","properties":{"parameter":{"$ref":"direction-parameter"},"type":{"$ref":"order"}}}),
    ]);
    original["model"]["relations"].as_array_mut().ok_or("missing relations")?.extend([
        json!({"id":"owns-direction-parameter","kind":"owns","from":"fulfillment","to":"direction-parameter"}),
        json!({"id":"owns-direction-node","kind":"owns","from":"fulfillment","to":"direction-node"}),
    ]);
    let baseline = validate(&original)?;
    parameter_enum_rule(&baseline, "INCOMING", None)?;
    parameter_enum_rule(&baseline, "OUTGOING", None)?;
    reopen("parameter-enum-empty", &original, &baseline)?;
    for outgoing in [false, true] {
        let relevant = if outgoing { "INCOMING" } else { "OUTGOING" };
        let irrelevant = if outgoing { "OUTGOING" } else { "INCOMING" };
        let mut connected = original.clone();
        add_activity_edge_object(
            &mut connected,
            "direction-edge",
            "uml.object_flow",
            json!({"$ref":if outgoing {"direction-node"} else {"order-output"}}),
            json!({"$ref":if outgoing {"order-input"} else {"direction-node"}}),
        )?;
        let defaulted = validate(&connected)?;
        parameter_enum_rule(
            &defaulted,
            relevant,
            if outgoing { None } else { Some("error") },
        )?;
        reopen(
            &format!("parameter-enum-default-{outgoing}"),
            &connected,
            &defaulted,
        )?;
        for direction in ["in", "out", "inout", "return"] {
            let mut edited = connected.clone();
            element(&mut edited, "direction-parameter")?["properties"]["direction"] =
                json!(direction);
            let response = validate(&edited)?;
            let allowed = if outgoing {
                matches!(direction, "in" | "inout")
            } else {
                matches!(direction, "out" | "inout" | "return")
            };
            parameter_enum_rule(
                &response,
                relevant,
                if allowed { None } else { Some("error") },
            )?;
            parameter_enum_rule(&response, irrelevant, None)?;
            reopen(
                &format!("parameter-enum-{direction}-{outgoing}"),
                &edited,
                &response,
            )?;
        }
        let mut invalid = connected.clone();
        element(&mut invalid, "direction-parameter")?["properties"]["direction"] =
            json!("sideways");
        let malformed = validate(&invalid)?;
        parameter_enum_rule(&malformed, relevant, Some("warning"))?;
        assert!(
            malformed["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?
                .iter()
                .any(|d| d["subject_id"] == "direction-parameter"
                    && d["code"] == "UML_PROPERTY_VALUE"
                    && d["severity"] == "error")
        );
        reopen(
            &format!("parameter-enum-malformed-{outgoing}"),
            &invalid,
            &malformed,
        )?;
        element(&mut invalid, "direction-node")?["properties"]["parameter"] =
            json!({"href":"external.uml#parameter"});
        let external = validate(&invalid)?;
        parameter_enum_rule(&external, relevant, Some("warning"))?;
        reopen(
            &format!("parameter-enum-external-{outgoing}"),
            &invalid,
            &external,
        )?;
        for absent in [false, true] {
            let mut missing = connected.clone();
            let properties = element(&mut missing, "direction-node")?["properties"]
                .as_object_mut()
                .ok_or("missing properties")?;
            if absent {
                properties.remove("parameter");
            } else {
                properties.insert("parameter".into(), Value::Null);
            }
            let response = validate(&missing)?;
            parameter_enum_rule(&response, relevant, Some("warning"))?;
            reopen(
                &format!("parameter-enum-missing-{absent}-{outgoing}"),
                &missing,
                &response,
            )?;
        }
        let mut null_direction = connected.clone();
        element(&mut null_direction, "direction-parameter")?["properties"]["direction"] =
            Value::Null;
        let response = validate(&null_direction)?;
        parameter_enum_rule(&response, relevant, Some("warning"))?;
        assert!(
            response["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?
                .iter()
                .any(|d| d["subject_id"] == "direction-parameter"
                    && d["code"] == "UML_PROPERTY_VALUE"
                    && d["severity"] == "error")
        );
        reopen(
            &format!("parameter-enum-null-direction-{outgoing}"),
            &null_direction,
            &response,
        )?;
        let mut uncertain = connected.clone();
        element(&mut uncertain, "direction-parameter")?["properties"]["direction"] =
            json!(if outgoing { "out" } else { "in" });
        add_activity_edge_object(
            &mut uncertain,
            "uncertain-opposite",
            "uml.object_flow",
            if outgoing {
                json!({"$ref":"order-output"})
            } else {
                json!({"href":"external.uml#source"})
            },
            if outgoing {
                json!({"href":"external.uml#target"})
            } else {
                json!({"$ref":"order-input"})
            },
        )?;
        let unknown_guard = validate(&uncertain)?;
        parameter_enum_rule(&unknown_guard, relevant, Some("warning"))?;
        reopen(
            &format!("parameter-enum-unknown-guard-{outgoing}"),
            &uncertain,
            &unknown_guard,
        )?;
        element(&mut uncertain, "direction-parameter")?["properties"]["direction"] = json!("inout");
        let valid_consequent = validate(&uncertain)?;
        parameter_enum_rule(&valid_consequent, relevant, None)?;
        reopen(
            &format!("parameter-enum-valid-consequent-{outgoing}"),
            &uncertain,
            &valid_consequent,
        )?;
        element(&mut invalid, "direction-edge")?["properties"]
            [if outgoing { "target" } else { "source" }] = json!({"$ref":"direction-node"});
        let both = validate(&invalid)?;
        parameter_enum_rule(&both, "INCOMING", None)?;
        parameter_enum_rule(&both, "OUTGOING", None)?;
        reopen(
            &format!("parameter-enum-both-directions-{outgoing}"),
            &invalid,
            &both,
        )?;
    }
    assert_eq!(validate(&original)?, baseline);
    reopen("parameter-enum-repaired", &original, &baseline)?;
    Ok(())
}

fn parameter_enum_rule(response: &Value, role: &str, severity: Option<&str>) -> FlowResult<()> {
    let error = format!("UML_ACTIVITY_PARAMETER_NODE_NO_{role}_EDGES");
    let warning = format!("MDE_ACTIVITY_PARAMETER_NODE_NO_{role}_EDGES_UNVERIFIED");
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == "direction-node" && (d["code"] == error || d["code"] == warning)
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity, "{response}");
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

#[test]
fn activity_parameter_type_identity_survives_nullable_external_types_and_repair() -> FlowResult<()>
{
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve Activity Parameter type identity through repair")
    );
    let mut original = imported_activity()?;
    original["model"]["elements"].as_array_mut().ok_or("missing elements")?.extend([
        json!({"id":"type-parameter","kind":"uml.parameter","name":"orderParam","properties":{"type":{"$ref":"order"}}}),
        json!({"id":"type-node","kind":"uml.activity_parameter_node","name":"parameterNode","properties":{"parameter":{"$ref":"type-parameter"},"type":{"$ref":"order"}}}),
        json!({"id":"special-order","kind":"uml.class","name":"SpecialOrder","properties":{}}),
    ]);
    original["model"]["relations"].as_array_mut().ok_or("missing relations")?.extend([
        json!({"id":"owns-type-parameter","kind":"owns","from":"fulfillment","to":"type-parameter"}),
        json!({"id":"owns-type-node","kind":"owns","from":"fulfillment","to":"type-node"}),
        json!({"id":"owns-special-order","kind":"owns","from":"root","to":"special-order"}),
        json!({"id":"special-order-base","kind":"uml.generalization","from":"special-order","to":"order","properties":{}}),
        json!({"id":"owns-special-order-base","kind":"owns","from":"special-order","to":"special-order-base"}),
    ]);
    let baseline = validate(&original)?;
    parameter_type_rule(&baseline, None)?;
    assert_eq!(baseline["has_errors"], false, "{baseline}");
    reopen("parameter-type-baseline", &original, &baseline)?;
    for (name, node_type, parameter_type, severity, contract) in [
        (
            "subtype-not-equal",
            json!({"$ref":"special-order"}),
            json!({"$ref":"order"}),
            Some("error"),
            false,
        ),
        (
            "supertype-not-equal",
            json!({"$ref":"order"}),
            json!({"$ref":"special-order"}),
            Some("error"),
            false,
        ),
        (
            "same-subtype",
            json!({"$ref":"special-order"}),
            json!({"$ref":"special-order"}),
            None,
            false,
        ),
        ("both-null", Value::Null, Value::Null, None, false),
        (
            "node-null",
            Value::Null,
            json!({"$ref":"order"}),
            Some("error"),
            false,
        ),
        (
            "parameter-null",
            json!({"$ref":"order"}),
            Value::Null,
            Some("error"),
            false,
        ),
        (
            "external-node",
            json!({"href":"external.uml#Order"}),
            json!({"$ref":"order"}),
            Some("warning"),
            false,
        ),
        (
            "external-parameter",
            json!({"$ref":"order"}),
            json!({"href":"external.uml#Order"}),
            Some("warning"),
            false,
        ),
        (
            "malformed-node",
            json!(42),
            json!({"$ref":"order"}),
            Some("warning"),
            true,
        ),
        (
            "malformed-parameter",
            json!({"$ref":"order"}),
            json!(42),
            Some("warning"),
            true,
        ),
        (
            "wrong-type",
            json!({"$ref":"type-parameter"}),
            json!({"$ref":"order"}),
            Some("warning"),
            true,
        ),
    ] {
        let mut edited = original.clone();
        element(&mut edited, "type-node")?["properties"]["type"] = node_type;
        element(&mut edited, "type-parameter")?["properties"]["type"] = parameter_type;
        let response = validate(&edited)?;
        parameter_type_rule(&response, severity)?;
        if contract {
            assert!(
                response["diagnostics"]
                    .as_array()
                    .ok_or("missing diagnostics")?
                    .iter()
                    .any(|d| matches!(
                        d["subject_id"].as_str(),
                        Some("type-node" | "type-parameter")
                    ) && d["code"] == "UML_TYPED_ELEMENT_TYPE"
                        && d["severity"] == "error"),
                "{response}"
            );
        }
        assert_eq!(
            response["has_errors"],
            contract || severity == Some("error"),
            "{response}"
        );
        reopen(&format!("parameter-type-{name}"), &edited, &response)?;
    }
    let mut absent = original.clone();
    for subject in ["type-node", "type-parameter"] {
        element(&mut absent, subject)?["properties"]
            .as_object_mut()
            .ok_or("missing properties")?
            .remove("type");
    }
    let untyped = validate(&absent)?;
    parameter_type_rule(&untyped, None)?;
    reopen("parameter-type-both-absent", &absent, &untyped)?;
    element(&mut absent, "type-node")?["properties"]["type"] = Value::Null;
    let null_absent = validate(&absent)?;
    parameter_type_rule(&null_absent, None)?;
    reopen("parameter-type-null-absent", &absent, &null_absent)?;
    element(&mut absent, "type-node")?["properties"]["parameter"] =
        json!({"href":"external.uml#parameter"});
    let external = validate(&absent)?;
    parameter_type_rule(&external, Some("warning"))?;
    reopen(
        "parameter-type-external-parameter-reference",
        &absent,
        &external,
    )?;
    element(&mut absent, "type-node")?["properties"]
        .as_object_mut()
        .ok_or("missing properties")?
        .remove("parameter");
    let missing = validate(&absent)?;
    parameter_type_rule(&missing, Some("warning"))?;
    reopen(
        "parameter-type-missing-parameter-reference",
        &absent,
        &missing,
    )?;
    element(&mut absent, "type-node")?["properties"]["parameter"] = Value::Null;
    let null_parameter = validate(&absent)?;
    parameter_type_rule(&null_parameter, Some("warning"))?;
    reopen(
        "parameter-type-null-parameter-reference",
        &absent,
        &null_parameter,
    )?;
    let mut dangling = original.clone();
    element(&mut dangling, "type-node")?["properties"]["type"] = json!({"$ref":"missing-type"});
    let unresolved = validate(&dangling)?;
    parameter_type_rule(&unresolved, Some("warning"))?;
    reopen("parameter-type-dangling-reference", &dangling, &unresolved)?;
    assert_eq!(validate(&original)?, baseline);
    reopen("parameter-type-repaired", &original, &baseline)?;
    Ok(())
}

fn parameter_type_rule(response: &Value, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == "type-node"
                && matches!(
                    d["code"].as_str(),
                    Some(
                        "UML_ACTIVITY_PARAMETER_NODE_SAME_TYPE"
                            | "MDE_ACTIVITY_PARAMETER_NODE_SAME_TYPE_UNVERIFIED"
                    )
                )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity, "{response}");
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

#[test]
fn object_node_incident_families_survive_defaults_unknowns_and_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve ObjectNode incident edge families through repair")
    );
    let mut original = imported_activity()?;
    original["model"]["elements"].as_array_mut().ok_or("missing elements")?.push(
        json!({"id":"buffer-node","kind":"uml.central_buffer_node","name":"OrderBuffer","properties":{"type":{"$ref":"order"}}}));
    original["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .push(
            json!({"id":"owns-buffer-node","kind":"owns","from":"fulfillment","to":"buffer-node"}),
        );
    let baseline = validate(&original)?;
    object_node_family_rule(&baseline, None)?;
    reopen("object-node-vacuous", &original, &baseline)?;
    for object_record in [false, true] {
        let mut edited = original.clone();
        if object_record {
            add_activity_edge_object(
                &mut edited,
                "buffer-edge",
                "uml.object_flow",
                json!({"$ref":"buffer-node"}),
                json!({"$ref":"receive"}),
            )?;
        } else {
            edited["model"]["relations"].as_array_mut().ok_or("missing relations")?.extend([
                json!({"id":"buffer-edge","kind":"uml.object_flow","from":"buffer-node","to":"receive","properties":{"activity":{"$ref":"fulfillment"}}}),
                json!({"id":"owns-buffer-edge","kind":"owns","from":"fulfillment","to":"buffer-edge"}),
            ]);
        }
        let objects = validate(&edited)?;
        object_node_family_rule(&objects, None)?;
        reopen(
            &format!("object-node-object-edge-{object_record}"),
            &edited,
            &objects,
        )?;
        let edge = if object_record {
            element(&mut edited, "buffer-edge")?
        } else {
            relation(&mut edited, "buffer-edge")?
        };
        edge["kind"] = json!("uml.control_flow");
        // The canonical binary Relation adapter maps concrete CF/OF only;
        // an abstract ActivityEdge is represented as an object record here.
        if object_record {
            let mut base_edge = edited.clone();
            element(&mut base_edge, "buffer-edge")?["kind"] = json!("uml.activity_edge");
            let non_object = validate(&base_edge)?;
            object_node_family_rule(&non_object, Some("error"))?;
            reopen("object-node-base-activity-edge", &base_edge, &non_object)?;
        }
        let control = validate(&edited)?;
        object_node_family_rule(&control, Some("error"))?;
        reopen(
            &format!("object-node-control-edge-{object_record}"),
            &edited,
            &control,
        )?;
        element(&mut edited, "buffer-node")?["properties"]["isControlType"] = json!(true);
        let enabled = validate(&edited)?;
        object_node_family_rule(&enabled, None)?;
        reopen(
            &format!("object-node-control-type-{object_record}"),
            &edited,
            &enabled,
        )?;
        element(&mut edited, "buffer-node")?["properties"]["isControlType"] = json!("false");
        let malformed = validate(&edited)?;
        object_node_family_rule(&malformed, Some("warning"))?;
        assert!(
            malformed["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?
                .iter()
                .any(|d| d["subject_id"] == "buffer-node"
                    && d["code"] == "UML_PROPERTY_VALUE"
                    && d["severity"] == "error")
        );
        reopen(
            &format!("object-node-malformed-control-{object_record}"),
            &edited,
            &malformed,
        )?;
        let edge = if object_record {
            element(&mut edited, "buffer-edge")?
        } else {
            relation(&mut edited, "buffer-edge")?
        };
        edge["kind"] = json!("uml.object_flow");
        let sufficient = validate(&edited)?;
        object_node_family_rule(&sufficient, None)?;
        reopen(
            &format!("object-node-valid-consequent-{object_record}"),
            &edited,
            &sufficient,
        )?;
        element(&mut edited, "buffer-node")?["properties"]["isControlType"] = json!(false);
        add_activity_edge_object(
            &mut edited,
            "possible-buffer-edge",
            "uml.control_flow",
            json!({"href":"external.uml#source"}),
            json!({"$ref":"receive"}),
        )?;
        let unknown = validate(&edited)?;
        object_node_family_rule(&unknown, Some("warning"))?;
        reopen(
            &format!("object-node-possible-control-{object_record}"),
            &edited,
            &unknown,
        )?;
        element(&mut edited, "possible-buffer-edge")?["kind"] = json!("uml.object_flow");
        let possible_object = validate(&edited)?;
        object_node_family_rule(&possible_object, None)?;
        reopen(
            &format!("object-node-possible-object-{object_record}"),
            &edited,
            &possible_object,
        )?;
        element(&mut edited, "possible-buffer-edge")?["kind"] = json!("uml.future_edge");
        element(&mut edited, "possible-buffer-edge")?["properties"]["target"] =
            json!({"$ref":"buffer-node"});
        let unknown_family = validate(&edited)?;
        object_node_family_rule(&unknown_family, Some("warning"))?;
        reopen(
            &format!("object-node-unknown-family-{object_record}"),
            &edited,
            &unknown_family,
        )?;
        let mut decisive = edited.clone();
        let edge = if object_record {
            element(&mut decisive, "buffer-edge")?
        } else {
            relation(&mut decisive, "buffer-edge")?
        };
        edge["kind"] = json!("uml.control_flow");
        let known_with_unknown = validate(&decisive)?;
        object_node_family_rule(&known_with_unknown, Some("error"))?;
        reopen(
            &format!("object-node-known-with-unknown-{object_record}"),
            &decisive,
            &known_with_unknown,
        )?;
        element(&mut edited, "buffer-node")?["properties"]["isControlType"] = json!(true);
        let unconstrained = validate(&edited)?;
        object_node_family_rule(&unconstrained, None)?;
        reopen(
            &format!("object-node-control-unknown-family-{object_record}"),
            &edited,
            &unconstrained,
        )?;
        element(&mut edited, "buffer-node")?["properties"]["isControlType"] = json!(false);
        element(&mut edited, "possible-buffer-edge")?["kind"] = json!("uml.control_flow");
        let incoming_witness = validate(&edited)?;
        object_node_family_rule(&incoming_witness, Some("error"))?;
        reopen(
            &format!("object-node-known-incoming-{object_record}"),
            &edited,
            &incoming_witness,
        )?;
        element(&mut edited, "possible-buffer-edge")?["properties"]["source"] =
            json!({"$ref":"buffer-node"});
        let self_loop = validate(&edited)?;
        object_node_family_rule(&self_loop, Some("error"))?;
        reopen(
            &format!("object-node-self-loop-{object_record}"),
            &edited,
            &self_loop,
        )?;
    }
    assert_eq!(validate(&original)?, baseline);
    reopen("object-node-repaired", &original, &baseline)?;
    Ok(())
}

fn object_node_family_rule(response: &Value, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == "buffer-node"
                && matches!(
                    d["code"].as_str(),
                    Some(
                        "UML_OBJECT_NODE_OBJECT_FLOW_EDGES"
                            | "MDE_OBJECT_NODE_OBJECT_FLOW_EDGES_UNVERIFIED"
                    )
                )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity, "{response}");
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

#[test]
fn object_node_selection_ordering_equivalence_survives_defaults_and_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature").contains(
            "Scenario: Preserve ObjectNode selection ordering equivalence through repair"
        )
    );
    let mut original = imported_activity()?;
    original["model"]["elements"].as_array_mut().ok_or("missing elements")?.push(
        json!({"id":"node-selection","kind":"uml.opaque_behavior","name":"chooseToken","properties":{}}));
    original["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .push(
            json!({"id":"owns-node-selection","kind":"owns","from":"root","to":"node-selection"}),
        );
    let baseline = validate(&original)?;
    object_node_selection_rule(&baseline, None)?;
    assert_eq!(baseline["has_errors"], false, "{baseline}");
    reopen("node-selection-defaults", &original, &baseline)?;
    for ordering in ["FIFO", "LIFO", "unordered", "ordered"] {
        for present in [false, true] {
            let mut edited = original.clone();
            element(&mut edited, "order-output")?["properties"]["ordering"] = json!(ordering);
            element(&mut edited, "order-output")?["properties"]["selection"] = if present {
                json!({"$ref":"node-selection"})
            } else {
                Value::Null
            };
            let response = validate(&edited)?;
            let valid = present == (ordering == "ordered");
            object_node_selection_rule(&response, if valid { None } else { Some("error") })?;
            assert_eq!(response["has_errors"], !valid, "{response}");
            reopen(
                &format!("node-selection-{ordering}-{present}"),
                &edited,
                &response,
            )?;
        }
    }
    let mut defaulted = original.clone();
    element(&mut defaulted, "order-output")?["properties"]["selection"] =
        json!({"$ref":"node-selection"});
    let default_violation = validate(&defaulted)?;
    object_node_selection_rule(&default_violation, Some("error"))?;
    assert!(
        element(&mut defaulted, "order-output")?["properties"]
            .get("ordering")
            .is_none()
    );
    reopen(
        "node-selection-implicit-FIFO",
        &defaulted,
        &default_violation,
    )?;
    for (name, ordering, selection, severity, value_error, contract_error) in [
        (
            "external-ordered",
            json!("ordered"),
            json!({"href":"external.uml#selection"}),
            None,
            false,
            false,
        ),
        (
            "external-FIFO",
            json!("FIFO"),
            json!({"href":"external.uml#selection"}),
            Some("error"),
            false,
            false,
        ),
        (
            "malformed-selection",
            json!("ordered"),
            json!(42),
            Some("warning"),
            false,
            true,
        ),
        (
            "wrong-selection-type",
            json!("FIFO"),
            json!({"$ref":"order"}),
            Some("warning"),
            false,
            true,
        ),
        (
            "invalid-ordering-present",
            json!("fifo"),
            json!({"$ref":"node-selection"}),
            Some("warning"),
            true,
            false,
        ),
        (
            "invalid-ordering-absent",
            json!(false),
            Value::Null,
            Some("warning"),
            true,
            false,
        ),
        (
            "null-ordering",
            Value::Null,
            Value::Null,
            Some("warning"),
            true,
            false,
        ),
    ] {
        let mut edited = original.clone();
        element(&mut edited, "order-output")?["properties"]["ordering"] = ordering;
        element(&mut edited, "order-output")?["properties"]["selection"] = selection;
        let response = validate(&edited)?;
        object_node_selection_rule(&response, severity)?;
        for (required, code) in [
            (value_error, "UML_PROPERTY_VALUE"),
            (contract_error, "UML_PROPERTY_CONTRACT"),
        ] {
            if required {
                assert!(
                    response["diagnostics"]
                        .as_array()
                        .ok_or("missing diagnostics")?
                        .iter()
                        .any(|d| d["subject_id"] == "order-output"
                            && d["code"] == code
                            && d["severity"] == "error"),
                    "{response}"
                );
            }
        }
        assert_eq!(
            response["has_errors"],
            value_error || contract_error || severity == Some("error"),
            "{response}"
        );
        reopen(&format!("node-selection-{name}"), &edited, &response)?;
    }
    let mut dangling = original.clone();
    element(&mut dangling, "order-output")?["properties"]["selection"] =
        json!({"$ref":"missing-behavior"});
    let unresolved = validate(&dangling)?;
    object_node_selection_rule(&unresolved, Some("warning"))?;
    reopen("node-selection-dangling", &dangling, &unresolved)?;
    assert_eq!(validate(&original)?, baseline);
    reopen("node-selection-repaired", &original, &baseline)?;
    Ok(())
}

fn object_node_selection_rule(response: &Value, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == "order-output"
                && matches!(
                    d["code"].as_str(),
                    Some(
                        "UML_OBJECT_NODE_SELECTION_BEHAVIOR"
                            | "MDE_OBJECT_NODE_SELECTION_BEHAVIOR_UNVERIFIED"
                    )
                )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity, "{response}");
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}

#[test]
fn pin_control_flag_implication_survives_defaults_unknowns_and_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/activity_edge_lifecycle.feature")
            .contains("Scenario: Preserve Pin control flag implication through repair")
    );
    let original = imported_activity()?;
    let baseline = validate(&original)?;
    for pin in ["order-input", "order-output"] {
        pin_control_rule(&baseline, pin, None)?;
        reopen(&format!("pin-control-defaults-{pin}"), &original, &baseline)?;
        let mut implicit = original.clone();
        element(&mut implicit, pin)?["properties"]["isControl"] = json!(true);
        let default_type = validate(&implicit)?;
        pin_control_rule(&default_type, pin, Some("error"))?;
        assert!(
            element(&mut implicit, pin)?["properties"]
                .get("isControlType")
                .is_none()
        );
        reopen(
            &format!("pin-control-default-type-{pin}"),
            &implicit,
            &default_type,
        )?;
        for (name, control, control_type, severity, malformed) in [
            ("both-false", json!(false), json!(false), None, false),
            (
                "control-without-type",
                json!(true),
                json!(false),
                Some("error"),
                false,
            ),
            (
                "type-without-control",
                json!(false),
                json!(true),
                None,
                false,
            ),
            ("both-true", json!(true), json!(true), None, false),
            (
                "unknown-control-false-type",
                json!("true"),
                json!(false),
                Some("warning"),
                true,
            ),
            (
                "unknown-control-true-type",
                json!("true"),
                json!(true),
                None,
                true,
            ),
            (
                "false-control-unknown-type",
                json!(false),
                json!("true"),
                None,
                true,
            ),
            (
                "true-control-unknown-type",
                json!(true),
                json!("true"),
                Some("warning"),
                true,
            ),
            (
                "null-control",
                Value::Null,
                json!(false),
                Some("warning"),
                true,
            ),
            (
                "null-control-type",
                json!(true),
                Value::Null,
                Some("warning"),
                true,
            ),
            ("both-unknown", json!([]), json!({}), Some("warning"), true),
        ] {
            let mut edited = original.clone();
            element(&mut edited, pin)?["properties"]["isControl"] = control;
            element(&mut edited, pin)?["properties"]["isControlType"] = control_type;
            let response = validate(&edited)?;
            pin_control_rule(&response, pin, severity)?;
            if malformed {
                assert!(
                    response["diagnostics"]
                        .as_array()
                        .ok_or("missing diagnostics")?
                        .iter()
                        .any(|d| d["subject_id"] == pin
                            && d["code"] == "UML_PROPERTY_VALUE"
                            && d["severity"] == "error"),
                    "{response}"
                );
            }
            assert_eq!(
                response["has_errors"],
                malformed || severity == Some("error"),
                "{response}"
            );
            reopen(&format!("pin-control-{pin}-{name}"), &edited, &response)?;
        }
    }
    let repaired = validate(&original)?;
    assert_eq!(repaired, baseline);
    reopen("pin-control-repaired", &original, &repaired)?;
    Ok(())
}

fn pin_control_rule(response: &Value, pin: &str, severity: Option<&str>) -> FlowResult<()> {
    let matching: Vec<_> = response["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|d| {
            d["subject_id"] == pin
                && matches!(
                    d["code"].as_str(),
                    Some("UML_PIN_CONTROL_PINS" | "MDE_PIN_CONTROL_PINS_UNVERIFIED")
                )
        })
        .collect();
    if let Some(severity) = severity {
        assert_eq!(matching.len(), 1, "{response}");
        assert_eq!(matching[0]["severity"], severity, "{response}");
    } else {
        assert!(matching.is_empty(), "{response}");
    }
    Ok(())
}
