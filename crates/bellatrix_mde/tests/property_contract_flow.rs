//! Feature flows through public commands, inspector and I/O.
//! Normative contracts: UML 2.5.1 Property metadata, Constraint and OpaqueExpression.
use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, project_model_section,
};
use bellatrix_mde::contracts::protocol::{
    Command, inspector::ElementInspector, projection::ModelingProjection,
};
use serde_json::{Value, json};

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
    decode(&serde_json::to_vec(&json!({
        "id":"project", "name":"Contracts", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"a","kind":"uml.class","name":"Customer"},
            {"id":"b","kind":"uml.class","name":"Failure"},
            {"id":"operation","kind":"uml.operation","name":"submit",
             "properties":{"raisedException":[{"$ref":"b"}]}},
            {"id":"parameter","kind":"uml.parameter","name":"input",
             "properties":{"type":{"$ref":"a"}}},
            {"id":"condition","kind":"uml.constraint","name":"Allowed",
             "properties":{"specification":{"$ref":"literal"},"constrainedElement":[{"$ref":"a"}]}},
            {"id":"literal","kind":"uml.literal_boolean","name":"",
             "properties":{"value":false,"visibility":null}},
            {"id":"expression","kind":"uml.opaque_expression","name":"",
             "properties":{"body":["value","value"],"language":["Rust","SQL"]}}
        ],
        "relations":[
            {"id":"owns-a","kind":"owns","from":"root","to":"a"},
            {"id":"owns-b","kind":"owns","from":"root","to":"b"},
            {"id":"owns-operation","kind":"owns","from":"a","to":"operation"},
            {"id":"owns-parameter","kind":"owns","from":"operation","to":"parameter"},
            {"id":"owns-condition","kind":"owns","from":"a","to":"condition"},
            {"id":"owns-literal","kind":"owns","from":"condition","to":"literal"},
            {"id":"owns-expression","kind":"owns","from":"root","to":"expression"}
        ]
    })).expect("document bytes"), "json").expect("import document")
}

fn external_edit(project: &Value, subject: &str, property: &str, value: Value) -> Value {
    let mut document: Value =
        serde_json::from_slice(&encode(project, "json").expect("save before external edit"))
            .expect("JSON document");
    let element = document["elements"]
        .as_array_mut()
        .expect("elements")
        .iter_mut()
        .find(|element| element["id"].as_str() == Some(subject))
        .expect("subject");
    element["properties"][property] = value;
    decode(
        &serde_json::to_vec(&document).expect("edited bytes"),
        "json",
    )
    .expect("reload authored draft")
}

#[test]
fn structural_document_flow_edits_inspects_rejects_bad_references_and_reopens_drafts() {
    // Given one canonical document, including an ordered non-unique body and a
    // Boolean false constraint expression (false is not a metamodel type error).
    let mut project = document();

    // When authoring through the real command and inspector contracts.
    project = apply(
        &project,
        &Command::Rename {
            element_id: "a".into(),
            name: "Account".into(),
        },
        1,
    )
    .expect("rename through the domain");
    project = apply(
        &project,
        &Command::SetProperty {
            element_id: "parameter".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"b"}),
        },
        2,
    )
    .expect("choose a canonical Type");
    let inspector = inspect_element(&project, "operation").expect("inspect Operation");
    assert!(
        inspector
            .parameters
            .iter()
            .any(|parameter| parameter.id == "parameter")
    );
    let before = encode(&project, "json").expect("before rejection");
    assert!(
        apply(
            &project,
            &Command::SetProperty {
                element_id: "parameter".into(),
                path: "properties.type".into(),
                value: json!({"$ref":"condition"}),
            },
            3
        )
        .is_err()
    );
    assert_eq!(encode(&project, "json").expect("after rejection"), before);

    for invalid in [
        json!([{"$ref":"b"},{"$ref":"b"}]),
        json!([{"$ref":"missing"}]),
        json!([{"$ref":"condition"}]),
        json!("b"),
    ] {
        let draft = external_edit(&project, "operation", "raisedException", invalid);
        let reopened = decode(&encode(&draft, "json").expect("save invalid draft"), "json")
            .expect("reopen invalid draft");
        assert_eq!(draft, reopened);
    }

    // Unsupported declarations remain visible and round-trip, but are not
    // presented as a checked UML contract. Repair uses the same command path.
    let unresolved = external_edit(&project, "parameter", "type", json!("LegacyName"));
    project = apply(
        &unresolved,
        &Command::SetProperty {
            element_id: "parameter".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"b"}),
        },
        4,
    )
    .expect("resolve preserved declaration");
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&project, format).expect("save"), format).expect("reopen");
        assert_eq!(project, reopened);
        assert_eq!(
            inspect_element(&reopened, "a").expect("same identity").name,
            "Account"
        );
    }
}

