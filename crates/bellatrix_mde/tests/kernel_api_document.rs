use bellatrix_mde::{Format, Graph, decode, decode_model_document, encode, validate_graph_model, validate_uml_model};
use serde_json::{Value, json};

fn model_json() -> Value {
    json!({"schema_version":1,"id":"document","name":"Example","root_model":"root",
        "elements":[{"id":"root","kind":"uml.model","name":"Root"},{"id":"class","kind":"uml.class","name":"Class"}],
        "relations":[{"id":"owns-class","kind":"owns","from":"root","to":"class"}]
    })
}

#[test]
fn codecs_match_legacy_and_are_repeatable_aba() -> Result<(), Box<dyn std::error::Error>> {
    let input = serde_json::to_vec(&model_json())?;
    let model = decode(&input, Format::Json)?;
    let protobuf = encode(&model, Format::Protobuf)?;
    let expected = decode_model_document(&input, "json")?;
    assert_eq!(decode_model_document(&protobuf, "protobuf")?, expected);
    assert_eq!(serde_json::from_str::<Value>(&expected)?["model"], serde_json::to_value(&model)?);
    assert_eq!(serde_json::from_str::<Value>(&validate_graph_model(&expected)?)?["has_errors"], false);
    let other_input = br#"{"id":"other","name":"Other"}"#;
    let other = decode_model_document(other_input, "json")?;
    assert_ne!(other, expected);
    for _ in 0..8 {
        assert_eq!(decode_model_document(&input, "json")?, expected);
        assert_eq!(decode_model_document(other_input, "json")?, other);
        assert_eq!(decode_model_document(&protobuf, "protobuf")?, expected);
    }
    assert_eq!(serde_json::to_vec(&model_json())?, input);
    Ok(())
}

#[test]
fn graph_validation_preserves_legacy_scope_and_errors() -> Result<(), Box<dyn std::error::Error>> {
    let mut semantic_invalid = model_json();
    semantic_invalid["elements"][1]["properties"] = json!({"visibility":"invalid"});
    let mut missing_endpoint = model_json();
    missing_endpoint["relations"][0]["to"] = json!("missing");
    let mut invalid_presentation = model_json();
    invalid_presentation["diagrams"] = json!([{"id":"diagram","name":"Diagram","uml_di_type":"UMLClassDiagram","width":-1}]);
    let mut duplicate = model_json();
    duplicate["elements"][1]["id"] = json!("root");
    for model in [model_json(), semantic_invalid.clone(), missing_endpoint, invalid_presentation, duplicate] {
        let bytes = serde_json::to_vec(&model)?;
        let legacy = decode(&bytes, Format::Json)?;
        let result = Graph::new(&legacy).validate();
        let envelope = decode_model_document(&bytes, "json")?;
        let response: Value = serde_json::from_str(&validate_graph_model(&envelope)?)?;
        assert_eq!(response["coverage"], "graph_integrity_and_presentation");
        assert_eq!(response["has_errors"], result.is_err());
        if let Err(error) = result {
            assert_eq!(response["diagnostics"][0]["message"], error.to_string());
        }
    }
    let envelope = json!({"api_version":1,"model":semantic_invalid}).to_string();
    let expected = validate_graph_model(&envelope)?;
    assert_eq!(serde_json::from_str::<Value>(&expected)?["has_errors"], false);
    assert_eq!(serde_json::from_str::<Value>(&validate_uml_model(&envelope)?)?["has_errors"], true);
    assert_eq!(validate_graph_model(&envelope)?, expected);
    for input in ["null", "{}", "{", r#"{"api_version":2,"model":{}}"#] {
        let error: Value = serde_json::from_str(&validate_graph_model(input).expect_err("invalid request"))?;
        assert_eq!(error["api_version"], 1);
        assert!(error["error"]["code"].is_string());
    }
    Ok(())
}

#[test]
fn populated_fixture_preserves_codec_data() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = include_bytes!("../fixtures/todo.json");
    let model = decode(bytes, Format::Json)?;
    let expected = decode_model_document(bytes, "json")?;
    let binary = encode(&model, Format::Protobuf)?;
    assert_eq!(decode_model_document(&binary, "protobuf")?, expected);
    let envelope: Value = serde_json::from_str(&expected)?;
    let restored = decode(&serde_json::to_vec(&envelope["model"])?, Format::Json)?;
    assert_eq!(restored, model);
    Ok(())
}
