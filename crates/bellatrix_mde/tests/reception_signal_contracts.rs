use bellatrix_mde::{
    Format, Project, StructuralSeverity,
    commands::{Command, apply}, decode, encode,
    uml::root::Model,
};
use serde_json::{Value, json};

fn document() -> Value {
    json!({
        "id":"receptions", "name":"Receptions", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Receiver", "properties":{"isActive":true}},
            {"id":"other-type", "kind":"uml.class", "name":"Other"},
            {"id":"signal", "kind":"uml.signal", "name":"Changed"},
            {"id":"reception", "kind":"uml.reception", "name":"Changed",
             "properties":{"signal":{"$ref":"signal"}}},
            {"id":"attribute", "kind":"uml.attribute", "name":"value",
             "properties":{"type":{"$ref":"class"}, "multiplicity":"0..*"}},
            {"id":"parameter", "kind":"uml.parameter", "name":"value",
             "properties":{"type":{"$ref":"class"}, "multiplicity":"0..*"}}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-other", "kind":"owns", "from":"root", "to":"other-type"},
            {"id":"owns-signal", "kind":"owns", "from":"root", "to":"signal"},
            {"id":"owns-reception", "kind":"owns", "from":"class", "to":"reception"},
            {"id":"owns-attribute", "kind":"owns", "from":"signal", "to":"attribute"},
            {"id":"owns-parameter", "kind":"owns", "from":"reception", "to":"parameter"}
        ]
    })
}

fn element<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    document["elements"].as_array_mut().expect("elements").iter_mut()
        .find(|element| element["id"] == id).expect("element identity")
}

fn load(document: &Value) -> Project {
    decode(&serde_json::to_vec(document).expect("document bytes"), Format::Json)
        .expect("preserve authored document")
}

fn reception_issues(project: &Project) -> Vec<bellatrix_mde::StructuralDiagnostic> {
    Model::from_project(project).validate().into_iter()
        .filter(|diagnostic| diagnostic.subject_id == "reception"
            && diagnostic.code.contains("RECEPTION_CONSTRAINT"))
        .collect()
}

#[test]
fn reception_matches_signal_and_rejects_each_normative_mismatch() {
    let project = load(&document());
    assert!(reception_issues(&project).is_empty());
    for (owner, property, value, rule) in [
        ("reception", "name", json!("Different"), "same_name_as_signal"),
        ("parameter", "name", json!("different"), "same_structure_as_signal"),
        ("parameter", "direction", json!("out"), "same_structure_as_signal"),
        ("parameter", "type", json!({"$ref":"other-type"}), "same_structure_as_signal"),
        ("parameter", "multiplicity", json!("1..*"), "same_structure_as_signal"),
        ("parameter", "multiplicity", json!("0..1"), "same_structure_as_signal"),
    ] {
        let mut content = document();
        // Type equality is stronger than subtype conformance for this invariant.
        content["relations"].as_array_mut().expect("relations").push(json!({
            "id":"type-parent", "kind":"uml.generalization", "from":"other-type", "to":"class"
        }));
        let record = element(&mut content, owner);
        if property == "name" { record[property] = value; }
        else { record["properties"][property] = value; }
        let draft = load(&content);
        assert!(reception_issues(&draft).iter().any(|diagnostic|
            diagnostic.severity == StructuralSeverity::Error && diagnostic.message.contains(rule)),
            "{owner}::{property}: {:?}", reception_issues(&draft));
    }
    let snapshot = encode(&project, Format::Json).expect("before invalid rename");
    assert!(apply(&project, &Command::Rename {
        element_id:"signal".into(), name:"Different".into(),
    }, 1).is_err());
    assert_eq!(encode(&project, Format::Json).expect("unchanged snapshot"), snapshot);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format).expect("save"), format).expect("reopen");
        assert_eq!(reopened, project);
        assert!(reception_issues(&reopened).is_empty());
    }
}