#[test]
fn constraint_authoring_flow_preserves_incomplete_drafts_and_identity() {
    let project = document();
    let mut draft = project.clone();
    draft["elements"]
        .as_array_mut()
        .expect("elements")
        .iter_mut()
        .find(|element| element["id"] == "condition")
        .expect("Constraint")["properties"]
        .as_object_mut()
        .expect("Constraint properties")
        .remove("specification");
    draft["elements"]
        .as_array_mut()
        .expect("elements")
        .retain(|element| element["id"] != "literal");
    draft["relations"]
        .as_array_mut()
        .expect("relationships")
        .retain(|relation| relation["to"] != "literal");
    for format in ["json", "protobuf"] {
        let reopened = decode(
            &encode(&draft, format).expect("save incomplete Constraint"),
            format,
        )
        .expect("reopen incomplete Constraint");
        assert_eq!(draft, reopened);
    }

    let repaired = decode(
        &encode(&project, "json").expect("repaired document"),
        "json",
    )
    .expect("reload repair");
    let repaired = apply(
        &repaired,
        &Command::Rename {
            element_id: "condition".into(),
            name: "MaySubmit".into(),
        },
        5,
    )
    .expect("edit after repair");
    let restored =
        decode(&encode(&repaired, "json").expect("save repair"), "json").expect("reopen repair");
    assert_eq!(
        inspect_element(&restored, "condition")
            .expect("same Constraint")
            .name,
        "MaySubmit"
    );
}

