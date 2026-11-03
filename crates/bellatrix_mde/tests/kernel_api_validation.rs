use bellatrix_mde::validate_uml_model;
use serde_json::{Value, json};

fn request() -> Value {
    json!({
        "api_version": 1,
        "model": {
            "schema_version": 1,
            "id": "model-document",
            "name": "Model document",
            "root_model": "root",
            "elements": [
                {"id":"root", "kind":"uml.model", "name":"Root"},
                {"id":"class", "kind":"uml.class", "name":"Example"}
            ],
            "relations": [{"id":"owns-class", "kind":"owns", "from":"root", "to":"class"}]
        }
    })
}

#[test]
fn validation_is_repeatable_aba_and_does_not_mutate_input() -> Result<(), Box<dyn std::error::Error>> {
    let first = request().to_string();
    let original = first.clone();
    let expected = validate_uml_model(&first)?;
    let response: Value = serde_json::from_str(&expected)?;
    assert_eq!(response["api_version"], 1);
    assert_eq!(response["coverage"], "implemented_rules_only");
    assert_eq!(response["has_errors"], false, "{expected}");

    let mut second = request();
    second["model"]["relations"][0]["to"] = json!("missing");
    let other = validate_uml_model(&second.to_string())?;
    assert_eq!(serde_json::from_str::<Value>(&other)?["has_errors"], true);
    assert_ne!(expected, other);
    for _ in 0..8 {
        assert_eq!(validate_uml_model(&first)?, expected);
        assert_eq!(validate_uml_model(&second.to_string())?, other);
    }
    assert_eq!(first, original);
    Ok(())
}

#[test]
fn semantic_errors_use_the_existing_uml_rules() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = request();
    input["model"]["elements"][1]["properties"] = json!({"visibility":"not-a-visibility"});
    let response: Value = serde_json::from_str(&validate_uml_model(&input.to_string())?)?;
    assert_eq!(response["has_errors"], true, "{response}");
    let diagnostics = response["diagnostics"].as_array().ok_or("missing diagnostics")?;
    assert!(diagnostics.iter().any(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.starts_with("UML_"))), "{response}");
    Ok(())
}

#[test]
fn invalid_contracts_return_versioned_deterministic_errors() -> Result<(), Box<dyn std::error::Error>> {
    let mut unsupported = request();
    unsupported["api_version"] = json!(2);
    let mut malformed_model = request();
    malformed_model["model"] = Value::Null;
    let mut unsupported_model = request();
    unsupported_model["model"]["schema_version"] = json!(2);
    let mut unknown_field = request();
    unknown_field["unexpected"] = json!(true);
    for (input, code) in [
        ("{".to_owned(), "INVALID_REQUEST"),
        ("{}".to_owned(), "INVALID_REQUEST"),
        (unsupported.to_string(), "UNSUPPORTED_API_VERSION"),
        (malformed_model.to_string(), "INVALID_MODEL"),
        (unsupported_model.to_string(), "INVALID_MODEL"),
        (unknown_field.to_string(), "INVALID_REQUEST"),
    ] {
        let error = validate_uml_model(&input).expect_err("invalid request must fail");
        let response: Value = serde_json::from_str(&error)?;
        assert_eq!(response["api_version"], 1);
        assert_eq!(response["error"]["code"], code);
        validate_uml_model(&request().to_string())?;
        assert_eq!(validate_uml_model(&input).expect_err("repeat invalid input"), error);
    }
    Ok(())
}