#[test]
fn reception_structure_uses_owned_attribute_order_and_count_not_inherited_attributes() {
    let mut content = document();
    content["elements"].as_array_mut().expect("elements").extend([
        json!({"id":"second-attribute", "kind":"uml.attribute", "name":"second"}),
        json!({"id":"second-parameter", "kind":"uml.parameter", "name":"second"}),
        json!({"id":"parent-signal", "kind":"uml.signal", "name":"Parent"}),
        json!({"id":"inherited-attribute", "kind":"uml.attribute", "name":"inherited"}),
    ]);
    content["relations"].as_array_mut().expect("relations").extend([
        json!({"id":"owns-second-attribute", "kind":"owns", "from":"signal", "to":"second-attribute"}),
        json!({"id":"owns-second-parameter", "kind":"owns", "from":"reception", "to":"second-parameter"}),
        json!({"id":"owns-parent", "kind":"owns", "from":"root", "to":"parent-signal"}),
        json!({"id":"owns-inherited", "kind":"owns", "from":"parent-signal", "to":"inherited-attribute"}),
        json!({"id":"signal-parent", "kind":"uml.generalization", "from":"signal", "to":"parent-signal"}),
    ]);
    assert!(reception_issues(&load(&content)).is_empty());
    element(&mut content, "signal")["properties"]["ownedAttribute"] =
        json!([{"$ref":"second-attribute"}, {"$ref":"attribute"}]);
    assert!(reception_issues(&load(&content)).iter().any(|diagnostic|
        diagnostic.severity == StructuralSeverity::Error
            && diagnostic.message.contains("same_structure_as_signal")));
    element(&mut content, "signal")["properties"]["ownedAttribute"] =
        json!([{"$ref":"attribute"}, {"$ref":"second-attribute"}]);
    assert!(reception_issues(&load(&content)).is_empty());
    content["elements"].as_array_mut().expect("elements")
        .retain(|record| record["id"] != "second-parameter");
    content["relations"].as_array_mut().expect("relations")
        .retain(|record| record["id"] != "owns-second-parameter");
    assert!(reception_issues(&load(&content)).iter().any(|diagnostic|
        diagnostic.severity == StructuralSeverity::Error
            && diagnostic.message.contains("same_structure_as_signal")));
}

#[test]
fn reception_external_references_stay_unverified_but_known_mismatches_still_fail() {
    for (owner, property) in [("reception", "signal"), ("parameter", "type")] {
        let mut content = document();
        element(&mut content, owner)["properties"][property] = json!({"href":"other.uml#Target"});
        let issues = reception_issues(&load(&content));
        assert!(issues.iter().any(|issue| issue.severity == StructuralSeverity::Warning), "{issues:?}");
        assert!(!issues.iter().any(|issue| issue.severity == StructuralSeverity::Error), "{issues:?}");
        if property == "type" {
            element(&mut content, "parameter")["properties"]["direction"] = json!("out");
            assert!(reception_issues(&load(&content)).iter().any(|issue|
                issue.severity == StructuralSeverity::Error
                    && issue.message.contains("same_structure_as_signal")));
        }
    }
    for signal in [Value::Null, json!({"$ref":"missing"}), json!({"$ref":"class"})] {
        let mut content = document();
        element(&mut content, "reception")["properties"]["signal"] = signal;
        assert!(reception_issues(&load(&content)).iter().any(|issue|
            issue.severity == StructuralSeverity::Error));
    }
}

#[test]
fn known_bound_mismatch_is_not_hidden_by_an_opaque_opposite_bound() {
    for upper_is_opaque in [false, true] {
        let mut content = document();
        let parameter = element(&mut content, "parameter");
        parameter["properties"].as_object_mut().expect("properties").remove("multiplicity");
        parameter["properties"]["lowerValue"] = json!({"$ref":"lower"});
        parameter["properties"]["upperValue"] = json!({"$ref":"upper"});
        content["elements"].as_array_mut().expect("elements").extend([
            if upper_is_opaque {
                json!({"id":"lower", "kind":"uml.literal_integer", "name":"",
                    "properties":{"value":1, "visibility":null}})
            } else {
                json!({"id":"lower", "kind":"uml.opaque_expression", "name":"",
                    "properties":{"visibility":null}})
            },
            if upper_is_opaque {
                json!({"id":"upper", "kind":"uml.opaque_expression", "name":"",
                    "properties":{"visibility":null}})
            } else {
                json!({"id":"upper", "kind":"uml.literal_unlimited_natural", "name":"",
                    "properties":{"value":1, "visibility":null}})
            },
        ]);
        content["relations"].as_array_mut().expect("relations").extend([
            json!({"id":"owns-lower", "kind":"owns", "from":"parameter", "to":"lower"}),
            json!({"id":"owns-upper", "kind":"owns", "from":"parameter", "to":"upper"}),
        ]);
        let issues = reception_issues(&load(&content));
        assert!(issues.iter().any(|issue| issue.severity == StructuralSeverity::Error
            && issue.message.contains("same_structure_as_signal")), "{issues:?}");
    }
    let mut content = document();
    element(&mut content, "parameter")["properties"]["multiplicity"] = json!("1..unknown");
    let issues = reception_issues(&load(&content));
    assert!(issues.iter().any(|issue| issue.severity == StructuralSeverity::Error
        && issue.message.contains("same_structure_as_signal")), "{issues:?}");
}
