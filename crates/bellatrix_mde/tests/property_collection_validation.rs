use bellatrix_mde::{Format, Project, StructuralSeverity, decode, encode, uml::root::Model};
use serde_json::{Value, json};

fn document() -> Project {
    decode(
        &serde_json::to_vec(&json!({
            "id":"collection-contracts", "name":"Collection contracts", "root_model":"root",
            "elements":[
                {"id":"root", "kind":"uml.model", "name":"Root"},
                {"id":"class", "kind":"uml.class", "name":"Owner"},
                {"id":"operation", "kind":"uml.operation", "name":"accept"},
                {"id":"other", "kind":"uml.operation", "name":"other"},
                {"id":"parameter", "kind":"uml.parameter", "name":"input"},
                {"id":"comment", "kind":"uml.comment", "name":"Comment"},
                {"id":"custom", "kind":"custom.type", "name":"Custom"}
            ],
            "relations":[
                {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
                {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"},
                {"id":"owns-other", "kind":"owns", "from":"class", "to":"other"},
                {"id":"owns-parameter", "kind":"owns", "from":"other", "to":"parameter"},
                {"id":"owns-comment", "kind":"owns", "from":"root", "to":"comment"},
                {"id":"owns-custom", "kind":"owns", "from":"root", "to":"custom"}
            ]
        }))
        .expect("collection document"),
        Format::Json,
    )
    .expect("load collection document")
}

#[test]
fn mixed_collection_violation_survives_validation_and_round_trip() {
    for values in [
        json!([{"href":"other.uml#Type"}, {"$ref":"missing"}]),
        json!([{"$ref":"missing"}, {"href":"other.uml#Type"}]),
    ] {
        let mut project = document();
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "operation")
            .expect("Operation")
            .properties
            .insert("raisedException".into(), values);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(
                &encode(&project, format).expect("save mixed collection"),
                format,
            )
            .expect("reopen mixed collection");
            assert_eq!(reopened, project);
            assert!(
                Model::from_project(&reopened)
                    .validate()
                    .iter()
                    .any(|diagnostic| diagnostic.subject_id == "operation"
                        && diagnostic.code == "UML_PROPERTY_CONTRACT"
                        && diagnostic.severity == StructuralSeverity::Error
                        && diagnostic
                            .message
                            .contains("unknown Element reference missing"))
            );
        }
    }
}

#[test]
fn external_reference_shape_survives_round_trip_and_public_validation_api() {
    for (values, expected_code, expected_severity) in [
        (
            json!([{"href":"other.uml#Type"}]),
            "MDE_PROPERTY_CONTRACT_UNVERIFIED",
            "warning",
        ),
        (json!([{"href":false}]), "UML_PROPERTY_CONTRACT", "error"),
        (json!([{"href":null}]), "UML_PROPERTY_CONTRACT", "error"),
        (json!([{"href":12}]), "UML_PROPERTY_CONTRACT", "error"),
        (
            json!([{"href":"other.uml#Type", "$ref":"class"}]),
            "UML_PROPERTY_CONTRACT",
            "error",
        ),
        (
            json!([{"href":"other.uml#Type"}, {"$ref":"missing"}]),
            "UML_PROPERTY_CONTRACT",
            "error",
        ),
        (
            json!([{"$ref":"missing"}, {"href":"other.uml#Type"}]),
            "UML_PROPERTY_CONTRACT",
            "error",
        ),
    ] {
        let mut project = document();
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "operation")
            .expect("Operation")
            .properties
            .insert("raisedException".into(), values);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format).expect("save draft"), format)
                .expect("reload draft");
            assert_eq!(reopened, project);
            let model: Value =
                serde_json::from_slice(&encode(&reopened, Format::Json).expect("JSON"))
                    .expect("model JSON");
            let request = json!({"api_version":1, "model":model}).to_string();
            let result = bellatrix_mde::validate_uml_model(&request).expect("validation API");
            assert_eq!(
                bellatrix_mde::validate_uml_model(&request).expect("repeat"),
                result
            );
            let response: Value = serde_json::from_str(&result).expect("API JSON");
            assert_eq!(
                response["has_errors"],
                expected_severity == "error",
                "{result}"
            );
            assert!(
                response["diagnostics"]
                    .as_array()
                    .expect("diagnostics")
                    .iter()
                    .any(|diagnostic| diagnostic["subject_id"] == "operation"
                        && diagnostic["code"] == expected_code
                        && diagnostic["severity"] == expected_severity
                        && diagnostic["message"]
                            .as_str()
                            .is_some_and(|message| message.contains("Operation-raisedException"))),
                "{result}"
            );
        }
    }
}
