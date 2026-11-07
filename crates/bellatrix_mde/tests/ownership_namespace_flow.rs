use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, project_model_section,
};
use bellatrix_mde::contracts::protocol::{
    Command, inspector::ElementInspector, projection::ModelingProjection,
};
use serde_json::{Value, json};
use std::collections::HashSet;

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> Result<Value, String> {
    let decoded = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&decoded).map_err(|error| error.to_string())?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> Result<Vec<u8>, String> {
    encode_model_document(&envelope(model), format)
}

fn apply(model: &Value, command: &Command, revision: u64) -> Result<Value, String> {
    let document = prepare_model_document(&envelope(model))?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        revision,
        command.clone(),
    )?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")?;
    decode(&bytes, "json")
}

fn projection(model: &Value) -> ModelingProjection {
    let document = prepare_model_document(&envelope(model)).expect("prepare canonical document");
    project_model_section(&document.id, &document.name, &document.section_json)
        .expect("project the public document")
}

fn inspect_element(model: &Value, id: &str) -> Option<ElementInspector> {
    projection(model).inspect_element(id)
}

fn inspected_property(model: &Value, id: &str, path: &str) -> Value {
    inspect_element(model, id)
        .expect("inspect canonical identity")
        .properties
        .into_iter()
        .find(|property| property.path == path)
        .expect("public inspector property")
        .value
}

fn document() -> Value {
    decode(
        &serde_json::to_vec(&json!({
            "id":"project", "name":"Ownership", "root_model":"root",
            "elements":[
                {"id":"root","kind":"uml.model","name":"Root"},
                {"id":"class","kind":"uml.class","name":"Account",
                 "properties":{"collaborationUse":[{"$ref":"use"}]}},
                {"id":"amount","kind":"uml.attribute","name":"amount",
                 "properties":{"lowerValue":{"$ref":"lower"},"upperValue":{"$ref":"upper"}}},
                {"id":"lower","kind":"uml.literal_integer","name":"Lower",
                 "properties":{"value":0,"visibility":null}},
                {"id":"upper","kind":"uml.literal_unlimited_natural","name":"Upper",
                 "properties":{"value":"*","visibility":null}},
                {"id":"condition","kind":"uml.constraint","name":"Allowed",
                 "properties":{"specification":{"$ref":"specification"}}},
                {"id":"specification","kind":"uml.literal_boolean","name":"Expression",
                 "properties":{"value":true,"visibility":null}},
                {"id":"use","kind":"uml.collaboration_use","name":"Usage",
                 "properties":{"visibility":null}}
            ],
            "relations":[
                {"id":"owns-class","kind":"owns","from":"root","to":"class"},
                {"id":"owns-amount","kind":"owns","from":"class","to":"amount"},
                {"id":"owns-lower","kind":"owns","from":"amount","to":"lower"},
                {"id":"owns-upper","kind":"owns","from":"amount","to":"upper"},
                {"id":"owns-condition","kind":"owns","from":"class","to":"condition"},
                {"id":"owns-specification","kind":"owns","from":"condition","to":"specification"},
                {"id":"owns-use","kind":"owns","from":"class","to":"use"}
            ]
        }))
        .expect("document bytes"),
        "json",
    )
    .expect("import document")
}

fn external_property(project: &Value, id: &str, name: &str, value: Value) -> Value {
    let mut content = serde_json::to_value(project).expect("authored document");
    let element = content["elements"]
        .as_array_mut()
        .expect("elements")
        .iter_mut()
        .find(|element| element["id"] == id)
        .expect("element");
    element["properties"][name] = value;
    decode(
        &serde_json::to_vec(&content).expect("edited document"),
        "json",
    )
    .expect("reopen authored draft")
}

