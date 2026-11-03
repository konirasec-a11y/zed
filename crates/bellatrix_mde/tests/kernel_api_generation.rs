use bellatrix_mde::analyze_generation_dependencies;
use serde_json::{Value, json};

fn request() -> Value {
    json!({"api_version":1,"selected_ids":["client","supplier","independent"],"model":{
        "schema_version":1,"id":"document","name":"Example","root_model":"root",
        "elements":[{"id":"root","kind":"uml.model","name":"Root"},
          {"id":"client","kind":"uml.class","name":"Client"},
          {"id":"supplier","kind":"uml.class","name":"Supplier"},
          {"id":"independent","kind":"uml.class","name":"Independent"}],
        "relations":[{"id":"dependency","kind":"depends_on","from":"client","to":"supplier"}]
    }})
}

#[test]
fn dependencies_are_ordered_and_external_requirements_survive_selection() -> Result<(), Box<dyn std::error::Error>> {
    let first = request().to_string();
    let expected = analyze_generation_dependencies(&first)?;
    let response: Value = serde_json::from_str(&expected)?;
    let ids = response["units"].as_array().ok_or("units")?.iter().map(|unit| unit["id"].clone()).collect::<Vec<_>>();
    assert_eq!(ids, vec![json!("independent"),json!("supplier"),json!("client")]);
    let mut other = request(); other["selected_ids"] = json!(["client","client"]);
    let second = analyze_generation_dependencies(&other.to_string())?;
    let result: Value = serde_json::from_str(&second)?;
    assert_eq!(result["units"].as_array().ok_or("units")?.len(), 1);
    assert_eq!(result["units"][0]["requires"], json!([{"id":"supplier","kind":"uml.class","name":"Supplier","relation_kind":"depends_on"}]));
    for _ in 0..8 {
        assert_eq!(analyze_generation_dependencies(&first)?, expected);
        assert_eq!(analyze_generation_dependencies(&other.to_string())?, second);
    }
    other["selected_ids"] = json!([]);
    assert_eq!(serde_json::from_str::<Value>(&analyze_generation_dependencies(&other.to_string())?)?["units"], json!([]));
    Ok(())
}

#[test]
fn ordering_and_cycle_errors_are_stable_under_permutations() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = request();
    let expected = analyze_generation_dependencies(&input.to_string())?;
    input["selected_ids"].as_array_mut().ok_or("selection")?.reverse();
    input["model"]["elements"].as_array_mut().ok_or("elements")?.reverse();
    assert_eq!(analyze_generation_dependencies(&input.to_string())?, expected);
    input["model"]["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":"back","kind":"depends_on","from":"supplier","to":"client"}));
    let error = analyze_generation_dependencies(&input.to_string()).expect_err("dependency cycle");
    assert_eq!(serde_json::from_str::<Value>(&error)?["error"]["code"], "GENERATION_CYCLE");
    input["model"]["relations"].as_array_mut().ok_or("relations")?.reverse();
    assert_eq!(analyze_generation_dependencies(&input.to_string()).expect_err("cycle"), error);
    input["selected_ids"] = json!(["client"]);
    assert!(analyze_generation_dependencies(&input.to_string()).is_ok());
    Ok(())
}

#[test]
fn contract_errors_and_non_ordering_edges_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = request();
    input["model"]["relations"].as_array_mut().ok_or("relations")?.push(json!({"id":"back","kind":"uml.association","from":"supplier","to":"client"}));
    assert!(analyze_generation_dependencies(&input.to_string()).is_ok());
    for (field, value, code) in [
        ("api_version",json!(2),"UNSUPPORTED_API_VERSION"),
        ("model",Value::Null,"INVALID_MODEL"),
        ("selected_ids",json!(["missing"]),"UNKNOWN_SELECTION"),
        ("selected_ids",json!(["dependency"]),"UNKNOWN_SELECTION"),
        ("extra",json!(true),"INVALID_REQUEST"),
    ] {
        let mut bad = request(); bad[field] = value;
        assert_eq!(serde_json::from_str::<Value>(&analyze_generation_dependencies(&bad.to_string()).expect_err("invalid request"))?["error"]["code"], code);
    }
    let mut bad = request(); bad["model"]["relations"][0]["to"] = json!("missing");
    assert_eq!(serde_json::from_str::<Value>(&analyze_generation_dependencies(&bad.to_string()).expect_err("broken graph"))?["error"]["code"], "GRAPH_INTEGRITY");
    assert_eq!(serde_json::from_str::<Value>(&analyze_generation_dependencies("null").expect_err("invalid JSON envelope"))?["error"]["code"], "INVALID_REQUEST");
    Ok(())
}

#[test]
fn concurrent_calls_do_not_share_selection_or_results() -> Result<(), Box<dyn std::error::Error>> {
    let first = request().to_string();
    let expected = analyze_generation_dependencies(&first)?;
    std::thread::scope(|scope| {
        let handles = (0..4).map(|_| scope.spawn(|| analyze_generation_dependencies(&first))).collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().expect("analysis thread panicked").expect("analysis failed"), expected);
        }
    });
    assert_eq!(first, request().to_string());
    Ok(())
}

#[test]
fn dependencies_on_relationship_identities_are_rejected_not_omitted() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = request();
    input["selected_ids"] = json!(["client"]);
    input["model"]["relations"].as_array_mut().ok_or("relations")?.extend([
        json!({"id":"association","kind":"uml.association","from":"client","to":"supplier"}),
        json!({"id":"requires-association","kind":"depends_on","from":"client","to":"association"}),
    ]);
    let graph_envelope = json!({"api_version":1,"model":input["model"]});
    assert_eq!(serde_json::from_str::<Value>(&bellatrix_mde::validate_graph_model(&graph_envelope.to_string())?)?["has_errors"], false);
    let error = analyze_generation_dependencies(&input.to_string()).expect_err("relationship is not a supported generation target");
    let response: Value = serde_json::from_str(&error)?;
    assert_eq!(response["error"]["code"], "GENERATION_UNSUPPORTED_TARGET");
    assert!(response["error"]["message"].as_str().ok_or("message")?.contains("requires-association"));
    input["model"]["relations"].as_array_mut().ok_or("relations")?.reverse();
    assert_eq!(analyze_generation_dependencies(&input.to_string()).expect_err("same unsupported target"), error);
    input["selected_ids"] = json!(["supplier"]);
    assert!(analyze_generation_dependencies(&input.to_string()).is_ok(), "unselected client must not expand this analysis scope");
    Ok(())
}
