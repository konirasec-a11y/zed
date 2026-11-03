//! Feature flows through public commands, inspector, canonical validation and I/O.
//! Normative contracts: UML 2.5.1 Property metadata, Constraint and OpaqueExpression.
use bellatrix_mde::{
    Format, ModelingSection, Project, StructuralSeverity,
    commands::{Command, apply}, decode, encode, inspect_element,
    uml::{
        references::ReferenceIndex,
        root::Model,
        value_specifications::{self, EvaluationError, LiteralValue},
    },
};
use serde_json::{Value, json};

fn document() -> Project {
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
    })).expect("document bytes"), Format::Json).expect("import document")
}

fn external_edit(project: &Project, subject: &str, property: &str, value: Value) -> Project {
    let mut document: Value = serde_json::from_slice(&encode(project, Format::Json)
        .expect("save before external edit")).expect("JSON document");
    let element = document["elements"].as_array_mut().expect("elements")
        .iter_mut().find(|element| element["id"].as_str() == Some(subject)).expect("subject");
    element["properties"][property] = value;
    decode(&serde_json::to_vec(&document).expect("edited bytes"), Format::Json)
        .expect("reload authored draft")
}

fn has_error(project: &Project, subject: &str, rule: &str) -> bool {
    Model::from_project(project).validate().iter().any(|diagnostic|
        diagnostic.severity == StructuralSeverity::Error
            && diagnostic.subject_id == subject && diagnostic.message.contains(rule))
}

#[test]
fn structural_document_flow_edits_inspects_rejects_bad_references_and_reopens_drafts() {
    // Given one canonical document, including an ordered non-unique body and a
    // Boolean false constraint expression (false is not a metamodel type error).
    let mut project = document();
    assert!(!has_error(&project, "expression", "OpaqueExpression-body"));
    assert!(!has_error(&project, "condition", "Constraint-boolean_value"));

    // When authoring through the real command and inspector contracts.
    project = apply(&project, &Command::Rename {
        element_id: "a".into(), name: "Account".into(),
    }, 1).expect("rename through the domain");
    project = apply(&project, &Command::SetProperty {
        element_id: "parameter".into(), path: "properties.type".into(), value: json!({"$ref":"b"}),
    }, 2).expect("choose a canonical Type");
    let inspector = inspect_element(&ModelingSection::from_model(&project), "operation")
        .expect("inspect Operation");
    assert!(inspector.parameters.iter().any(|parameter| parameter.id == "parameter"));
    let before = encode(&project, Format::Json).expect("before rejection");
    assert!(apply(&project, &Command::SetProperty {
        element_id: "parameter".into(), path: "properties.type".into(),
        value: json!({"$ref":"condition"}),
    }, 3).is_err());
    assert_eq!(encode(&project, Format::Json).expect("after rejection"), before);

    // Then externally authored collections are validated as part of the whole
    // document, not silently accepted because the scalar editor cannot edit them.
    for invalid in [json!([{"$ref":"b"},{"$ref":"b"}]),
                    json!([{"$ref":"missing"}]), json!([{"$ref":"condition"}]), json!("b")] {
        let draft = external_edit(&project, "operation", "raisedException", invalid);
        assert!(has_error(&draft, "operation", "Operation-raisedException"));
        let reopened = decode(&encode(&draft, Format::Json).expect("save invalid draft"),
            Format::Json).expect("reopen invalid draft");
        assert_eq!(draft, reopened);
        assert!(has_error(&reopened, "operation", "Operation-raisedException"));
    }
    let bad_body = external_edit(&project, "expression", "body", json!(["value", 12]));
    assert!(has_error(&bad_body, "expression", "OpaqueExpression-body"));
    let duplicate_languages = external_edit(&project, "expression", "language", json!(["Rust","Rust"]));
    assert!(has_error(&duplicate_languages, "expression", "OpaqueExpression-language"));
    let mismatched_languages = external_edit(&project, "expression", "language", json!(["Rust"]));
    assert!(has_error(&mismatched_languages, "expression", "OpaqueExpression-language_body_size"));

    // Unsupported declarations remain visible and round-trip, but are not
    // presented as a checked UML contract. Repair uses the same command path.
    let unresolved = external_edit(&project, "parameter", "type", json!("LegacyName"));
    assert!(Model::from_project(&unresolved).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "parameter"
            && diagnostic.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"));
    project = apply(&unresolved, &Command::SetProperty {
        element_id: "parameter".into(), path: "properties.type".into(), value: json!({"$ref":"b"}),
    }, 4).expect("resolve preserved declaration");
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format).expect("save"), format).expect("reopen");
        assert_eq!(project, reopened);
        assert!(!has_error(&reopened, "operation", "Operation-raisedException"));
        assert_eq!(reopened.element("a").expect("same identity").name, "Account");
    }
}

