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
