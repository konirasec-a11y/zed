use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    decode, encode, validate_uml_model,
};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const RULE: &str = "UML_PORT_DEFAULT_VALUE";

fn document(type_id: &str, default: bool) -> Result<Project> {
    let mut project: Project = serde_json::from_value(json!({
        "id":"project", "name":"Port default", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"owner", "kind":"uml.class", "name":"Owner"},
            {"id":"class", "kind":"uml.class", "name":"Implementation"},
            {"id":"interface", "kind":"uml.interface", "name":"Contract"},
            {"id":"port", "kind":"uml.port", "name":"Endpoint", "properties":{
                "aggregation":"composite", "type":{"$ref":type_id}, "defaultValue":null
            }}
        ],
        "relations":[
            {"id":"owns-owner", "kind":"owns", "from":"root", "to":"owner"},
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-interface", "kind":"owns", "from":"root", "to":"interface"},
            {"id":"owns-port", "kind":"owns", "from":"owner", "to":"port"}
        ]
    }))?;
    if default {
        project.elements.push(serde_json::from_value(json!({
            "id":"value", "kind":"uml.literal_integer", "name":"", "properties":{"value":0, "visibility":null}
        }))?);
        project.relations.push(serde_json::from_value(json!({
            "id":"owns-value", "kind":"owns", "from":"port", "to":"value"
        }))?);
        author(&mut project, "defaultValue", json!({"$ref":"value"}));
    }
    Ok(project)
}

fn author(project: &mut Project, key: &str, value: Value) {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "port")
        .expect("Port")
        .properties
        .insert(key.into(), value);
}

fn diagnostics(project: &Project) -> Result<Vec<Value>> {
    let input = json!({"api_version":1, "model":project}).to_string();
    let response: Value = serde_json::from_str(&validate_uml_model(&input)?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics absent")?
        .clone())
}

fn rule_diagnostic(issue: &Value) -> bool {
    issue["subject_id"] == "port"
        && issue["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("Port-default_value:"))
}

#[test]
fn given_interface_typed_port_then_only_nonempty_resolved_default_is_forbidden() -> Result {
    for project in [document("interface", false)?, document("class", true)?] {
        let issues = diagnostics(&project)?;
        assert!(!issues.iter().any(rule_diagnostic), "{issues:#?}");
        assert!(
            !issues.iter().any(|issue| issue["severity"] == "error"),
            "{issues:#?}"
        );
    }
    let invalid = document("interface", true)?;
    let issues = diagnostics(&invalid)?;
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue["code"] == RULE && issue["severity"] == "error")
            .count(),
        1,
        "{issues:#?}"
    );
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&invalid, format)?, format)?;
        assert_eq!(reopened, invalid);
        assert_eq!(diagnostics(&reopened)?, issues);
    }
    let mut attribute = invalid.clone();
    attribute
        .elements
        .iter_mut()
        .find(|element| element.id == "port")
        .ok_or("property missing")?
        .kind = "uml.attribute".into();
    assert!(!diagnostics(&attribute)?.iter().any(rule_diagnostic));
    Ok(())
}

#[test]
fn given_unresolved_references_then_constraint_stays_unverified_without_hiding_invalid_identity()
-> Result {
    for value in [
        json!({"href":"external.uml#Contract"}),
        json!("Contract"),
        json!({"$ref":"missing"}),
    ] {
        let mut project = document("interface", true)?;
        author(&mut project, "type", value.clone());
        let issues = diagnostics(&project)?;
        assert!(
            !issues.iter().any(|issue| issue["code"] == RULE),
            "{issues:#?}"
        );
        assert!(
            issues
                .iter()
                .any(|issue| rule_diagnostic(issue) && issue["severity"] == "warning"),
            "{issues:#?}"
        );
        if value.get("$ref").is_some() {
            assert!(
                issues
                    .iter()
                    .any(|issue| issue["code"] == "UML_TYPED_ELEMENT_TYPE"
                        && issue["severity"] == "error"),
                "{issues:#?}"
            );
        }
    }
    for value in [
        json!({"href":"external.uml#Value"}),
        json!("legacy expression"),
        json!({"$ref":"missing"}),
    ] {
        let mut project = document("interface", false)?;
        author(&mut project, "defaultValue", value.clone());
        let issues = diagnostics(&project)?;
        assert!(
            !issues.iter().any(|issue| issue["code"] == RULE),
            "{issues:#?}"
        );
        assert!(
            issues
                .iter()
                .any(|issue| rule_diagnostic(issue) && issue["severity"] == "warning"),
            "{issues:#?}"
        );
        if value.get("$ref").is_some() {
            assert!(
                issues
                    .iter()
                    .any(|issue| issue["code"] == "UML_PROPERTY_CONTRACT"
                        && issue["severity"] == "error"),
                "{issues:#?}"
            );
        }
    }
    // Empty default proves the consequent without guessing the external Type.
    let mut empty = document("interface", false)?;
    author(&mut empty, "type", json!({"href":"external.uml#Contract"}));
    assert!(!diagnostics(&empty)?.iter().any(rule_diagnostic));
    Ok(())
}

