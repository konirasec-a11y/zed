use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, project_model_section, validate_uml_model,
};
use bellatrix_mde_protocol::{Command, inspector::ElementInspector};
use serde_json::{Value, json};

const READ_ONLY_RULE: &str = "Property-derived_union_is_read_only";
const DERIVED_RULE: &str = "Property-derived_union_is_derived";
const QUALIFIER_RULE: &str = "Property-qualified_is_association_end";

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

fn import_document(model: &Value) -> Value {
    decode(
        &serde_json::to_vec(model).expect("canonical document bytes"),
        "json",
    )
    .expect("import canonical document with codec defaults")
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

fn inspect_element(model: &Value, id: &str) -> Option<ElementInspector> {
    let document = prepare_model_document(&envelope(model)).expect("prepare canonical document");
    project_model_section(&document.id, &document.name, &document.section_json)
        .expect("project the public document")
        .inspect_element(id)
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

fn model_diagnostics(model: &Value) -> Vec<Value> {
    let response =
        validate_uml_model(&envelope(model)).expect("validate through the kernel facade");
    let mut response: Value = serde_json::from_str(&response).expect("validation response");
    response["diagnostics"]
        .as_array_mut()
        .expect("public diagnostics")
        .to_owned()
}

fn element<'a>(model: &'a Value, id: &str) -> &'a Value {
    model["elements"]
        .as_array()
        .expect("canonical elements")
        .iter()
        .find(|element| element["id"] == id)
        .expect("authored Element")
}

fn element_mut<'a>(model: &'a mut Value, id: &str) -> &'a mut Value {
    model["elements"]
        .as_array_mut()
        .expect("canonical elements")
        .iter_mut()
        .find(|element| element["id"] == id)
        .expect("authored Element")
}

fn document() -> Value {
    import_document(&json!({
        "id":"project", "name":"Property invariants", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"a","kind":"uml.class","name":"A"},
            {"id":"b","kind":"uml.class","name":"B"},
            {"id":"attribute","kind":"uml.attribute","name":"items",
             "properties":{"type":{"$ref":"b"}}},
            {"id":"association","kind":"uml.nary_association","name":"Link",
             "properties":{"memberEnd":[{"$ref":"left"},{"$ref":"right"}]}},
            {"id":"left","kind":"uml.attribute","name":"left",
             "properties":{"type":{"$ref":"a"}}},
            {"id":"right","kind":"uml.attribute","name":"right",
             "properties":{"type":{"$ref":"b"}}}
        ],
        "relations":[
            {"id":"owns-a","kind":"owns","from":"root","to":"a"},
            {"id":"owns-b","kind":"owns","from":"root","to":"b"},
            {"id":"owns-attribute","kind":"owns","from":"a","to":"attribute"},
            {"id":"owns-association","kind":"owns","from":"root","to":"association"},
            {"id":"owns-left","kind":"owns","from":"association","to":"left"},
            {"id":"owns-right","kind":"owns","from":"association","to":"right"}
        ]
    }))
}

fn has_error(project: &Value, subject: &str, rule: &str) -> bool {
    model_diagnostics(project).iter().any(|issue| {
        issue["severity"] == "error"
            && issue["subject_id"] == subject
            && issue["message"]
                .as_str()
                .is_some_and(|message| message.contains(rule))
    })
}

fn author(project: &mut Value, subject: &str, property: &str, value: Value) {
    element_mut(project, subject)["properties"][property] = value;
}

fn edit(project: &Value, subject: &str, property: &str, value: Value) -> Result<Value, String> {
    apply(
        project,
        &Command::SetProperty {
            element_id: subject.into(),
            path: format!("properties.{property}"),
            value,
        },
        1,
    )
}

fn add_qualifier(project: &mut Value, owner: &str, explicit: bool) {
    project["elements"]
        .as_array_mut()
        .expect("canonical elements")
        .push(json!({
            "id":"qualifier","kind":"uml.attribute","name":"key",
            "properties":{"type":{"$ref":"a"},"visibility":null}
        }));
    project["relations"]
        .as_array_mut()
        .expect("canonical relations")
        .push(json!({
            "id":"owns-qualifier","kind":"owns","from":owner,"to":"qualifier"
        }));
    if explicit {
        author(project, owner, "qualifier", json!([{"$ref":"qualifier"}]));
    }
    *project = import_document(project);
}

