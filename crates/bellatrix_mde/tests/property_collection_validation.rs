use bellatrix_mde::{
    Format, Project, StructuralSeverity, decode, encode,
    uml::{
        property_contracts::{self, PropertyOutcome},
        references::ReferenceIndex,
        root::Model,
        schema,
    },
};
use serde_json::{Value, json};

fn document() -> Project {
    decode(&serde_json::to_vec(&json!({
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
    })).expect("collection document"), Format::Json).expect("load collection document")
}

fn check(project: &Project, property: &str, value: &Value) -> PropertyOutcome {
    let references = ReferenceIndex::new(Model::from_project(project));
    let operation = references.element("operation").expect("Operation identity");
    let descriptor = schema::uml_property("Operation", property)
        .expect("resolved property").expect("Operation property");
    property_contracts::check_value(&references, operation, descriptor, value)
}

#[test]
fn unresolved_collection_members_do_not_hide_local_violations_in_either_order() {
    let project = document();
    let external = json!({"href":"other.uml#Type"});
    for (property, invalid, expected) in [
        ("raisedException", json!({"$ref":"missing"}), "unknown Element reference"),
        ("raisedException", json!({"$ref":"comment"}), "must conform to UML Type"),
        ("raisedException", json!(false), "reference["),
        ("raisedException", Value::Null, "cannot be null"),
        ("ownedParameter", json!({"$ref":"parameter"}), "must be owned by operation"),
    ] {
        for values in [
            json!([external.clone(), invalid.clone()]),
            json!([invalid, external.clone()]),
        ] {
            let outcome = check(&project, property, &values);
            assert!(matches!(&outcome, PropertyOutcome::Violation(message)
                if message.contains(expected)), "{property}: {values}: {outcome:?}");
        }
    }
    for local in ["class", "custom"] {
        for values in [
            json!([external.clone(), {"$ref":local}, {"$ref":local}]),
            json!([{"$ref":local}, {"$ref":local}, external.clone()]),
        ] {
            let outcome = check(&project, "raisedException", &values);
            assert!(matches!(&outcome, PropertyOutcome::Violation(message)
                if message.contains("duplicate value")), "{values}: {outcome:?}");
        }
    }
}

#[test]
fn collection_controls_remain_satisfied_or_unverified_without_proven_violations() {
    let project = document();
    for values in [json!([]), json!([{"$ref":"class"}])] {
        assert_eq!(check(&project, "raisedException", &values), PropertyOutcome::Satisfied);
    }
    for values in [
        json!([{"href":"other.uml#Type"}]),
        json!([{"href":"other.uml#Type"}, {"$ref":"class"}]),
        json!([{"$ref":"class"}, {"href":"other.uml#Type"}]),
        json!([{"$ref":"custom"}]),
    ] {
        assert!(matches!(check(&project, "raisedException", &values),
            PropertyOutcome::Unverified(_)), "{values}");
    }
}

#[test]
fn mixed_collection_violation_survives_validation_and_round_trip() {
    for values in [
        json!([{"href":"other.uml#Type"}, {"$ref":"missing"}]),
        json!([{"$ref":"missing"}, {"href":"other.uml#Type"}]),
    ] {
        let mut project = document();
        project.elements.iter_mut().find(|element| element.id == "operation")
            .expect("Operation").properties.insert("raisedException".into(), values);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format).expect("save mixed collection"), format)
                .expect("reopen mixed collection");
            assert_eq!(reopened, project);
            assert!(Model::from_project(&reopened).validate().iter().any(|diagnostic|
                diagnostic.subject_id == "operation"
                    && diagnostic.code == "UML_PROPERTY_CONTRACT"
                    && diagnostic.severity == StructuralSeverity::Error
                    && diagnostic.message.contains("unknown Element reference missing")));
        }
    }
}

#[test]
fn external_reference_envelopes_are_validated_before_deferring_resolution() {
    let project = document();
    for href in [
        "other.uml#Type", "../models/types.uml#Type", "#Type", "types.uml",
        "http://www.omg.org/spec/UML/20131001/PrimitiveTypes.xmi#Boolean",
        "urn:example:types#Type",
    ] {
        let external = json!({"href": href});
        let references = ReferenceIndex::new(Model::from_project(&project));
        let parameter = references.element("parameter").expect("Parameter");
        let descriptor = schema::uml_property("Parameter", "type").expect("schema").expect("type");
        assert!(matches!(property_contracts::check_value(&references, parameter, descriptor, &external),
            PropertyOutcome::Unverified(_)), "{external}");
        for value in [json!([external.clone()]), json!([external.clone(), {"$ref":"class"}])] {
            assert!(matches!(check(&project, "raisedException", &value), PropertyOutcome::Unverified(_)), "{value}");
        }
        assert!(matches!(check(&project, "raisedException", &json!([external.clone(), external])),
            PropertyOutcome::Violation(message) if message.contains("duplicate external reference")));
    }
    for invalid in [
        json!({"href":false}), json!({"href":null}), json!({"href":42}),
        json!({"href":[]}), json!({"href":{}}), json!({"href":""}),
        json!({"href":"other.uml#Type", "$ref":"class"}),
        json!({"href":"other.uml#Type", "extra":true}),
    ] {
        for value in [
            json!([invalid.clone()]),
            json!([{"href":"other.uml#Valid"}, invalid.clone()]),
            json!([invalid.clone(), {"href":"other.uml#Valid"}]),
            json!([{"$ref":"class"}, invalid.clone()]),
        ] {
            assert!(matches!(check(&project, "raisedException", &value),
                PropertyOutcome::Violation(message) if message.contains("exactly one nonempty href")), "{value}");
        }
        let references = ReferenceIndex::new(Model::from_project(&project));
        let parameter = references.element("parameter").expect("Parameter");
        let descriptor = schema::uml_property("Parameter", "type").expect("schema").expect("type");
        assert!(matches!(property_contracts::check_value(&references, parameter, descriptor, &invalid),
            PropertyOutcome::Violation(_)), "{invalid}");
    }
}

