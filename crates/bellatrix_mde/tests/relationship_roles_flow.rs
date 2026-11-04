use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, project_model_section, validate_uml_model,
};
use bellatrix_mde_contracts::protocol::Command;
use serde_json::{Value, json};

fn request(project: &Value) -> String {
    json!({"api_version": 1, "model": project}).to_string()
}

fn decode(bytes: &[u8], format: &str) -> Result<Value, String> {
    let envelope = decode_model_document(bytes, format)?;
    let envelope: Value = serde_json::from_str(&envelope).map_err(|error| error.to_string())?;
    envelope
        .get("model")
        .cloned()
        .ok_or_else(|| "model response is missing model".into())
}

fn encode(project: &Value, format: &str) -> Result<Vec<u8>, String> {
    encode_model_document(&request(project), format)
}

fn apply(project: &Value, command: &Command, revision: u64) -> Result<Value, String> {
    let document = prepare_model_document(&request(project))?;
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

fn findings(project: &Value) -> Vec<Value> {
    let response = validate_uml_model(&request(project)).expect("public UML validation");
    let response: Value = serde_json::from_str(&response).expect("validation response");
    response["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .clone()
}

fn subject<'a>(project: &'a Value, id: &str) -> &'a Value {
    project["elements"]
        .as_array()
        .expect("elements")
        .iter()
        .chain(project["relations"].as_array().expect("relations"))
        .find(|item| item["id"] == id)
        .expect("authored subject")
}

fn inspected(project: &Value, id: &str, path: &str) -> Value {
    let document = prepare_model_document(&request(project)).expect("canonical section");
    let projection = project_model_section(&document.id, &document.name, &document.section_json)
        .expect("public read projection");
    projection
        .inspect_element(id)
        .expect("inspector")
        .properties
        .into_iter()
        .find(|property| property.path == path)
        .expect("inspector property")
        .value
}

fn document() -> Value {
    decode(
        &serde_json::to_vec(&json!({
            "id":"project", "name":"Relationships", "root_model":"root",
            "elements":[
                {"id":"root","kind":"uml.model","name":"Root"},
                {"id":"a","kind":"uml.class","name":"Customer"},
                {"id":"b","kind":"uml.class","name":"Service"},
                {"id":"c","kind":"uml.class","name":"Repository"},
                {"id":"d","kind":"uml.interface","name":"Contract"},
                {"id":"comment","kind":"uml.comment","name":"Documentation"},
                {"id":"object","kind":"uml.usage","name":"ObjectDependency",
                 "properties":{"client":[{"$ref":"a"},{"$ref":"d"}],
                               "supplier":[{"$ref":"a"},{"$ref":"c"}]}},
                {"id":"flow","kind":"uml.information_flow","name":"Information",
                 "properties":{"informationSource":[{"$ref":"a"},{"$ref":"b"}],
                               "informationTarget":[{"$ref":"c"},{"$ref":"d"}],
                               "conveyed":[{"$ref":"c"}]}}
            ],
            "relations":[
                {"id":"owns-a","kind":"owns","from":"root","to":"a"},
                {"id":"owns-b","kind":"owns","from":"root","to":"b"},
                {"id":"owns-c","kind":"owns","from":"root","to":"c"},
                {"id":"owns-d","kind":"owns","from":"root","to":"d"},
                {"id":"owns-comment","kind":"owns","from":"root","to":"comment"},
                {"id":"owns-object","kind":"owns","from":"root","to":"object"},
                {"id":"owns-flow","kind":"owns","from":"root","to":"flow"},
                {"id":"owns-multi","kind":"owns","from":"root","to":"multi"},
                {"id":"owns-binary","kind":"owns","from":"root","to":"binary"},
                {"id":"multi","kind":"uml.dependency","from":"b","to":"d",
                 "properties":{"name":"Multiple",
                               "client":[{"$ref":"a"},{"$ref":"b"}],
                               "supplier":[{"$ref":"c"},{"$ref":"d"}]}},
                {"id":"binary","kind":"uml.dependency","from":"b","to":"c"}
            ]
        }))
        .expect("document bytes"),
        "json",
    )
    .expect("import authored relationships")
}

