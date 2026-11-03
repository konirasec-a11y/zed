use bellatrix_mde::{ModelingSection, Project, set_inspected_property};
use serde_json::{Value, json};

fn document() -> ModelingSection {
    let project: Project = serde_json::from_value(json!({
        "id":"optional-reference", "name":"Optional reference", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Owner", "properties":{"isActive":true}},
            {"id":"operation", "kind":"uml.operation", "name":"accept",
             "properties":{"bodyCondition":"unfinished condition", "raisedException":"unfinished list"}},
            {"id":"reception", "kind":"uml.reception", "name":"Receive",
             "properties":{"signal":"unfinished reference"}},
            {"id":"readonly", "kind":"uml.operation", "name":"readOnly",
             "properties":{"ownedParameter":[]}}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"},
            {"id":"owns-reception", "kind":"owns", "from":"class", "to":"reception"},
            {"id":"owns-readonly", "kind":"owns", "from":"class", "to":"readonly"}
        ]
    })).expect("authored drafts");
    ModelingSection::from_model(&project)
}

#[test]
fn optional_reference_can_be_cleared_without_unlocking_other_fields() {
    let mut section = document();
    set_inspected_property(&mut section, "operation", "properties.bodyCondition", Value::Null)
        .expect("clear an optional scalar reference represented by an editable legacy string");
    assert_eq!(section.elements.iter().find(|element| element.id == "operation")
        .expect("operation").properties["bodyCondition"], Value::Null);
    for (owner, property, expected) in [
        ("reception", "properties.signal", "expected string"),
        ("operation", "properties.raisedException", "expected string"),
        ("readonly", "properties.ownedParameter", "read-only"),
        ("operation", "properties.type", "read-only"),
    ] {
        let before = section.clone();
        let error = set_inspected_property(&mut section, owner, property, Value::Null)
            .expect_err("required, collection and read-only fields stay protected");
        assert!(error.contains(expected), "{owner}::{property}: {error}");
        assert_eq!(section, before);
    }
}