#[test]
fn external_reference_shape_survives_round_trip_and_public_validation_api() {
    for (values, expected_code, expected_severity) in [
        (json!([{"href":"other.uml#Type"}]), "MDE_PROPERTY_CONTRACT_UNVERIFIED", "warning"),
        (json!([{"href":false}]), "UML_PROPERTY_CONTRACT", "error"),
        (json!([{"href":null}]), "UML_PROPERTY_CONTRACT", "error"),
        (json!([{"href":12}]), "UML_PROPERTY_CONTRACT", "error"),
        (json!([{"href":"other.uml#Type", "$ref":"class"}]), "UML_PROPERTY_CONTRACT", "error"),
        (json!([{"href":"other.uml#Type"}, {"$ref":"missing"}]), "UML_PROPERTY_CONTRACT", "error"),
        (json!([{"$ref":"missing"}, {"href":"other.uml#Type"}]), "UML_PROPERTY_CONTRACT", "error"),
    ] {
        let mut project = document();
        project.elements.iter_mut().find(|element| element.id == "operation")
            .expect("Operation").properties.insert("raisedException".into(), values);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format).expect("save draft"), format).expect("reload draft");
            assert_eq!(reopened, project);
            let model: Value = serde_json::from_slice(&encode(&reopened, Format::Json).expect("JSON")).expect("model JSON");
            let request = json!({"api_version":1, "model":model}).to_string();
            let result = bellatrix_mde::validate_uml_model(&request).expect("validation API");
            assert_eq!(bellatrix_mde::validate_uml_model(&request).expect("repeat"), result);
            let response: Value = serde_json::from_str(&result).expect("API JSON");
            assert_eq!(response["has_errors"], expected_severity == "error", "{result}");
            assert!(response["diagnostics"].as_array().expect("diagnostics").iter().any(|diagnostic|
                diagnostic["subject_id"] == "operation" && diagnostic["code"] == expected_code
                    && diagnostic["severity"] == expected_severity
                    && diagnostic["message"].as_str().is_some_and(|message| message.contains("Operation-raisedException"))), "{result}");
        }
    }
}

#[test]
fn direct_type_signature_and_bound_queries_distinguish_invalid_external_envelopes() {
    use bellatrix_mde::uml::{
        classifiers::QueryError, feature_compatibility, namespace_imports,
        value_specifications::{self, EvaluationError},
    };

    for (value, valid) in [
        (json!({"href":"../types.uml#Type"}), true),
        (json!({"href":false}), false), (json!({"href":null}), false),
        (json!({"href":5}), false), (json!({"href":[]}), false),
        (json!({"href":{}}), false), (json!({"href":""}), false),
        (json!({"href":"types.uml#Type", "$ref":"class"}), false),
    ] {
        let mut project = document();
        project.elements.iter_mut().find(|element| element.id == "other").expect("other").name = "accept".into();
        project.elements.push(serde_json::from_value(json!({
            "id":"left-parameter", "kind":"uml.parameter", "name":"input",
            "properties":{"type":value.clone()}
        })).expect("parameter"));
        project.relations.push(serde_json::from_value(json!({
            "id":"owns-left-parameter", "kind":"owns", "from":"operation", "to":"left-parameter"
        })).expect("ownership"));
        project.elements.iter_mut().find(|element| element.id == "parameter").expect("parameter")
            .properties.insert("type".into(), json!({"$ref":"class"}));
        let references = ReferenceIndex::new(Model::from_project(&project));
        let parameter = references.element("left-parameter").expect("parameter");
        for result in [
            feature_compatibility::resolve_feature_type(&references, parameter).map(|_| true),
            namespace_imports::is_distinguishable_from(&references, "class", "operation", "other"),
        ] {
            if valid {
                assert!(matches!(result, Err(QueryError::Unsupported { .. })), "{value}: {result:?}");
            } else {
                assert!(matches!(result, Err(QueryError::Invalid { .. })), "{value}: {result:?}");
            }
        }
        for role in ["lowerValue", "upperValue"] {
            let mut bounds = document();
            bounds.elements.iter_mut().find(|element| element.id == "parameter").expect("parameter")
                .properties.insert(role.into(), value.clone());
            let references = ReferenceIndex::new(Model::from_project(&bounds));
            let parameter = references.element("parameter").expect("parameter");
            let result = if role == "lowerValue" {
                value_specifications::lower_bound(&references, parameter).map(|_| ())
            } else {
                value_specifications::upper_bound(&references, parameter).map(|_| ())
            };
            if valid {
                assert!(matches!(result, Err(EvaluationError::Unsupported { .. })), "{role}: {value}: {result:?}");
            } else {
                assert!(matches!(result, Err(EvaluationError::Invalid { .. })), "{role}: {value}: {result:?}");
            }
        }
    }
}
