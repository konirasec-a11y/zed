use bellatrix_mde::{decode_model_document, encode_model_document, validate_uml_model};
use serde_json::{Value, json};

#[test]
fn kernel_contract_flow_validates_rooted_and_rootless_drafts_without_views()
-> Result<(), Box<dyn std::error::Error>> {
    let decode = |bytes: &[u8], format: &str| -> Result<Value, Box<dyn std::error::Error>> {
        let mut response: Value = serde_json::from_str(&decode_model_document(bytes, format)?)?;
        Ok(response["model"].take())
    };
    let encode = |model: &Value, format: &str| {
        encode_model_document(&json!({"api_version": 1, "model": model}).to_string(), format)
    };

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
        let project = decode(&serde_json::to_vec(&draft)?, "json")?;
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&project, format)?, format)?;
            assert_eq!(project, reopened);
            assert_eq!(validate(&reopened)?, invalid);
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