#[test]
fn canonical_composite_roles_flow_queries_edits_rejects_conflicts_and_reopens() {
    let project = document();
    let projected = projection(&project);
    assert_eq!(
        projected
            .hierarchy_rows
            .iter()
            .filter(|row| row.depth > 0)
            .map(|row| row.element_id.as_str())
            .collect::<HashSet<_>>(),
        HashSet::from([
            "class",
            "amount",
            "lower",
            "upper",
            "condition",
            "specification",
            "use"
        ])
    );
    let members = inspected_property(&project, "class", "uml.ownedMember");
    let members = members.as_array().expect("projected owned members");
    assert!(members.contains(&json!({"$ref":"amount"})));
    assert!(members.contains(&json!({"$ref":"condition"})));
    assert!(!members.contains(&json!({"$ref":"use"})));
    assert!(inspected_property(&project, "lower", "uml.namespace").is_null());
    assert_eq!(
        inspected_property(&project, "amount", "uml.qualifiedName"),
        json!("Root::Account::amount")
    );

    let edited = apply(
        &project,
        &Command::Rename {
            element_id: "class".into(),
            name: "Customer".into(),
        },
        1,
    )
    .expect("edit through the public command");
    let inspector =
        inspect_element(&edited, "amount").expect("inspect the same canonical Property");
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "uml.qualifiedName"
                && property.value == json!("Root::Customer::amount"))
    );
    for command in [
        Command::SetOwner {
            element_id: "class".into(),
            owner_id: Some("class".into()),
        },
        Command::SetOwner {
            element_id: "lower".into(),
            owner_id: Some("root".into()),
        },
    ] {
        let before = encode(&edited, "json").expect("before rejected edit");
        assert!(apply(&edited, &command, 2).is_err());
        assert_eq!(
            encode(&edited, "json").expect("after rejected edit"),
            before
        );
    }

    let mut wrong_role = serde_json::to_value(&edited).expect("authored ownership");
    wrong_role["relations"]
        .as_array_mut()
        .expect("relationships")
        .iter_mut()
        .find(|relation| relation["id"] == "owns-use")
        .expect("ownership")["from"] = json!("amount");
    let wrong_role = decode(
        &serde_json::to_vec(&wrong_role).expect("draft bytes"),
        "json",
    )
    .expect("reopen an unresolved ownership role");
    let repaired = apply(
        &wrong_role,
        &Command::SetOwner {
            element_id: "use".into(),
            owner_id: Some("class".into()),
        },
        3,
    )
    .expect("repair the explicit role through the public ownership command");
    assert_eq!(repaired, edited);

    let mut two_attributes = serde_json::to_value(&edited).expect("two authored attributes");
    two_attributes["elements"]
        .as_array_mut()
        .expect("elements")
        .push(json!({
            "id":"other","kind":"uml.attribute","name":"other"
        }));
    two_attributes["relations"]
        .as_array_mut()
        .expect("relationships")
        .push(json!({
            "id":"owns-other","kind":"owns","from":"class","to":"other"
        }));
    let two_attributes = decode(
        &serde_json::to_vec(&two_attributes).expect("document bytes"),
        "json",
    )
    .expect("reopen two attributes");
    for incomplete in [json!([{"$ref":"amount"}]), json!([])] {
        let draft = external_property(&two_attributes, "class", "ownedAttribute", incomplete);
        assert!(inspected_property(&draft, "class", "uml.members.status").is_string());
        assert_eq!(
            decode(
                &encode(&draft, "protobuf").expect("save partial roles"),
                "protobuf"
            )
            .expect("reopen partial roles"),
            draft
        );
    }
    let complete = external_property(
        &two_attributes,
        "class",
        "ownedAttribute",
        json!([{"$ref":"amount"},{"$ref":"other"}]),
    );
    assert!(
        inspected_property(&complete, "class", "uml.ownedMember")
            .as_array()
            .expect("complete role membership")
            .contains(&json!({"$ref":"other"}))
    );

    let legacy = external_property(&edited, "amount", "defaultValue", json!("[]"));
    assert_eq!(
        decode(
            &encode(&legacy, "json").expect("save unresolved declaration"),
            "json"
        )
        .expect("reopen unresolved declaration"),
        legacy
    );

    let mut unknown_type = serde_json::to_value(&edited).expect("authored extension type");
    unknown_type["elements"]
        .as_array_mut()
        .expect("elements")
        .iter_mut()
        .find(|element| element["id"] == "specification")
        .expect("specification")["kind"] = json!("custom.expression");
    let unknown_type = decode(
        &serde_json::to_vec(&unknown_type).expect("extension bytes"),
        "json",
    )
    .expect("reopen a preserved extension");
    assert_eq!(
        decode(
            &encode(&unknown_type, "protobuf").expect("save extension"),
            "protobuf"
        )
        .expect("reopen extension"),
        unknown_type
    );

    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&edited, format).expect("save"), format).expect("reopen");
        assert_eq!(reopened, edited);
        assert_eq!(
            projection(&reopened)
                .hierarchy_rows
                .iter()
                .filter(|row| row.depth > 0)
                .count(),
            7
        );
        assert_eq!(
            inspected_property(&reopened, "amount", "properties.lowerValue"),
            json!({"$ref":"lower"})
        );
    }
}