#[test]
fn derived_union_flow_uses_defaults_rejects_inconsistent_edits_and_reopens() {
    let mut project = document();
    assert!(!has_error(&project, "attribute", READ_ONLY_RULE));
    assert!(!has_error(&project, "attribute", DERIVED_RULE));
    // Imported Boolean fields use the existing generic Inspector editor.
    for property in ["isDerived", "isDerivedUnion"] {
        author(&mut project, "attribute", property, json!(false));
    }
    let before = encode(&project, "json").expect("snapshot before rejected edit");
    assert!(
        edit(&project, "attribute", "isDerivedUnion", json!(true))
            .expect_err("derived union requires both flags")
            .contains(READ_ONLY_RULE)
    );
    assert_eq!(
        encode(&project, "json").expect("unchanged document"),
        before
    );
    assert_eq!(
        element(&project, "attribute")["properties"].get("isDerivedUnion"),
        Some(&json!(false))
    );

    for property in ["isReadOnly", "isDerived", "isDerivedUnion"] {
        project = edit(&project, "attribute", property, json!(true)).expect("author valid flags");
    }
    let inspector = inspect_element(&project, "attribute").expect("inspect the same Property");
    for property in ["isReadOnly", "isDerived", "isDerivedUnion"] {
        assert!(
            inspector
                .properties
                .iter()
                .any(|field| field.path == format!("properties.{property}")
                    && field.value == json!(true))
        );
    }
    for (property, rule) in [("isReadOnly", READ_ONLY_RULE), ("isDerived", DERIVED_RULE)] {
        let before = encode(&project, "json").expect("snapshot before inconsistent edit");
        assert!(
            edit(&project, "attribute", property, json!(false))
                .expect_err("inconsistent union flag")
                .contains(rule)
        );
        assert_eq!(encode(&project, "json").expect("atomic rejection"), before);
    }
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&project, format).expect("save"), format).expect("reopen");
        assert_eq!(project, reopened);
        assert!(!has_error(&reopened, "attribute", READ_ONLY_RULE));
        assert!(!has_error(&reopened, "attribute", DERIVED_RULE));
    }
}

#[test]
fn imported_union_drafts_keep_both_diagnostics_and_can_be_repaired_in_place() {
    for kind in ["uml.attribute", "uml.port"] {
        let mut project = document();
        element_mut(&mut project, "attribute")["kind"] = json!(kind);
        author(&mut project, "attribute", "isDerivedUnion", json!(true));
        assert!(has_error(&project, "attribute", READ_ONLY_RULE));
        assert!(has_error(&project, "attribute", DERIVED_RULE));
        for format in ["json", "protobuf"] {
            let reopened = decode(
                &encode(&project, format).expect("save invalid draft"),
                format,
            )
            .expect("reopen invalid draft");
            assert_eq!(reopened, project);
            assert!(has_error(&reopened, "attribute", READ_ONLY_RULE));
            assert!(has_error(&reopened, "attribute", DERIVED_RULE));
        }
        for property in ["isReadOnly", "isDerived"] {
            author(&mut project, "attribute", property, json!(false));
        }
        project = edit(&project, "attribute", "isReadOnly", json!(true)).expect("first repair");
        assert!(!has_error(&project, "attribute", READ_ONLY_RULE));
        assert!(has_error(&project, "attribute", DERIVED_RULE));
        project = edit(&project, "attribute", "isDerived", json!(true)).expect("second repair");
        assert!(!has_error(&project, "attribute", DERIVED_RULE));

        for property in ["isDerivedUnion", "isReadOnly", "isDerived"] {
            for invalid in [Value::Null, json!("true"), json!(1)] {
                assert!(edit(&project, "attribute", property, invalid.clone()).is_err());
                let mut draft = project.clone();
                author(&mut draft, "attribute", property, invalid);
                assert!(model_diagnostics(&draft).iter().any(|issue| {
                    issue["subject_id"] == "attribute"
                        && issue["code"] == "UML_PROPERTY_VALUE"
                        && issue["message"]
                            .as_str()
                            .is_some_and(|message| message.contains(property))
                }));
            }
        }
    }
}

