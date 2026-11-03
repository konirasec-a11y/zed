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

#[test]
fn kernel_contract_flow_validates_rooted_and_rootless_drafts_without_views()
-> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::{Format, decode, encode};

    // KB-10: omitted root designation cannot bypass namespace, relationship
    // projection or Property subsetting contracts on the same authored graph.
    let model = json!({
        "schema_version":1, "id":"contracts", "name":"Contracts",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"owner","kind":"uml.class","name":"Owner"},
            {"id":"a","kind":"uml.class","name":"A"},
            {"id":"b","kind":"uml.class","name":"B"},
            {"id":"general","kind":"uml.attribute","name":"items",
             "properties":{"type":{"$ref":"a"}}},
            {"id":"specific","kind":"uml.attribute","name":"selected",
             "properties":{"type":{"$ref":"a"},"subsettedProperty":[{"$ref":"general"}]}}
        ],
        "relations":[
            {"id":"owns-owner","kind":"owns","from":"root","to":"owner"},
            {"id":"owns-a","kind":"owns","from":"root","to":"a"},
            {"id":"owns-b","kind":"owns","from":"root","to":"b"},
            {"id":"owns-general","kind":"owns","from":"owner","to":"general"},
            {"id":"owns-specific","kind":"owns","from":"owner","to":"specific"},
            {"id":"owns-dependency","kind":"owns","from":"root","to":"dependency"},
            {"id":"dependency","kind":"uml.dependency","from":"a","to":"b",
             "properties":{"client":[{"$ref":"a"}],"supplier":[{"$ref":"b"}]}}
        ]
    });
    let validate = |model: &Value| -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&validate_uml_model(
            &json!({"api_version":1,"model":model}).to_string(),
        ).map_err(std::io::Error::other)?)?)
    };
    let has = |response: &Value, code: &str, subject: &str, severity: &str| {
        response["diagnostics"].as_array().is_some_and(|diagnostics|
            diagnostics.iter().any(|diagnostic| diagnostic["code"] == code
                && diagnostic["subject_id"] == subject && diagnostic["severity"] == severity))
    };
    let mut rootless_responses = None;
    for rooted in [false, true] {
        let mut authored = model.clone();
        if rooted { authored["root_model"] = json!("root"); }
        let valid = validate(&authored)?;
        assert_eq!(valid["has_errors"], false, "{valid}");
        assert_eq!(valid["diagnostics"], json!([]), "{valid}");

        let mut draft = authored.clone();
        draft["elements"][3]["name"] = json!("A");
        draft["elements"][5]["properties"]["type"] = json!({"$ref":"b"});
        draft["relations"][6]["properties"]["client"] = json!([{"$ref":"b"}]);
        let invalid = validate(&draft)?;
        for (code, subject) in [
            ("UML_NAMESPACE_MEMBERS_DISTINGUISHABLE", "a"),
            ("UML_PROPERTY_SUBSETTING", "specific"),
            ("UML_RELATIONSHIP_ENDPOINTS", "dependency"),
        ] {
            assert!(has(&invalid, code, subject, "error"), "rooted={rooted}: {invalid}");
        }
        assert_eq!(invalid["coverage"], "implemented_rules_only");
        let project = decode(&serde_json::to_vec(&draft)?, Format::Json)?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format)?, format)?;
            assert_eq!(project, reopened);
            assert_eq!(validate(&serde_json::to_value(&reopened)?)?, invalid);
        }

        draft["elements"][3]["name"] = json!("B");
        draft["elements"][5]["properties"]["type"] = json!({"$ref":"a"});
        draft["relations"][6]["properties"]["client"] = json!([{"$ref":"a"}]);
        assert_eq!(validate(&draft)?, valid);
        draft["elements"][5]["properties"]["subsettedProperty"] =
            json!([{"href":"external.xmi#Property"}]);
        let unresolved = validate(&draft)?;
        assert!(has(&unresolved, "UML_PROPERTY_SUBSETTING_UNSUPPORTED", "specific", "warning"),
            "{unresolved}");
        assert_eq!(unresolved["has_errors"], false, "{unresolved}");
        let mut malformed_scalar = authored.clone();
        malformed_scalar["elements"][1]["properties"] = json!({"isAbstract":"yes"});
        let scalar_response = validate(&malformed_scalar)?;
        assert!(has(&scalar_response, "UML_PROPERTY_VALUE", "owner", "error"),
            "rooted={rooted}: {scalar_response}");
        let responses = vec![valid.clone(), invalid, unresolved, scalar_response];
        for response in &responses {
            let diagnostics = response["diagnostics"].as_array().ok_or("missing diagnostics")?;
            let unique = diagnostics.iter().map(Value::to_string)
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(unique.len(), diagnostics.len(), "duplicate diagnostics: {response}");
        }
        if let Some(expected) = &rootless_responses {
            assert_eq!(&responses, expected, "root designation changed semantic diagnostics");
        } else {
            rootless_responses = Some(responses);
        }
        // A previous invalid or unresolved call cannot contaminate the repair.
        assert_eq!(validate(&authored)?, valid);
    }
    let unowned = json!({
        "schema_version":1, "id":"draft", "name":"Incomplete document",
        "elements":[{"id":"class","kind":"uml.class","name":"Unowned"}]
    });
    let response = validate(&unowned)?;
    assert_eq!(response["has_errors"], false, "{response}");
    assert!(!has(&response, "UML_ELEMENT_HAS_OWNER", "class", "error"));
    Ok(())
}
