use bellatrix_mde::{reachable_graph_nodes, shortest_graph_path};
use serde_json::{Value, json};

fn model() -> Value {
    json!({
        "schema_version": 1, "id": "document", "name": "Graph", "root_model": "root",
        "elements": [
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"a", "kind":"uml.class", "name":"A"},
            {"id":"b", "kind":"uml.class", "name":"B"},
            {"id":"c", "kind":"uml.class", "name":"C"}
        ],
        "relations": [
            {"id":"owns-a", "kind":"owns", "from":"root", "to":"a"},
            {"id":"owns-b", "kind":"owns", "from":"root", "to":"b"},
            {"id":"owns-c", "kind":"owns", "from":"root", "to":"c"},
            {"id":"ac", "kind":"depends_on", "from":"a", "to":"c"},
            {"id":"ab", "kind":"depends_on", "from":"a", "to":"b"},
            {"id":"bc", "kind":"depends_on", "from":"b", "to":"c"},
            {"id":"ca", "kind":"depends_on", "from":"c", "to":"a"}
        ]
    })
}

#[test]
fn reachability_is_sorted_filtered_and_repeatable_aba() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = json!({"api_version":1,"model":model(),"roots":["a","a"],"relation_kinds":["depends_on"]});
    let input = request.to_string();
    let expected = reachable_graph_nodes(&input)?;
    assert_eq!(serde_json::from_str::<Value>(&expected)?, json!({"api_version":1,"node_ids":["a","b","c"]}));
    request["relation_kinds"] = json!([]);
    let other = reachable_graph_nodes(&request.to_string())?;
    assert_eq!(serde_json::from_str::<Value>(&other)?["node_ids"], json!(["a"]));
    for _ in 0..8 {
        assert_eq!(reachable_graph_nodes(&input)?, expected);
        assert_eq!(reachable_graph_nodes(&request.to_string())?, other);
    }
    request["roots"] = json!([]);
    assert_eq!(serde_json::from_str::<Value>(&reachable_graph_nodes(&request.to_string())?)?["node_ids"], json!([]));
    Ok(())
}

#[test]
fn shortest_path_preserves_order_and_distinguishes_empty_from_absent() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = json!({"api_version":1,"model":model(),"from":"root","to":"c"});
    let input = request.to_string();
    let expected = shortest_graph_path(&input)?;
    assert_eq!(serde_json::from_str::<Value>(&expected)?["relation_ids"], json!(["owns-c"]));
    request["relation_kind"] = json!("depends_on");
    let other = shortest_graph_path(&request.to_string())?;
    assert_eq!(serde_json::from_str::<Value>(&other)?["relation_ids"], Value::Null);
    for _ in 0..8 {
        assert_eq!(shortest_graph_path(&input)?, expected);
        assert_eq!(shortest_graph_path(&request.to_string())?, other);
    }
    request["from"] = json!("b"); request["to"] = json!("a");
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string())?)?["relation_ids"], json!(["bc","ca"]));
    request["to"] = json!("b");
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string())?)?["relation_ids"], json!([]));
    Ok(())
}

#[test]
fn queries_reject_unknown_nodes_and_broken_graphs() -> Result<(), Box<dyn std::error::Error>> {
    let reach = json!({"api_version":1,"model":model(),"roots":["missing"],"relation_kinds":["owns"]});
    let path = json!({"api_version":1,"model":model(),"from":"missing","to":"missing"});
    for (function, request) in [
        (reachable_graph_nodes as fn(&str) -> Result<String,String>, reach),
        (shortest_graph_path as fn(&str) -> Result<String,String>, path),
    ] {
        let mut cases = vec![(request.clone(), "UNKNOWN_NODE")];
        let mut invalid = request.clone(); invalid["model"]["relations"][0]["to"] = json!("missing");
        cases.push((invalid, "GRAPH_INTEGRITY"));
        let mut invalid = request.clone(); invalid["api_version"] = json!(2);
        cases.push((invalid, "UNSUPPORTED_API_VERSION"));
        let mut invalid = request.clone(); invalid["model"] = Value::Null;
        cases.push((invalid, "INVALID_MODEL"));
        let mut invalid = request.clone(); invalid["extra"] = json!(true);
        cases.push((invalid, "INVALID_REQUEST"));
        for (input, code) in cases {
            let error = function(&input.to_string()).expect_err("invalid query");
            assert_eq!(serde_json::from_str::<Value>(&error)?["error"]["code"], code);
            assert_eq!(function(&input.to_string()).expect_err("repeat query"), error);
        }
        assert_eq!(serde_json::from_str::<Value>(&function("{").expect_err("malformed JSON"))?["error"]["code"], "INVALID_REQUEST");
    }
    Ok(())
}