#[test]
fn qualifier_flow_uses_canonical_association_and_ownership_instead_of_stored_inverse() {
    for explicit in [false, true] {
        let mut valid = document();
        add_qualifier(&mut valid, "left", explicit);
        assert!(!has_error(&valid, "left", QUALIFIER_RULE));
        assert_eq!(
            inspected_property(&valid, "left", "uml.association"),
            json!({"$ref":"association"})
        );
        assert!(
            element(&valid, "left")["properties"]
                .get("association")
                .is_none()
        );

        let mut invalid = document();
        add_qualifier(&mut invalid, "attribute", explicit);
        assert!(has_error(&invalid, "attribute", QUALIFIER_RULE));
        for format in ["json", "protobuf"] {
            let reopened = decode(
                &encode(&invalid, format).expect("save qualified draft"),
                format,
            )
            .expect("reopen qualified draft");
            assert_eq!(invalid, reopened);
            assert!(has_error(&reopened, "attribute", QUALIFIER_RULE));
            let reopened = decode(&encode(&valid, format).expect("save valid end"), format)
                .expect("reopen valid end");
            assert_eq!(valid, reopened);
            assert!(!has_error(&reopened, "left", QUALIFIER_RULE));
        }
        let mut forged = invalid.clone();
        author(
            &mut forged,
            "attribute",
            "association",
            json!({"$ref":"association"}),
        );
        assert!(has_error(&forged, "attribute", QUALIFIER_RULE));
        assert!(
            model_diagnostics(&forged)
                .iter()
                .any(|issue| issue["subject_id"] == "attribute"
                    && issue["code"] == "UML_PROPERTY_ASSOCIATION_STORAGE")
        );

        if !explicit {
            let repaired = apply(
                &invalid,
                &Command::SetOwner {
                    element_id: "qualifier".into(),
                    owner_id: Some("left".into()),
                },
                2,
            )
            .expect("move qualifier to an Association end without changing its identity");
            assert!(!has_error(&repaired, "attribute", QUALIFIER_RULE));
            assert!(!has_error(&repaired, "left", QUALIFIER_RULE));
            assert_eq!(
                element(&repaired, "qualifier"),
                element(&invalid, "qualifier")
            );
        }
    }
    let mut empty = document();
    author(&mut empty, "attribute", "qualifier", json!([]));
    assert!(!has_error(&empty, "attribute", QUALIFIER_RULE));
}