#[test]
fn visibility_flow_uses_derived_namespace_and_recovers_an_invalid_draft() {
    let project = document();
    assert_eq!(
        inspected_property(&project, "use", "uml.owner"),
        json!("class")
    );
    let inspector =
        inspect_element(&project, "use").expect("inspect a non-member owned NamedElement");
    assert!(
        inspector
            .properties
            .iter()
            .any(|property| property.path == "uml.namespace" && property.value.is_null())
    );

    assert!(
        apply(
            &project,
            &Command::SetProperty {
                element_id: "use".into(),
                path: "properties.visibility".into(),
                value: json!("public"),
            },
            1
        )
        .is_err()
    );
    let draft = external_property(&project, "use", "visibility", json!("public"));
    let draft = external_property(&draft, "amount", "isReadOnly", json!("not a Boolean"));
    let restored = decode(
        &encode(&draft, "protobuf").expect("save invalid draft"),
        "protobuf",
    )
    .expect("reopen invalid draft");
    let repaired = apply(
        &restored,
        &Command::SetProperty {
            element_id: "use".into(),
            path: "properties.visibility".into(),
            value: Value::Null,
        },
        2,
    )
    .expect("repair visibility without blocking on an unrelated draft error");
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&repaired, format).expect("save repair"), format)
            .expect("reopen repair");
        assert_eq!(reopened, repaired);
    }
}

#[test]
fn parameter_role_flow_isolated_from_unrelated_malformed_composites() {
    let mut content = serde_json::to_value(document()).expect("authored namespace");
    content["elements"]
        .as_array_mut()
        .expect("elements")
        .extend([
            json!({"id":"operation","kind":"uml.operation","name":"accept",
            "properties":{"bodyCondition":"unfinished condition"}}),
            json!({"id":"parameter","kind":"uml.parameter","name":"input",
            "properties":{"type":{"$ref":"class"}}}),
        ]);
    content["relations"]
        .as_array_mut()
        .expect("relationships")
        .extend([
            json!({"id":"owns-operation","kind":"owns","from":"class","to":"operation"}),
            json!({"id":"owns-parameter","kind":"owns","from":"operation","to":"parameter"}),
        ]);
    let inferred = decode(&serde_json::to_vec(&content).expect("draft bytes"), "json")
        .expect("reopen the incomplete condition");
    let explicit = external_property(
        &inferred,
        "operation",
        "ownedParameter",
        json!([{"$ref":"parameter"}]),
    );
    for draft in [inferred, explicit] {
        let operation = inspect_element(&draft, "operation").expect("inspect operation parameters");
        assert_eq!(
            operation
                .parameters
                .iter()
                .map(|parameter| parameter.id.as_str())
                .collect::<Vec<_>>(),
            vec!["parameter"]
        );

        let edited = apply(
            &draft,
            &Command::Rename {
                element_id: "parameter".into(),
                name: "renamed".into(),
            },
            1,
        )
        .expect("edit the parameter while preserving the independent condition error");
        for value in [
            json!({"$ref":"parameter"}),
            json!([false,{"$ref":"parameter"}]),
        ] {
            let contradiction = external_property(&edited, "operation", "bodyCondition", value);
            assert!(
                inspect_element(&contradiction, "operation")
                    .expect("inspect contradictory roles")
                    .parameters
                    .is_empty()
            );
        }
        let repaired = apply(
            &edited,
            &Command::SetProperty {
                element_id: "operation".into(),
                path: "properties.bodyCondition".into(),
                value: Value::Null,
            },
            2,
        )
        .expect("repair only the unrelated composite condition");
        for saved in [&edited, &repaired] {
            for format in ["json", "protobuf"] {
                let reopened = decode(&encode(saved, format).expect("save draft"), format)
                    .expect("reopen draft");
                assert_eq!(&reopened, saved);
                let operation = inspect_element(&reopened, "operation")
                    .expect("preserved canonical parameter role");
                assert_eq!(
                    operation
                        .parameters
                        .first()
                        .expect("projected parameter")
                        .name,
                    "renamed"
                );
            }
        }
    }
}

#[test]
fn external_parameter_ownership_survives_reopening() {
    let project = decode(
        &serde_json::to_vec(&json!({
            "id":"external-parameters", "name":"External parameters", "root_model":"root",
            "elements":[
                {"id":"root", "kind":"uml.model", "name":"Root"},
                {"id":"class", "kind":"uml.class", "name":"Owner"},
                {"id":"operation", "kind":"uml.operation", "name":"accept",
                 "properties":{"ownedParameter":[{"href":"other.uml#parameter"}]}},
                {"id":"behavior", "kind":"uml.opaque_behavior", "name":"Execute",
                 "properties":{"ownedParameter":[{"href":"other.uml#parameter"}]}}
            ],
            "relations":[
                {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
                {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"},
                {"id":"owns-behavior", "kind":"owns", "from":"root", "to":"behavior"}
            ]
        }))
        .expect("external parameter document"),
        "json",
    )
    .expect("preserve external roles");
    for format in ["json", "protobuf"] {
        let reopened = decode(
            &encode(&project, format).expect("save unresolved roles"),
            format,
        )
        .expect("reopen unresolved roles");
        assert_eq!(reopened, project);
    }
}