#[test]
fn given_unassigned_owned_value_then_its_role_is_not_invented_as_a_default() -> Result {
    let mut project = document("interface", true)?;
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "port")
        .ok_or("Port missing")?
        .properties
        .remove("defaultValue");
    let issues = diagnostics(&project)?;
    assert!(
        issues
            .iter()
            .any(|issue| rule_diagnostic(issue) && issue["severity"] == "warning"),
        "{issues:#?}"
    );
    assert!(
        !issues.iter().any(|issue| issue["code"] == RULE),
        "{issues:#?}"
    );
    // An explicitly identified lower bound is not a defaultValue.
    author(&mut project, "lowerValue", json!({"$ref":"value"}));
    assert!(!diagnostics(&project)?.iter().any(rule_diagnostic));
    Ok(())
}

#[test]
fn when_type_changes_to_interface_then_default_conflict_rolls_back_and_can_be_repaired() -> Result {
    let project = document("class", true)?;
    let before = encode(&project, Format::Json)?;
    let change = Command::SetProperty {
        element_id: "port".into(),
        path: "properties.type".into(),
        value: json!({"$ref":"interface"}),
    };
    let result = apply(&project, &change, 1);
    assert!(
        matches!(result, Err(ref error) if error.contains(RULE)),
        "{result:?}"
    );
    assert_eq!(encode(&project, Format::Json)?, before);
    let invalid = document("interface", true)?;
    let repaired = apply(
        &invalid,
        &Command::SetProperty {
            element_id: "port".into(),
            path: "properties.type".into(),
            value: json!({"$ref":"class"}),
        },
        1,
    )?;
    assert!(!diagnostics(&repaired)?.iter().any(rule_diagnostic));
    Ok(())
}

#[test]
fn legacy_default_editor_rejects_reference_objects_without_partial_changes() -> Result {
    let mut project = document("interface", true)?;
    author(&mut project, "defaultValue", Value::Null);
    author(&mut project, "lowerValue", json!({"$ref":"value"}));
    assert!(
        !diagnostics(&project)?
            .iter()
            .any(|issue| issue["severity"] == "error")
    );
    let before = encode(&project, Format::Json)?;
    let result = apply(
        &project,
        &Command::SetProperty {
            element_id: "port".into(),
            path: "properties.defaultValue".into(),
            value: json!({"$ref":"value"}),
        },
        1,
    );
    assert!(
        matches!(result, Err(ref error) if error == "expected string"),
        "{result:?}"
    );
    assert_eq!(encode(&project, Format::Json)?, before);
    Ok(())
}

#[test]
fn resolved_wrong_metaclasses_remain_contract_errors_not_successful_port_validation() -> Result {
    for (property, target, code) in [
        ("type", "value", "UML_TYPED_ELEMENT_TYPE"),
        ("defaultValue", "class", "UML_PROPERTY_CONTRACT"),
    ] {
        let mut project = document("interface", true)?;
        // Both identities exist: LiteralInteger is not a Type, and Class is not
        // a ValueSpecification. Resolution alone cannot establish conformance.
        author(&mut project, property, json!({"$ref":target}));
        let input = json!({"api_version":1, "model":project}).to_string();
        let response: Value = serde_json::from_str(&validate_uml_model(&input)?)?;
        assert_eq!(response["has_errors"], true, "{response:#}");
        let issues = response["diagnostics"]
            .as_array()
            .ok_or("diagnostics missing")?;
        assert!(
            issues.iter().any(|issue| issue["subject_id"] == "port"
                && issue["code"] == code
                && issue["severity"] == "error"
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("must conform to UML"))),
            "{issues:#?}"
        );
        assert!(
            issues.iter().any(|issue| rule_diagnostic(issue)
                && issue["code"] == "MDE_PROPERTY_CONTRACT_UNVERIFIED"
                && issue["severity"] == "warning"),
            "{issues:#?}"
        );
        assert!(
            !issues.iter().any(|issue| issue["code"] == RULE),
            "{issues:#?}"
        );
    }
    Ok(())
}
