use bellatrix_mde::validate_uml_model;
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn request(kind: &str, aggregation: Option<Value>) -> Value {
    let mut properties = json!({});
    if let Some(aggregation) = aggregation {
        properties["aggregation"] = aggregation;
    }
    json!({"api_version": 1, "model": {
        "id": "document", "name": "Ports", "root_model": "root",
        "elements": [
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"owner", "kind":"uml.class", "name":"Owner"},
            {"id":"property", "kind":kind, "name":"Endpoint", "properties":properties}
        ],
        "relations": [
            {"id":"owns-owner", "kind":"owns", "from":"root", "to":"owner"},
            {"id":"owns-property", "kind":"owns", "from":"owner", "to":"property"}
        ]
    }})
}

fn validate(input: &Value) -> Result<Value> {
    Ok(serde_json::from_str(&validate_uml_model(
        &input.to_string(),
    )?)?)
}

fn diagnostics(response: &Value) -> Result<&Vec<Value>> {
    response["diagnostics"]
        .as_array()
        .ok_or_else(|| "diagnostics missing".into())
}

#[test]
fn port_aggregation_checks_inherited_default_and_explicit_values() -> Result {
    for aggregation in [None, Some(json!("none")), Some(json!("shared"))] {
        let input = request("uml.port", aggregation);
        let before = input.clone();
        let response = validate(&input)?;
        assert_eq!(response["has_errors"], true, "{response}");
        let issues = diagnostics(&response)?
            .iter()
            .filter(|issue| issue["code"] == "UML_PORT_AGGREGATION")
            .collect::<Vec<_>>();
        assert_eq!(issues.len(), 1, "{response}");
        assert_eq!(
            issues.first().ok_or("Port diagnostic missing")?["subject_id"],
            "property"
        );
        assert_eq!(
            issues.first().ok_or("Port diagnostic missing")?["severity"],
            "error"
        );
        assert!(
            issues.first().ok_or("Port diagnostic missing")?["message"]
                .as_str()
                .ok_or("message missing")?
                .starts_with("Port-port_aggregation:")
        );
        assert_eq!(input, before);
    }
    let valid = validate(&request("uml.port", Some(json!("composite"))))?;
    assert_eq!(valid["has_errors"], false, "{valid}");
    assert!(
        !diagnostics(&valid)?
            .iter()
            .any(|issue| issue["code"] == "UML_PORT_AGGREGATION")
    );
    // Ordinary Property retains the normative none default; Port adds this rule.
    let attribute = validate(&request("uml.attribute", None))?;
    assert_eq!(attribute["has_errors"], false, "{attribute}");
    Ok(())
}

#[test]
fn malformed_port_aggregation_is_not_replaced_by_a_default() -> Result {
    for value in [
        Value::Null,
        json!(false),
        json!(1),
        json!("invalid"),
        json!({"href":"external#aggregation"}),
    ] {
        let response = validate(&request("uml.port", Some(value)))?;
        assert_eq!(response["has_errors"], true, "{response}");
        let issues = diagnostics(&response)?;
        assert!(
            issues
                .iter()
                .any(|issue| issue["code"] == "UML_PROPERTY_VALUE"
                    && issue["subject_id"] == "property"
                    && issue["message"]
                        .as_str()
                        .is_some_and(|message| message.contains("aggregation"))),
            "{response}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue["code"] == "UML_PORT_AGGREGATION"),
            "{response}"
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue["code"] == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
                    && issue["message"]
                        .as_str()
                        .is_some_and(|message| message.starts_with("Port-port_aggregation:"))),
            "{response}"
        );
    }
    Ok(())
}

#[test]
fn port_aggregation_validation_is_repeatable_aba_and_parallel() -> Result {
    let valid = request("uml.port", Some(json!("composite"))).to_string();
    let invalid = request("uml.port", None).to_string();
    let expected_valid = validate_uml_model(&valid)?;
    let expected_invalid = validate_uml_model(&invalid)?;
    for _ in 0..4 {
        assert_eq!(validate_uml_model(&valid)?, expected_valid);
        assert_eq!(validate_uml_model(&invalid)?, expected_invalid);
        assert_eq!(validate_uml_model(&valid)?, expected_valid);
    }
    std::thread::scope(|scope| -> Result {
        let handles = (0..4)
            .map(|_| scope.spawn(|| (validate_uml_model(&valid), validate_uml_model(&invalid))))
            .collect::<Vec<_>>();
        for handle in handles {
            let (actual_valid, actual_invalid) =
                handle.join().map_err(|_| "validation worker panicked")?;
            assert_eq!(actual_valid?, expected_valid);
            assert_eq!(actual_invalid?, expected_invalid);
        }
        Ok(())
    })?;
    Ok(())
}