fn external_edit(project: &Value, subject: &str, property: &str, value: Value) -> Value {
    let mut content = project.clone();
    let collection = if content["elements"]
        .as_array()
        .expect("elements")
        .iter()
        .any(|element| element["id"] == subject)
    {
        "elements"
    } else {
        "relations"
    };
    let element = content[collection]
        .as_array_mut()
        .expect("records")
        .iter_mut()
        .find(|element| element["id"] == subject)
        .expect("subject");
    element["properties"][property] = value;
    decode(&serde_json::to_vec(&content).expect("edited draft"), "json")
        .expect("reload authored draft")
}

fn verify_roles(project: &Value) {
    for (id, clients, suppliers) in [
        (
            "multi",
            json!([{"$ref":"a"},{"$ref":"b"}]),
            json!([{"$ref":"c"},{"$ref":"d"}]),
        ),
        (
            "object",
            json!([{"$ref":"a"},{"$ref":"d"}]),
            json!([{"$ref":"a"},{"$ref":"c"}]),
        ),
    ] {
        assert_eq!(inspected(project, id, "properties.client"), clients);
        assert_eq!(inspected(project, id, "properties.supplier"), suppliers);
    }
    let binary = subject(project, "binary");
    assert_eq!((&binary["from"], &binary["to"]), (&json!("b"), &json!("c")));
    assert!(binary["properties"].get("client").is_none());
    assert!(binary["properties"].get("supplier").is_none());
    assert_eq!(
        inspected(project, "flow", "properties.informationSource"),
        json!([{"$ref":"a"},{"$ref":"b"}])
    );
    assert_eq!(
        inspected(project, "flow", "properties.informationTarget"),
        json!([{"$ref":"c"},{"$ref":"d"}])
    );
    assert!(!findings(project).iter().any(|diagnostic| {
        matches!(
            diagnostic["subject_id"].as_str(),
            Some("multi" | "object" | "binary" | "flow")
        ) && (diagnostic["code"] == "UML_RELATIONSHIP_ENDPOINTS"
            || diagnostic["code"] == "MDE_RELATIONSHIP_ENDPOINTS_UNVERIFIED"
            || diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("canonical binary endpoint")))
    }));
}

#[test]
fn relationship_authoring_flow_preserves_multiple_roles_objects_inverses_and_binary_views() {
    let project = document();
    verify_roles(&project);
    let edited = apply(
        &project,
        &Command::Rename {
            element_id: "a".into(),
            name: "Account".into(),
        },
        1,
    )
    .expect("edit a client without changing identities");
    verify_roles(&edited);
    let before = encode(&edited, "json").expect("before rejected deletion");
    assert!(
        apply(
            &edited,
            &Command::DeleteElement {
                element_id: "a".into()
            },
            2
        )
        .is_err()
    );
    assert_eq!(encode(&edited, "json").expect("original snapshot"), before);
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&edited, format).expect("save"), format).expect("reopen");
        assert_eq!(reopened, edited);
        verify_roles(&reopened);
        let projection = subject(&reopened, "multi");
        assert_eq!(
            (&projection["from"], &projection["to"]),
            (&json!("b"), &json!("d"))
        );
    }
    let expanded = external_edit(
        &edited,
        "multi",
        "client",
        json!([{"$ref":"d"},{"$ref":"b"},{"$ref":"a"}]),
    );
    let expanded = apply(
        &expanded,
        &Command::Rename {
            element_id: "d".into(),
            name: "StorageContract".into(),
        },
        3,
    )
    .expect("edit after expanding the semantic endpoint collection");
    assert_eq!(
        inspected(&expanded, "multi", "properties.client"),
        json!([{"$ref":"d"},{"$ref":"b"},{"$ref":"a"}])
    );
    for format in ["json", "protobuf"] {
        let reopened = decode(
            &encode(&expanded, format).expect("save expanded roles"),
            format,
        )
        .expect("reopen expanded roles");
        assert_eq!(reopened, expanded);
    }
}