#[test]
fn constraint_authoring_flow_keeps_identity_ownership_boolean_contract_and_recovery() {
    let project = document();
    let mut draft = project.clone();
    draft.elements.iter_mut().find(|element| element.id == "condition").expect("Constraint")
        .properties.remove("specification");
    draft.elements.retain(|element| element.id != "literal");
    draft.relations.retain(|relation| relation.to != "literal");
    let diagnostics = Model::from_project(&draft).validate();
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.subject_id == "condition"
        && diagnostic.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
        && diagnostic.severity == StructuralSeverity::Warning
        && diagnostic.message.contains("Constraint draft has no authored specification")));
    assert!(!diagnostics.iter().any(|diagnostic| diagnostic.subject_id == "condition"
        && diagnostic.severity == StructuralSeverity::Error));
    for value in [Value::Null, json!({"$ref":"missing"}), json!(false)] {
        let invalid = external_edit(&draft, "condition", "specification", value);
        assert!(has_error(&invalid, "condition", "Constraint-specification"));
    }
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&draft, format).expect("save incomplete Constraint"), format)
            .expect("reopen incomplete Constraint");
        assert_eq!(draft, reopened);
        assert_eq!(Model::from_project(&reopened).validate(), diagnostics);
    }
    let self_constrained = external_edit(&project, "condition", "constrainedElement",
        json!([{"$ref":"condition"}]));
    assert!(has_error(&self_constrained, "condition", "Constraint-not_apply_to_self"));
    let wrong_owner = external_edit(&project, "condition", "specification", json!({"$ref":"expression"}));
    assert!(has_error(&wrong_owner, "condition", "Constraint-specification"));

    // Reload a draft whose expression has the wrong UML value type, repair it,
    // then edit through the public command without changing its semantic ID.
    let mut content = serde_json::to_value(&project).expect("document");
    let literal = content["elements"].as_array_mut().expect("elements").iter_mut()
        .find(|element| element["id"] == "literal").expect("literal");
    literal["kind"] = json!("uml.literal_integer");
    literal["properties"]["value"] = json!(7);
    let draft = decode(&serde_json::to_vec(&content).expect("draft"), Format::Json).expect("reload");
    assert!(has_error(&draft, "condition", "Constraint-boolean_value"));
    let draft = decode(&encode(&draft, Format::Protobuf).expect("save draft"),
        Format::Protobuf).expect("reopen draft");
    assert!(has_error(&draft, "condition", "Constraint-boolean_value"));
    let repaired = decode(&encode(&project, Format::Json).expect("repaired document"),
        Format::Json).expect("reload repair");
    let repaired = apply(&repaired, &Command::Rename {
        element_id: "condition".into(), name: "MaySubmit".into(),
    }, 5).expect("edit after repair");
    let restored = decode(&encode(&repaired, Format::Json).expect("save repair"),
        Format::Json).expect("reopen repair");
    assert_eq!(restored.element("condition").expect("same Constraint").name, "MaySubmit");
    assert!(!has_error(&restored, "condition", "Constraint-boolean_value"));
    assert!(!has_error(&restored, "condition", "Constraint-not_apply_to_self"));
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
    let ownership = literals.iter().map(|literal| json!({
        "id":format!("owns-{}", literal["id"].as_str().expect("literal identity")),
        "kind":"owns", "from":"root", "to":literal["id"],
    })).collect::<Vec<_>>();
    content["elements"].as_array_mut().ok_or("elements are missing")?.extend(literals);
    content["relations"].as_array_mut().ok_or("relations are missing")?.extend(ownership);
    let mut project = decode(&serde_json::to_vec(&content)?, Format::Json)?;
    let references = ReferenceIndex::new(Model::from_project(&project));
    assert_eq!(value_specifications::evaluate(&references, "real")?, LiteralValue::Real(1.25));
    for id in ["real", "integer", "unlimited", "string", "null", "boolean", "literal"] {
        assert!(value_specifications::is_computable(&references, id)?);
    }
    assert_eq!(value_specifications::integer_value(&references, "integer")?, Some(0.into()));
    assert!(value_specifications::unlimited_value(&references, "unlimited")?
        .ok_or("unlimited value is missing")?.is_unlimited());
    assert_eq!(value_specifications::boolean_value(&references, "boolean")?, Some(false));
    assert_eq!(value_specifications::real_value(&references, "real")?, Some(1.25));
    assert_eq!(value_specifications::real_value(&references, "integer")?, None);
    assert_eq!(value_specifications::integer_value(&references, "real")?, None);
    assert_eq!(value_specifications::boolean_value(&references, "real")?, None);
    assert_eq!(value_specifications::string_value(&references, "string")?, None);
    assert_eq!(value_specifications::evaluate(&references, "string")?, LiteralValue::String(None));
    assert!(!value_specifications::is_null(&references, "string")?);
    assert!(value_specifications::is_null(&references, "null")?);
    assert_eq!(value_specifications::evaluate(&references, "null")?, LiteralValue::Null);
    assert_eq!(value_specifications::real_value(&references, "null")?, None);
    assert_eq!(value_specifications::string_value(&references, "null")?, None);
    assert!(!value_specifications::is_null(&references, "real")?);
    assert!(!value_specifications::is_computable(&references, "expression")?);
    assert!(!value_specifications::is_null(&references, "expression")?);
    assert_eq!(value_specifications::real_value(&references, "expression")?, None);
    assert_eq!(value_specifications::boolean_value(&references, "expression")?, None);
    assert!(matches!(value_specifications::evaluate(&references, "expression"),
        Err(EvaluationError::Unsupported { .. })));
    for id in ["missing", "a"] {
        assert!(matches!(value_specifications::is_computable(&references, id),
            Err(EvaluationError::Invalid { .. })));
        assert!(matches!(value_specifications::real_value(&references, id),
            Err(EvaluationError::Invalid { .. })));
    }

    project = apply(&project, &Command::SetProperty {
        element_id: "real".into(), path: "properties.value".into(), value: json!(-0.125),
    }, 10)?;
    // The inspector edits declared scalar fields; import a value before editing
    // the optional String field that was deliberately absent above.
    project = external_edit(&project, "string", "value", json!("authored text"));
    project = apply(&project, &Command::SetProperty {
        element_id: "string".into(), path: "properties.value".into(), value: json!(""),
    }, 11)?;
    assert_eq!(value_specifications::string_value(
        &ReferenceIndex::new(Model::from_project(&project)), "string")?, Some(String::new()));
    let before = encode(&project, Format::Json)?;
    for invalid in [Value::Null, json!("3.5"), json!("NaN"), json!("Infinity"), json!(true), json!([])] {
        assert!(apply(&project, &Command::SetProperty {
            element_id: "real".into(), path: "properties.value".into(), value: invalid.clone(),
        }, 12).is_err());
        assert_eq!(encode(&project, Format::Json)?, before);
        let draft = external_edit(&project, "real", "value", invalid);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&draft, format)?, format)?;
            assert_eq!(draft, reopened);
            let references = ReferenceIndex::new(Model::from_project(&reopened));
            assert!(matches!(value_specifications::evaluate(&references, "real"),
                Err(EvaluationError::Invalid { .. })));
            assert!(matches!(value_specifications::is_computable(&references, "real"),
                Err(EvaluationError::Invalid { .. })));
            assert!(matches!(value_specifications::real_value(&references, "real"),
                Err(EvaluationError::Invalid { .. })));
            assert!(matches!(value_specifications::is_null(&references, "real"),
                Err(EvaluationError::Invalid { .. })));
            assert!(has_error(&reopened, "real", "Real::value"));
        }
    }

    let mut missing = project.clone();
    missing.elements.iter_mut().find(|element| element.id == "real")
        .ok_or("real literal is missing")?.properties.remove("value");
    assert!(has_error(&missing, "real", "LiteralReal-value"));
    assert!(matches!(value_specifications::real_value(
        &ReferenceIndex::new(Model::from_project(&missing)), "real"),
        Err(EvaluationError::Invalid { .. })));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&missing, format)?, format)?;
        assert_eq!(missing, reopened);
        assert!(has_error(&reopened, "real", "LiteralReal-value"));
    }
    // Missing fields cannot be created by SetProperty. Repair the external
    // document, then exercise editing through the existing scalar inspector.
    let repaired = external_edit(&missing, "real", "value", json!(0.25));
    let repaired = apply(&repaired, &Command::SetProperty {
        element_id: "real".into(), path: "properties.value".into(), value: json!(-0.125),
    }, 13)?;
    assert!(!has_error(&repaired, "real", "LiteralReal-value"));

    let mut real_condition = repaired.clone();
    let literal = real_condition.elements.iter_mut().find(|element| element.id == "literal")
        .ok_or("constraint literal is missing")?;
    literal.kind = "uml.literal_real".into();
    literal.properties.insert("value".into(), json!(0.5));
    assert!(has_error(&real_condition, "condition", "Constraint-boolean_value"));
    assert!(!Model::from_project(&real_condition).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "condition"
            && diagnostic.severity == StructuralSeverity::Warning
            && diagnostic.message.contains("Constraint-boolean_value")));
    let mut opaque_condition = repaired.clone();
    let literal = opaque_condition.elements.iter_mut().find(|element| element.id == "literal")
        .ok_or("constraint literal is missing")?;
    literal.kind = "uml.opaque_expression".into();
    literal.properties.remove("value");
    literal.properties.insert("body".into(), json!(["true"]));
    literal.properties.insert("language".into(), json!(["Rust"]));
    assert!(!has_error(&opaque_condition, "condition", "Constraint-boolean_value"));
    assert!(Model::from_project(&opaque_condition).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "condition"
            && diagnostic.severity == StructuralSeverity::Warning
            && diagnostic.message.contains("Constraint-boolean_value")));

    let mut real_bound = repaired.clone();
    real_bound.elements.iter_mut().find(|element| element.id == "parameter")
        .ok_or("parameter is missing")?.properties.insert("lowerValue".into(), json!({"$ref":"real"}));
    real_bound.elements.iter_mut().find(|element| element.id == "real")
        .ok_or("real literal is missing")?.properties.insert("visibility".into(), Value::Null);
    real_bound.relations.iter_mut().find(|relation| relation.id == "owns-real")
        .ok_or("literal ownership is missing")?.from = "parameter".into();
    assert!(has_error(&real_bound, "parameter", "lower_is_integer"));
    let references = ReferenceIndex::new(Model::from_project(&real_bound));
    assert!(matches!(value_specifications::multiplicity(&references, references.element("parameter")?),
        Err(EvaluationError::Invalid { .. })));

    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&repaired, format)?, format)?;
        assert_eq!(repaired, reopened);
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        assert!(value_specifications::is_computable(&references, "real")?);
        assert_eq!(value_specifications::real_value(&references, "real")?, Some(-0.125));
        assert_eq!(value_specifications::string_value(&references, "string")?, Some(String::new()));
        assert_eq!(value_specifications::boolean_value(&references, "literal")?, Some(false));
        assert!(!value_specifications::is_computable(&references, "expression")?);
        assert!(value_specifications::is_null(&references, "null")?);
        assert!(!has_error(&reopened, "condition", "Constraint-boolean_value"));
    }
    Ok(())
}