#[test]
fn equal_length_paths_are_independent_of_storage_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = json!({"api_version":1,"model":model(),"from":"root","to":"c"});
    let relations = request["model"]["relations"].as_array_mut().ok_or("missing relations")?;
    relations.retain(|relation| relation["id"] != "owns-c");
    let expected = shortest_graph_path(&request.to_string())?;
    assert_eq!(serde_json::from_str::<Value>(&expected)?["relation_ids"], json!(["owns-a","ac"]));
    request["model"]["relations"].as_array_mut().ok_or("missing relations")?.reverse();
    request["model"]["elements"].as_array_mut().ok_or("missing elements")?.reverse();
    assert_eq!(shortest_graph_path(&request.to_string())?, expected);
    Ok(())
}

#[test]
fn presentation_errors_do_not_block_topology_but_references_still_do() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = json!({"api_version":1,"model":model(),"from":"a","to":"c"});
    request["model"]["diagrams"] = json!([{
        "id":"diagram", "name":"Diagram", "uml_di_type":"UMLClassDiagram", "width":-1,
        "views":[{"id":"view", "subject":"a", "geometry":{"kind":"rect","x":0,"y":0,"width":-10,"height":10},"style":{"opacity":2}}]
    }]);
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string())?)?["relation_ids"], json!(["ac"]));
    let validation = json!({"api_version":1,"model":request["model"]});
    assert_eq!(serde_json::from_str::<Value>(&bellatrix_mde::validate_uml_model(&validation.to_string())?)?["has_errors"], true);
    request["model"]["diagrams"][0]["views"][0]["subject"] = json!("missing");
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string()).expect_err("broken view reference"))?["error"]["code"], "GRAPH_INTEGRITY");
    request["model"]["diagrams"][0]["views"][0]["subject"] = json!("a");
    request["model"]["diagrams"][0]["views"][0]["id"] = json!("a");
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string()).expect_err("duplicate identity"))?["error"]["code"], "GRAPH_INTEGRITY");
    Ok(())
}

#[test]
fn null_kind_and_parallel_edge_ties_have_stable_meaning() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = json!({"api_version":1,"model":model(),"from":"a","to":"c"});
    let expected = shortest_graph_path(&request.to_string())?;
    request["relation_kind"] = Value::Null;
    assert_eq!(shortest_graph_path(&request.to_string())?, expected);
    request["relation_kind"] = json!("not-a-kind");
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string())?)?["relation_ids"], Value::Null);
    request["relation_kind"] = json!("depends_on");
    request["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(json!({"id":"aa-parallel","kind":"depends_on","from":"a","to":"c"}));
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string())?)?["relation_ids"], json!(["aa-parallel"]));
    request["model"]["relations"].as_array_mut().ok_or("missing relations")?.reverse();
    assert_eq!(serde_json::from_str::<Value>(&shortest_graph_path(&request.to_string())?)?["relation_ids"], json!(["aa-parallel"]));
    Ok(())
}

#[test]
fn decoding_errors_do_not_expose_internal_rust_names() -> Result<(), Box<dyn std::error::Error>> {
    for (function, input) in [
        (shortest_graph_path as fn(&str) -> Result<String,String>, "null".to_owned()),
        (reachable_graph_nodes as fn(&str) -> Result<String,String>, "null".to_owned()),
        (bellatrix_mde::validate_uml_model as fn(&str) -> Result<String,String>, "null".to_owned()),
        (shortest_graph_path as fn(&str) -> Result<String,String>, json!({"api_version":1,"model":[],"from":"a","to":"b"}).to_string()),
    ] {
        let error = function(&input).expect_err("invalid input");
        let message = serde_json::from_str::<Value>(&error)?["error"]["message"].as_str().ok_or("message")?.to_owned();
        assert!(!message.contains("struct") && !message.contains("enum") && !message.contains("Request") && !message.contains("Project"), "{message}");
    }
    Ok(())
}
