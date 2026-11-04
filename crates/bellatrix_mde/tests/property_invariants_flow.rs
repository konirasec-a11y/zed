use bellatrix_mde::{
    Format, ModelingSection, Project, StructuralSeverity,
    commands::{Command, apply},
    decode, encode, inspect_element,
    uml::{associations, references::ReferenceIndex, root::Model},
};
use serde_json::{Value, json};

const READ_ONLY_RULE: &str = "Property-derived_union_is_read_only";
const DERIVED_RULE: &str = "Property-derived_union_is_derived";
const QUALIFIER_RULE: &str = "Property-qualified_is_association_end";

fn document() -> Project {
    serde_json::from_value(json!({
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
    .expect("canonical document")
}

fn has_error(project: &Project, subject: &str, rule: &str) -> bool {
    Model::from_project(project).validate().iter().any(|issue| {
        issue.severity == StructuralSeverity::Error
            && issue.subject_id == subject
            && issue.message.contains(rule)
    })
}

fn author(project: &mut Project, subject: &str, property: &str, value: Value) {
    project
        .elements
        .iter_mut()
        .find(|element| element.id == subject)
        .expect("authored Element")
        .properties
        .insert(property.into(), value);
}

fn edit(project: &Project, subject: &str, property: &str, value: Value) -> Result<Project, String> {
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

fn add_qualifier(project: &mut Project, owner: &str, explicit: bool) {
    project.elements.push(
        serde_json::from_value(json!({
            "id":"qualifier","kind":"uml.attribute","name":"key",
            "properties":{"type":{"$ref":"a"},"visibility":null}
        }))
        .expect("qualifier Property"),
    );
    project.relations.push(
        serde_json::from_value(json!({
            "id":"owns-qualifier","kind":"owns","from":owner,"to":"qualifier"
        }))
        .expect("canonical qualifier ownership"),
    );
    if explicit {
        author(project, owner, "qualifier", json!([{"$ref":"qualifier"}]));
    }
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
    let before = encode(&project, Format::Json).expect("snapshot before rejected edit");
    assert!(
        edit(&project, "attribute", "isDerivedUnion", json!(true))
            .expect_err("derived union requires both flags")
            .contains(READ_ONLY_RULE)
    );
    assert_eq!(
        encode(&project, Format::Json).expect("unchanged document"),
        before
    );
    assert_eq!(
        project
            .element("attribute")
            .expect("same Property")
            .properties
            .get("isDerivedUnion"),
        Some(&json!(false))
    );

    for property in ["isReadOnly", "isDerived", "isDerivedUnion"] {
        project = edit(&project, "attribute", property, json!(true)).expect("author valid flags");
    }
    let inspector = inspect_element(&ModelingSection::from_model(&project), "attribute")
        .expect("inspect the same Property");
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
        let before = encode(&project, Format::Json).expect("snapshot before inconsistent edit");
        assert!(
            edit(&project, "attribute", property, json!(false))
                .expect_err("inconsistent union flag")
                .contains(rule)
        );
        assert_eq!(
            encode(&project, Format::Json).expect("atomic rejection"),
            before
        );
    }
    for format in [Format::Json, Format::Protobuf] {
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
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "attribute")
            .expect("Property or subclass")
            .kind = kind.into();
        author(&mut project, "attribute", "isDerivedUnion", json!(true));
        assert!(has_error(&project, "attribute", READ_ONLY_RULE));
        assert!(has_error(&project, "attribute", DERIVED_RULE));
        for format in [Format::Json, Format::Protobuf] {
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
                assert!(Model::from_project(&draft).validate().iter().any(
                    |issue| issue.subject_id == "attribute"
                        && issue.code == "UML_PROPERTY_VALUE"
                        && issue.message.contains(property)
                ));
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
        let references = ReferenceIndex::new(Model::from_project(&valid));
        assert_eq!(
            associations::association_of(&references, "left")
                .expect("canonical inverse")
                .expect("Association")
                .id(),
            "association"
        );
        assert!(
            !valid
                .element("left")
                .expect("member Property")
                .properties
                .contains_key("association")
        );

        let mut invalid = document();
        add_qualifier(&mut invalid, "attribute", explicit);
        assert!(has_error(&invalid, "attribute", QUALIFIER_RULE));
        for format in [Format::Json, Format::Protobuf] {
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
            Model::from_project(&forged)
                .validate()
                .iter()
                .any(|issue| issue.subject_id == "attribute"
                    && issue.code == "UML_PROPERTY_ASSOCIATION_STORAGE")
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
            assert_eq!(repaired.element("qualifier"), invalid.element("qualifier"));
        }
    }
    let mut empty = document();
    author(&mut empty, "attribute", "qualifier", json!([]));
    assert!(!has_error(&empty, "attribute", QUALIFIER_RULE));
}

#[test]
fn classifier_feature_rules_reject_edits_preserve_drafts_and_reopen() {
    let mut project: Project = serde_json::from_value(json!({
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
    })).expect("authored classifier document");
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
        let snapshot = encode(&project, Format::Json).expect("before edit");
        let error =
            edit(&project, subject, property, value.clone()).expect_err("invalid semantic edit");
        assert!(error.contains(rule), "{error}");
        assert_eq!(
            encode(&project, Format::Json).expect("after rejection"),
            snapshot
        );
        let mut draft = project.clone();
        author(&mut draft, subject, property, value);
        let diagnostic_subject = if rule == "Interface-visibility" {
            "interface"
        } else {
            subject
        };
        assert!(has_error(&draft, diagnostic_subject, rule));
        for format in [Format::Json, Format::Protobuf] {
            let reopened =
                decode(&encode(&draft, format).expect("save draft"), format).expect("reopen draft");
            assert_eq!(reopened, draft);
            assert!(has_error(&reopened, diagnostic_subject, rule));
        }
    }
    // The default false value has the same semantics as an explicit false.
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "class")
        .expect("class")
        .properties
        .remove("isActive");
    assert!(has_error(&project, "class", "Class-passive_class"));
    project = edit(&project, "class", "isActive", json!(true)).expect("repair passive draft");
    assert!(!has_error(&project, "class", "Class-passive_class"));

    let mut behavior_only = project.clone();
    behavior_only
        .relations
        .retain(|relation| relation.to != "reception");
    behavior_only
        .elements
        .retain(|element| element.id != "reception");
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
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "behavior")
        .expect("behavior")
        .properties
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
        let before = encode(&mixed, Format::Json).expect("mixed draft snapshot");
        let error = edit(&mixed, "operation", "isAbstract", json!(true))
            .expect_err("known method proves the abstract constraint violation");
        assert!(
            error.contains("BehavioralFeature-abstract_no_method"),
            "{error}"
        );
        assert_eq!(
            encode(&mixed, Format::Json).expect("unchanged mixed draft"),
            before
        );
        author(&mut mixed, "operation", "isAbstract", json!(true));
        for format in [Format::Json, Format::Protobuf] {
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
            draft
                .relations
                .retain(|relation| relation.to != "reception");
            draft.elements.retain(|element| element.id != "reception");
            author(&mut draft, "class", "isActive", json!(false));
        }
        if subject == "interface" {
            draft
                .relations
                .retain(|relation| relation.to != "operation");
            draft.elements.retain(|element| element.id != "operation");
        }
        author(&mut draft, subject, property, value);
        let issues = Model::from_project(&draft).validate();
        assert!(
            issues.iter().any(|issue| issue.subject_id == subject
                && issue.severity == StructuralSeverity::Warning
                && issue.message.contains(rule)),
            "{issues:?}"
        );
        assert!(!has_error(&draft, subject, rule));
    }
}