#[test]
fn classifier_feature_rules_reject_edits_preserve_drafts_and_reopen() {
    let mut project = import_document(&json!({
        "id":"feature-rules", "name":"Feature rules", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"class","kind":"uml.class","name":"Active","properties":{"isActive":true}},
            {"id":"interface","kind":"uml.interface","name":"Contract"},
            {"id":"operation","kind":"uml.operation","name":"execute","properties":{"visibility":"public"}},
            {"id":"reception","kind":"uml.reception","name":"Receive","properties":{"visibility":"public"}},
            {"id":"behavior","kind":"uml.opaque_behavior","name":"Implement",
             "properties":{"specification":{"$ref":"operation"}}}
        ],
        "relations":[
            {"id":"own-class","kind":"owns","from":"root","to":"class"},
            {"id":"own-interface","kind":"owns","from":"root","to":"interface"},
            {"id":"own-operation","kind":"owns","from":"interface","to":"operation"},
            {"id":"own-reception","kind":"owns","from":"class","to":"reception"},
            {"id":"own-behavior","kind":"owns","from":"class","to":"behavior"}
        ]
    }));
    let cases = [
        ("class", "isActive", json!(false), "Class-passive_class"),
        (
            "operation",
            "isAbstract",
            json!(true),
            "BehavioralFeature-abstract_no_method",
        ),
        (
            "operation",
            "visibility",
            json!("private"),
            "Interface-visibility",
        ),
    ];
    for (subject, property, value, rule) in cases {
        let snapshot = encode(&project, "json").expect("before edit");
        let error =
            edit(&project, subject, property, value.clone()).expect_err("invalid semantic edit");
        assert!(error.contains(rule), "{error}");
        assert_eq!(encode(&project, "json").expect("after rejection"), snapshot);
        let mut draft = project.clone();
        author(&mut draft, subject, property, value);
        let diagnostic_subject = if rule == "Interface-visibility" {
            "interface"
        } else {
            subject
        };
        assert!(has_error(&draft, diagnostic_subject, rule));
        for format in ["json", "protobuf"] {
            let reopened =
                decode(&encode(&draft, format).expect("save draft"), format).expect("reopen draft");
            assert_eq!(reopened, draft);
            assert!(has_error(&reopened, diagnostic_subject, rule));
        }
    }
    // The default false value has the same semantics as an explicit false.
    element_mut(&mut project, "class")["properties"]
        .as_object_mut()
        .expect("Class properties")
        .remove("isActive");
    assert!(has_error(&project, "class", "Class-passive_class"));
    project = edit(&project, "class", "isActive", json!(true)).expect("repair passive draft");
    assert!(!has_error(&project, "class", "Class-passive_class"));

    let mut behavior_only = project.clone();
    behavior_only["relations"]
        .as_array_mut()
        .expect("canonical relations")
        .retain(|relation| relation["to"] != "reception");
    behavior_only["elements"]
        .as_array_mut()
        .expect("canonical elements")
        .retain(|element| element["id"] != "reception");
    author(
        &mut behavior_only,
        "class",
        "classifierBehavior",
        json!({"$ref":"behavior"}),
    );
    assert!(edit(&behavior_only, "class", "isActive", json!(false)).is_err());
    author(
        &mut behavior_only,
        "class",
        "classifierBehavior",
        Value::Null,
    );
    let passive = edit(&behavior_only, "class", "isActive", json!(false))
        .expect("passive without classifier behavior");
    assert!(!has_error(&passive, "class", "Class-passive_class"));
    assert!(edit(&project, "operation", "visibility", Value::Null).is_err());

    // No behavioral execution is needed to enforce the references. A method
    // stored on the Operation and the inverse specification both count.
    element_mut(&mut project, "behavior")["properties"]
        .as_object_mut()
        .expect("Behavior properties")
        .remove("specification");
    author(
        &mut project,
        "operation",
        "method",
        json!([{"$ref":"behavior"}]),
    );
    assert!(edit(&project, "operation", "isAbstract", json!(true)).is_err());
    for methods in [
        json!([{"$ref":"behavior"}, {"href":"other.uml#method"}]),
        json!([{"href":"other.uml#method"}, {"$ref":"behavior"}]),
    ] {
        let mut mixed = project.clone();
        author(&mut mixed, "operation", "method", methods);
        let before = encode(&mixed, "json").expect("mixed draft snapshot");
        let error = edit(&mixed, "operation", "isAbstract", json!(true))
            .expect_err("known method proves the abstract constraint violation");
        assert!(
            error.contains("BehavioralFeature-abstract_no_method"),
            "{error}"
        );
        assert_eq!(
            encode(&mixed, "json").expect("unchanged mixed draft"),
            before
        );
        author(&mut mixed, "operation", "isAbstract", json!(true));
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&mixed, format).expect("save mixed draft"), format)
                .expect("reopen mixed draft");
            assert!(has_error(
                &reopened,
                "operation",
                "BehavioralFeature-abstract_no_method"
            ));
        }
    }
    author(&mut project, "operation", "method", json!([]));
    project =
        edit(&project, "operation", "isAbstract", json!(true)).expect("abstract without method");
    assert!(!has_error(
        &project,
        "operation",
        "BehavioralFeature-abstract_no_method"
    ));

    for (subject, property, value, rule) in [
        (
            "operation",
            "method",
            json!([{"href":"other.uml#method"}]),
            "BehavioralFeature-abstract_no_method",
        ),
        (
            "class",
            "classifierBehavior",
            json!({"href":"other.uml#behavior"}),
            "Class-passive_class",
        ),
        (
            "interface",
            "ownedOperation",
            json!([{"href":"other.uml#operation"}]),
            "Interface-visibility",
        ),
    ] {
        let mut draft = project.clone();
        if subject == "class" {
            draft["relations"]
                .as_array_mut()
                .expect("canonical relations")
                .retain(|relation| relation["to"] != "reception");
            draft["elements"]
                .as_array_mut()
                .expect("canonical elements")
                .retain(|element| element["id"] != "reception");
            author(&mut draft, "class", "isActive", json!(false));
        }
        if subject == "interface" {
            draft["relations"]
                .as_array_mut()
                .expect("canonical relations")
                .retain(|relation| relation["to"] != "operation");
            draft["elements"]
                .as_array_mut()
                .expect("canonical elements")
                .retain(|element| element["id"] != "operation");
        }
        author(&mut draft, subject, property, value);
        let issues = model_diagnostics(&draft);
        assert!(
            issues.iter().any(|issue| issue["subject_id"] == subject
                && issue["severity"] == "warning"
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(rule))),
            "{issues:?}"
        );
        assert!(!has_error(&draft, subject, rule));
    }
}