#[test]
fn literal_authoring_flow_distinguishes_computable_absent_invalid_and_opaque_values()
-> Result<(), Box<dyn std::error::Error>> {
    let mut content = serde_json::to_value(document())?;
    let literals = [
        json!({"id":"real","kind":"uml.literal_real","name":"not the value",
            "properties":{"value":1.25}}),
        json!({"id":"integer","kind":"uml.literal_integer","name":""}),
        json!({"id":"unlimited","kind":"uml.literal_unlimited_natural","name":"",
            "properties":{"value":"*"}}),
        json!({"id":"string","kind":"uml.literal_string","name":""}),
        json!({"id":"null","kind":"uml.literal_null","name":""}),
        json!({"id":"boolean","kind":"uml.literal_boolean","name":""}),
    ];
    let ownership = literals
        .iter()
        .map(|literal| {
            json!({
                "id":format!("owns-{}", literal["id"].as_str().expect("literal identity")),
                "kind":"owns", "from":"root", "to":literal["id"],
            })
        })
        .collect::<Vec<_>>();
    content["elements"]
        .as_array_mut()
        .ok_or("elements are missing")?
        .extend(literals);
    content["relations"]
        .as_array_mut()
        .ok_or("relations are missing")?
        .extend(ownership);
    let mut project = decode(&serde_json::to_vec(&content)?, "json")?;
    // Public projections preserve authored values and distinguish absent, null
    // and opaque kinds. They do not expose the internal evaluator's typed
    // getters, isComputable/isNull or derived defaults (Integer 0/Boolean false).
    assert_eq!(
        inspected_property(&project, "real", "properties.value"),
        json!(1.25)
    );
    assert_eq!(
        inspected_property(&project, "unlimited", "properties.value"),
        json!("*")
    );
    assert_eq!(
        inspected_property(&project, "literal", "properties.value"),
        json!(false)
    );
    for (id, kind) in [
        ("real", "uml.literal_real"),
        ("integer", "uml.literal_integer"),
        ("unlimited", "uml.literal_unlimited_natural"),
        ("string", "uml.literal_string"),
        ("null", "uml.literal_null"),
        ("boolean", "uml.literal_boolean"),
        ("literal", "uml.literal_boolean"),
        ("expression", "uml.opaque_expression"),
        ("a", "uml.class"),
    ] {
        assert_eq!(
            inspect_element(&project, id)
                .ok_or("projected identity is missing")?
                .kind,
            kind
        );
    }
    for id in ["integer", "boolean", "string", "null", "expression"] {
        assert!(
            !inspect_element(&project, id)
                .ok_or("projected identity is missing")?
                .properties
                .iter()
                .any(|property| property.path == "properties.value")
        );
    }
    assert!(inspect_element(&project, "missing").is_none());

    project = apply(
        &project,
        &Command::SetProperty {
            element_id: "real".into(),
            path: "properties.value".into(),
            value: json!(-0.125),
        },
        10,
    )?;
    // The inspector edits declared scalar fields; import a value before editing
    // the optional String field that was deliberately absent above.
    project = external_edit(&project, "string", "value", json!("authored text"));
    project = apply(
        &project,
        &Command::SetProperty {
            element_id: "string".into(),
            path: "properties.value".into(),
            value: json!(""),
        },
        11,
    )?;
    assert_eq!(
        inspected_property(&project, "string", "properties.value"),
        json!("")
    );
    let before = encode(&project, "json")?;
    for invalid in [
        Value::Null,
        json!("3.5"),
        json!("NaN"),
        json!("Infinity"),
        json!(true),
        json!([]),
    ] {
        assert!(
            apply(
                &project,
                &Command::SetProperty {
                    element_id: "real".into(),
                    path: "properties.value".into(),
                    value: invalid.clone(),
                },
                12
            )
            .is_err()
        );
        assert_eq!(encode(&project, "json")?, before);
        let draft = external_edit(&project, "real", "value", invalid);
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&draft, format)?, format)?;
            assert_eq!(draft, reopened);
        }
    }

    let mut missing = project.clone();
    missing["elements"]
        .as_array_mut()
        .ok_or("elements are missing")?
        .iter_mut()
        .find(|element| element["id"] == "real")
        .ok_or("real literal is missing")?["properties"]
        .as_object_mut()
        .ok_or("literal properties are missing")?
        .remove("value");
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&missing, format)?, format)?;
        assert_eq!(missing, reopened);
    }
    // Missing fields cannot be created by SetProperty. Repair the external
    // document, then exercise editing through the existing scalar inspector.
    let repaired = external_edit(&missing, "real", "value", json!(0.25));
    let repaired = apply(
        &repaired,
        &Command::SetProperty {
            element_id: "real".into(),
            path: "properties.value".into(),
            value: json!(-0.125),
        },
        13,
    )?;

    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&repaired, format)?, format)?;
        assert_eq!(repaired, reopened);
        assert_eq!(
            inspected_property(&reopened, "real", "properties.value"),
            json!(-0.125)
        );
        assert_eq!(
            inspected_property(&reopened, "string", "properties.value"),
            json!("")
        );
        assert_eq!(
            inspected_property(&reopened, "literal", "properties.value"),
            json!(false)
        );
        assert_eq!(
            inspect_element(&reopened, "expression")
                .ok_or("opaque expression is missing")?
                .kind,
            "uml.opaque_expression"
        );
        assert_eq!(
            inspect_element(&reopened, "null")
                .ok_or("null literal is missing")?
                .kind,
            "uml.literal_null"
        );
    }
    Ok(())
}