#[test]
fn relationship_draft_flow_distinguishes_invalid_unresolved_and_derived_roles_then_recovers() {
    let project = document();
    for value in [
        json!([]),
        json!([{"$ref":"a"},{"$ref":"a"}]),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"comment"}]),
        json!("a"),
    ] {
        let draft = external_edit(&project, "object", "client", value);
        assert!(
            findings(&draft)
                .iter()
                .any(|diagnostic| diagnostic["subject_id"] == "object"
                    && diagnostic["severity"] == "error")
        );
        let reopened = decode(&encode(&draft, "protobuf").expect("save draft"), "protobuf")
            .expect("reopen invalid draft without destroying it");
        assert_eq!(reopened, draft);
        let repaired = external_edit(
            &reopened,
            "object",
            "client",
            json!([{"$ref":"a"},{"$ref":"d"}]),
        );
        verify_roles(&repaired);
    }
    let external = external_edit(
        &project,
        "object",
        "client",
        json!([{"href":"other.xmi#Client"}]),
    );
    assert!(
        findings(&external)
            .iter()
            .any(|diagnostic| diagnostic["subject_id"] == "object"
                && diagnostic["code"] == "MDE_RELATIONSHIP_ENDPOINTS_UNVERIFIED")
    );
    let derived = external_edit(&project, "object", "source", json!([{"$ref":"b"}]));
    assert!(
        findings(&derived)
            .iter()
            .any(|diagnostic| diagnostic["subject_id"] == "object"
                && diagnostic["code"] == "MDE_RELATIONSHIP_ENDPOINTS_UNVERIFIED")
    );
    let inverse = external_edit(
        &project,
        "a",
        "clientDependency",
        json!([{"$ref":"binary"}]),
    );
    assert!(findings(&inverse).iter().any(|diagnostic| {
        diagnostic["subject_id"] == "a"
            && diagnostic["code"] == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("clientDependency"))
    }));
    let inconsistent = external_edit(&project, "multi", "client", json!([{"$ref":"a"}]));
    assert_eq!(
        inspected(&inconsistent, "multi", "properties.client"),
        json!([{"$ref":"a"}])
    );
    assert!(findings(&inconsistent).iter().any(|diagnostic| {
        diagnostic["subject_id"] == "multi"
            && diagnostic["code"] == "UML_RELATIONSHIP_ENDPOINTS"
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("binary projection"))
    }));
}

#[test]
fn dependency_specialization_flow_reads_subset_roles_without_duplicate_authoritative_lists() {
    let mut content = document();
    content["elements"]
        .as_array_mut()
        .expect("elements")
        .push(json!({
            "id":"realization", "kind":"uml.interface_realization", "name":"Implementation",
            "properties":{"implementingClassifier":{"$ref":"b"}, "contract":{"$ref":"d"},
                          "visibility":null}
        }));
    content["relations"]
        .as_array_mut()
        .expect("relations")
        .push(json!({
            "id":"owns-realization", "kind":"owns", "from":"b", "to":"realization"
        }));
    let project = decode(
        &serde_json::to_vec(&content).expect("specialized document"),
        "json",
    )
    .expect("import specialized relationship");
    assert_eq!(
        inspected(&project, "realization", "properties.implementingClassifier"),
        json!({"$ref":"b"})
    );
    assert_eq!(
        inspected(&project, "realization", "properties.contract"),
        json!({"$ref":"d"})
    );
    let edited = apply(
        &project,
        &Command::Rename {
            element_id: "b".into(),
            name: "ImplementationService".into(),
        },
        1,
    )
    .expect("edit using derived Dependency roles");
    let expanded = external_edit(
        &edited,
        "realization",
        "client",
        json!([{"$ref":"a"},{"$ref":"b"}]),
    );
    assert_eq!(
        inspected(&expanded, "realization", "properties.client"),
        json!([{"$ref":"a"},{"$ref":"b"}])
    );
    let invalid = external_edit(&expanded, "realization", "client", json!([{"$ref":"a"}]));
    assert!(findings(&invalid).iter().any(|diagnostic| {
        diagnostic["subject_id"] == "realization"
            && diagnostic["severity"] == "error"
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("specialized roles must subset"))
    }));
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&expanded, format).expect("save subsets"), format)
            .expect("reopen subsets");
        assert_eq!(reopened, expanded);
        assert_eq!(
            inspected(&reopened, "realization", "properties.client"),
            json!([{"$ref":"a"},{"$ref":"b"}])
        );
    }
}
