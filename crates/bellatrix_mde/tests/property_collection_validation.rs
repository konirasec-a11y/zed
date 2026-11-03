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
